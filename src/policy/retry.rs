//! Controller-owned diagnostic retry sessions. No execution or approval authority.
use super::Decision;
use crate::{
    coverage,
    plan::FrozenPlan,
    report::{
        AttemptRecord, CaseStatus,
        normalize::canonical_digest,
        store::{Access, AppendInput, EvidenceStore, Provenance, Receipt},
    },
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryClass {
    StablePass,
    StableFailure,
    Flaky,
    Incomplete,
}
#[derive(Debug, Serialize)]
pub struct CaseHistory {
    pub test_id: String,
    pub environment: String,
    pub classification: RetryClass,
}
#[derive(Debug, Serialize)]
pub struct RetryAssessment {
    pub decision: Decision,
    pub cases: Vec<CaseHistory>,
    pub attempts: usize,
}
struct Round {
    id: String,
    receipt: Receipt,
}
/// Keep this session under a controller-owned request identity. Recreating it is
/// not a supported way to replace required history. Durable scheduler CAS is separate.
pub struct RetrySession<'a> {
    store: &'a EvidenceStore,
    access: Access,
    plan: FrozenPlan,
    plan_digest: String,
    provenance_digest: String,
    max_attempts: usize,
    rounds: Vec<Round>,
    poisoned: bool,
}
fn execution_digest(p: &Provenance) -> Result<String, String> {
    let fixed = [
        p.execution_source.as_str(),
        p.tool.as_str(),
        p.tool_version.as_str(),
        p.analyzer.as_str(),
        p.analyzer_version.as_str(),
        p.config_digest.as_str(),
    ];
    if p.command.is_empty()
        || p.command.len() > 128
        || p.environment.len() > 128
        || p.approval_refs.len() > 128
    {
        return Err("retry provenance count budget".into());
    }
    let mut bytes = 0usize;
    for s in fixed
        .into_iter()
        .chain(p.command.iter().map(String::as_str))
        .chain(
            p.environment
                .iter()
                .flat_map(|(k, v)| [k.as_str(), v.as_str()]),
        )
        .chain(p.approval_refs.iter().map(String::as_str))
    {
        if s.len() > 4096 {
            return Err("retry provenance string budget".into());
        }
        bytes = bytes.checked_add(s.len()).ok_or("retry provenance size")?;
        if bytes > 32 * 1024 {
            return Err("retry provenance byte budget".into());
        }
    }
    // Exclude only parent: every other execution input, including original secret
    // bytes, is compared privately. No secret values are returned or logged.
    canonical_digest(&(fixed, &p.command, &p.environment, &p.approval_refs))
}
impl<'a> RetrySession<'a> {
    pub fn new(
        store: &'a EvidenceStore,
        access: Access,
        plan: &FrozenPlan,
        provenance: &Provenance,
        max_attempts: usize,
    ) -> Result<Self, String> {
        if !(1..=8).contains(&max_attempts) || provenance.parent_attempt.is_some() {
            return Err("invalid retry session limits/parent".into());
        }
        coverage::preflight(plan, None, &[], &[])?;
        plan.validate()?;
        Ok(Self {
            store,
            access,
            plan: plan.clone(),
            plan_digest: canonical_digest(plan)?,
            provenance_digest: execution_digest(provenance)?,
            max_attempts,
            rounds: Vec::new(),
            poisoned: false,
        })
    }
    pub fn receipts(&self) -> impl Iterator<Item = &Receipt> {
        self.rounds.iter().map(|r| &r.receipt)
    }
    pub fn record(&mut self, input: AppendInput<'_>, now: u64) -> Result<(), String> {
        if self.poisoned {
            return Err("retry session requires controller recovery".into());
        }
        // An unrecorded or uncertain attempt must never be hidden behind prior green.
        self.poisoned = true;
        if self.rounds.len() >= self.max_attempts {
            return Err("diagnostic retry budget exhausted".into());
        }
        if input.attempt.attempt_id.len() > 128
            || self.rounds.iter().any(|r| r.id == input.attempt.attempt_id)
        {
            return Err("retry attempt identity reused/oversized".into());
        }
        if input.provenance.parent_attempt.as_deref() != self.rounds.last().map(|r| r.id.as_str()) {
            return Err("retry parent mismatch".into());
        }
        coverage::preflight(input.plan, Some(input.attempt), &[], &[])?;
        if canonical_digest(input.plan)? != self.plan_digest
            || input.attempt.plan_digest != self.plan_digest
            || execution_digest(input.provenance)? != self.provenance_digest
        {
            return Err("retry frozen plan/execution mismatch".into());
        }
        let id = input.attempt.attempt_id.clone();
        let receipt = self.store.append(&self.access, input, now)?;
        self.rounds.push(Round { id, receipt });
        self.poisoned = false;
        Ok(())
    }
    pub fn assess(&self, now: u64) -> Result<RetryAssessment, String> {
        if self.poisoned {
            return Err("retry session requires controller recovery".into());
        }
        let mut attempts: Vec<AttemptRecord> = Vec::new();
        let mut satisfied = !self.rounds.is_empty();
        for round in &self.rounds {
            let current = self.store.consume(&self.access, &round.receipt, now)?;
            if current.attempt.attempt_id != round.id
                || current.attempt.plan_digest != self.plan_digest
            {
                return Err("retry history binding conflict".into());
            }
            satisfied &=
                coverage::assess(&self.plan, &current.attempt, &[])?.decision == Decision::Allow;
            attempts.push(current.attempt);
        }
        let required: std::collections::BTreeSet<_> = self
            .plan
            .instances()
            .iter()
            .map(|i| (&i.test_id, &i.environment))
            .collect();
        let mut cases = Vec::new();
        for (test_id, environment) in required {
            let mut passed = false;
            let mut failed = false;
            let mut complete = !attempts.is_empty();
            for a in &attempts {
                match a
                    .observations
                    .iter()
                    .find(|o| &o.test_id == test_id && &o.environment == environment)
                {
                    Some(o) if o.status == CaseStatus::Fail => {
                        failed = true;
                    }
                    Some(o)
                        if o.status == CaseStatus::Pass
                            && o.started
                            && o.finished
                            && a.finished =>
                    {
                        passed = true;
                    }
                    _ => {
                        complete = false;
                    }
                }
            }
            let classification = if failed && passed {
                RetryClass::Flaky
            } else if failed {
                RetryClass::StableFailure
            } else if passed && complete {
                RetryClass::StablePass
            } else {
                RetryClass::Incomplete
            };
            satisfied &= classification == RetryClass::StablePass;
            cases.push(CaseHistory {
                test_id: test_id.clone(),
                environment: environment.clone(),
                classification,
            });
        }
        Ok(RetryAssessment {
            decision: if satisfied {
                Decision::Allow
            } else {
                Decision::Block
            },
            cases,
            attempts: attempts.len(),
        })
    }
}
