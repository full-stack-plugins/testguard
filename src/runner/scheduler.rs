//! Controller-owned in-memory scheduling of fixture producer work, not OS execution.
use crate::{
    plan::FrozenPlan,
    policy::Weakening,
    report::{
        AttemptRecord,
        envelope::FixtureBundle,
        transport::{self, BoundFixtureAttempt, FailureKind, FixtureInvocation},
    },
};
use guardengine::integration::RunBinding;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};
const INPUT_BYTES: usize = 256 * 1024;
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Limits {
    pub requests: usize,
    pub inflight: usize,
    pub history_bytes: usize,
    pub bundle_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            requests: 256,
            inflight: 16,
            history_bytes: 64 * 1024 * 1024,
            bundle_bytes: 4 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Pending,
    Completed,
    Abandoned,
}
#[derive(Clone, Debug)]
pub struct AttemptStatus {
    pub request_id: String,
    pub run_id: String,
    pub status: Status,
    pub exit_code: Option<i32>,
}
#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub attempt: AttemptStatus,
    pub binding: RunBinding,
    pub work_digest: String,
    pub raw_bundle: Arc<[u8]>,
}
pub enum Admission {
    Start(Lease),
    Existing(AttemptStatus),
}
#[derive(Debug, PartialEq, Eq)]
pub enum Publication {
    Current,
    Archived,
    Duplicate,
}
struct Row {
    scope: String,
    work: String,
    binding: RunBinding,
    status: AttemptStatus,
}
struct State {
    rows: BTreeMap<String, Row>,
    runs: BTreeSet<String>,
    current: BTreeMap<String, String>,
    history: Vec<HistoryEntry>,
    bytes: usize,
    inflight: usize,
}
struct Inner {
    state: Mutex<State>,
    limits: Limits,
}
#[derive(Clone)]
pub struct Scheduler {
    inner: Arc<Inner>,
}
/// Single-use execution admission. Dropping it abandons admission, never restores old success.
pub struct Lease {
    owner: Arc<Inner>,
    request: String,
    work: String,
    producer: Option<Box<BoundFixtureAttempt>>,
    completion_issued: bool,
    changes: Vec<Weakening>,
    advice: Vec<String>,
}
/// Completion cannot be built from external JSON or a caller-supplied digest.
/// ```compile_fail
/// let _: testguard::runner::scheduler::Completion = serde_json::from_str("{}").unwrap();
/// ```
pub struct Completion {
    owner: Arc<Inner>,
    request: String,
    work: String,
    bundle: FixtureBundle,
    raw: Arc<[u8]>,
}
fn identity(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 256 && !s.contains('\0')
}
pub(crate) fn measure(value: &impl Serialize, limit: usize) -> Result<usize, String> {
    struct Count {
        remaining: usize,
    }
    impl std::io::Write for Count {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            if b.len() > self.remaining {
                return Err(std::io::ErrorKind::InvalidInput.into());
            }
            self.remaining -= b.len();
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count { remaining: limit };
    serde_json::to_writer(&mut count, value).map_err(|_| "scheduler byte budget".to_string())?;
    Ok(limit - count.remaining)
}
fn hash(value: &impl Serialize) -> Result<String, String> {
    crate::report::normalize::canonical_digest(value)
}
fn binding_budget(b: &RunBinding) -> Result<(), String> {
    if b.requirement_ids.len() > 256 {
        return Err("scheduler requirement budget".into());
    }
    measure(b, 64 * 1024)?;
    Ok(())
}
fn scope(b: &RunBinding) -> Result<String, String> {
    hash(&(
        "testguard.scheduler-scope/v1alpha1",
        &b.repo_id,
        &b.task_id,
        &b.worktree_id,
        &b.requirement_ids,
    ))
}
fn input_budget(
    plan: &FrozenPlan,
    inv: &FixtureInvocation,
    changes: &[Weakening],
    advice: &[String],
) -> Result<(), String> {
    let o = plan.obligations();
    if !identity(&inv.run_id)
        || changes.len() > 64
        || advice.len() > 64
        || advice.iter().any(|s| s.len() > 4096)
        || plan.instances().len() > 1024
        || o.obligations.len() > 1024
        || o.sources.len() > 1024
        || o.requirements.len() > 256
        || o.environments.len() > 256
        || o.obligations
            .iter()
            .any(|o| o.environments.len() > 256 || o.source_ids.len() > 256)
    {
        return Err("scheduler input count/identity budget".into());
    }
    binding_budget(&inv.binding)?;
    measure(&(plan, inv, changes, advice), INPUT_BYTES)?;
    crate::coverage::preflight(plan, None, changes, advice)
}
impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
impl Scheduler {
    pub fn new() -> Self {
        Self::with_limits(Limits::default()).expect("fixed scheduler limits")
    }
    pub fn with_limits(limits: Limits) -> Result<Self, String> {
        let hard = Limits::default();
        if limits.requests == 0
            || limits.requests > hard.requests
            || limits.inflight == 0
            || limits.inflight > hard.inflight
            || limits.inflight > limits.requests
            || limits.history_bytes == 0
            || limits.history_bytes > hard.history_bytes
            || limits.bundle_bytes == 0
            || limits.bundle_bytes > hard.bundle_bytes
        {
            return Err("invalid scheduler limits".into());
        }
        Ok(Self {
            inner: Arc::new(Inner {
                limits,
                state: Mutex::new(State {
                    rows: BTreeMap::new(),
                    runs: BTreeSet::new(),
                    current: BTreeMap::new(),
                    history: vec![],
                    bytes: 0,
                    inflight: 0,
                }),
            }),
        })
    }
    pub fn admit(
        &self,
        id: &str,
        plan: &FrozenPlan,
        inv: FixtureInvocation,
        changes: &[Weakening],
        advice: &[String],
    ) -> Result<Admission, String> {
        if !identity(id) {
            return Err("invalid request identity".into());
        }
        input_budget(plan, &inv, changes, advice)?;
        let producer = transport::producer();
        let work = hash(&(
            "testguard.scheduler-work/v1alpha1",
            plan,
            &inv,
            changes,
            advice,
            &producer,
            self.inner.limits,
        ))?;
        let scope = scope(&inv.binding)?;
        let mut state = self.inner.state.lock().map_err(|_| "scheduler poisoned")?;
        if let Some(row) = state.rows.get(id) {
            return if row.work == work {
                Ok(Admission::Existing(row.status.clone()))
            } else {
                Err("request identity conflict".into())
            };
        }
        if state.rows.len() >= self.inner.limits.requests
            || state.inflight >= self.inner.limits.inflight
            || state.runs.contains(&inv.run_id)
        {
            return Err("scheduler capacity/run conflict".into());
        }
        let bound = transport::prepare(plan, inv.clone()).map_err(|e| e.message)?;
        let status = AttemptStatus {
            request_id: id.into(),
            run_id: inv.run_id.clone(),
            status: Status::Pending,
            exit_code: None,
        };
        state.runs.insert(inv.run_id);
        state.current.insert(scope.clone(), id.into());
        state.rows.insert(
            id.into(),
            Row {
                scope,
                work: work.clone(),
                binding: inv.binding,
                status,
            },
        );
        state.inflight += 1;
        Ok(Admission::Start(Lease {
            owner: self.inner.clone(),
            request: id.into(),
            work,
            producer: Some(Box::new(bound)),
            completion_issued: false,
            changes: changes.to_vec(),
            advice: advice.to_vec(),
        }))
    }
    pub fn publish(&self, done: &Completion) -> Result<Publication, String> {
        if !Arc::ptr_eq(&self.inner, &done.owner) {
            return Err("foreign scheduler completion".into());
        }
        let mut state = self.inner.state.lock().map_err(|_| "scheduler poisoned")?;
        let row = state.rows.get(&done.request).ok_or("unknown completion")?;
        if row.work != done.work
            || row.binding != done.bundle.envelope.binding
            || row.status.run_id != done.bundle.envelope.run_id
        {
            return Err("completion binding changed".into());
        }
        if row.status.status == Status::Completed {
            return Ok(Publication::Duplicate);
        }
        if row.status.status != Status::Pending {
            return Err("abandoned completion".into());
        }
        let bytes = state
            .bytes
            .checked_add(done.raw.len())
            .ok_or("history byte overflow")?;
        if bytes > self.inner.limits.history_bytes {
            return Err("history byte capacity".into());
        }
        let current = state.current.get(&row.scope) == Some(&done.request);
        let mut status = row.status.clone();
        status.status = Status::Completed;
        status.exit_code = Some(done.bundle.exit_code());
        let entry = HistoryEntry {
            attempt: status.clone(),
            binding: row.binding.clone(),
            work_digest: row.work.clone(),
            raw_bundle: done.raw.clone(),
        };
        state
            .rows
            .get_mut(&done.request)
            .expect("row exists")
            .status = status;
        state.inflight = state.inflight.saturating_sub(1);
        state.bytes = bytes;
        state.history.push(entry);
        Ok(if current {
            Publication::Current
        } else {
            Publication::Archived
        })
    }
    pub fn current(&self, b: &RunBinding) -> Result<Option<AttemptStatus>, String> {
        binding_budget(b)?;
        let key = scope(b)?;
        let state = self.inner.state.lock().map_err(|_| "scheduler poisoned")?;
        Ok(state
            .current
            .get(&key)
            .and_then(|id| state.rows.get(id))
            .filter(|r| r.binding == *b)
            .map(|r| r.status.clone()))
    }
    pub fn history(&self) -> Result<Vec<HistoryEntry>, String> {
        Ok(self
            .inner
            .state
            .lock()
            .map_err(|_| "scheduler poisoned")?
            .history
            .clone())
    }
}
impl Lease {
    pub fn complete(mut self, a: &AttemptRecord) -> Result<Completion, String> {
        if a.observations.len() > 4096
            || a.artifacts.len() > 256
            || a.observations.iter().any(|o| o.features.len() > 256)
        {
            return Err("scheduler attempt count budget".into());
        }
        measure(a, INPUT_BYTES)?;
        let producer = self.producer.as_ref().ok_or("spent lease")?;
        crate::coverage::preflight(&producer.plan, Some(a), &self.changes, &self.advice)?;
        let bundle = self
            .producer
            .take()
            .expect("lease owns producer")
            .complete(a, &self.changes, &self.advice)
            .map_err(|e| e.message)?;
        self.finish(bundle)
    }
    pub fn fail(mut self, kind: FailureKind, input: &[u8]) -> Result<Completion, String> {
        if input.len() > INPUT_BYTES {
            return Err("scheduler failure-input budget".into());
        }
        let bundle = self
            .producer
            .take()
            .ok_or("spent lease")?
            .fail(kind, input)
            .map_err(|e| e.message)?;
        self.finish(bundle)
    }
    fn finish(&mut self, bundle: FixtureBundle) -> Result<Completion, String> {
        measure(&bundle, self.owner.limits.bundle_bytes)?;
        let raw = serde_json::to_vec(&bundle).map_err(|_| "scheduler serialization")?;
        self.completion_issued = true;
        Ok(Completion {
            owner: self.owner.clone(),
            request: self.request.clone(),
            work: self.work.clone(),
            bundle,
            raw: raw.into(),
        })
    }
}
fn abandon(owner: &Inner, request: &str) {
    if let Ok(mut state) = owner.state.lock()
        && let Some(row) = state.rows.get_mut(request)
        && row.status.status == Status::Pending
    {
        row.status.status = Status::Abandoned;
        state.inflight = state.inflight.saturating_sub(1);
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if !self.completion_issued {
            abandon(&self.owner, &self.request);
        }
    }
}
impl Drop for Completion {
    fn drop(&mut self) {
        abandon(&self.owner, &self.request);
    }
}
impl Completion {
    pub(crate) fn verify_work(
        &self,
        plan: &FrozenPlan,
        inv: &FixtureInvocation,
        changes: &[Weakening],
        advice: &[String],
    ) -> Result<(), String> {
        input_budget(plan, inv, changes, advice)?;
        let work = hash(&(
            "testguard.scheduler-work/v1alpha1",
            plan,
            inv,
            changes,
            advice,
            transport::producer(),
            self.owner.limits,
        ))?;
        if work != self.work {
            return Err("completion differs from protected full work".into());
        }
        Ok(())
    }
    pub fn bundle(&self) -> &FixtureBundle {
        &self.bundle
    }
}
