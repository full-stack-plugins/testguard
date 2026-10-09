//! Private receipts from a pinned trusted fixture; no arbitrary-program coverage or authorization.
use super::{Metric, instrumented::Event};
use crate::{
    plan::FrozenPlan,
    report::{
        AttemptRecord, CaseObservation, CaseStatus,
        normalize::{ArtifactRef, bytes_digest, canonical_digest},
    },
};
use serde::Serialize;
use std::collections::BTreeSet;
pub const PROFILE: &str = "testguard.advanced-instrumented-local/v1";
pub const ENVIRONMENT: &str = "local-in-process";
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricKind {
    Contract,
    State,
    Concurrency,
    Mutation,
}
#[derive(Clone, Copy)]
pub struct UnitSpec {
    pub kind: MetricKind,
    pub id: &'static str,
    pub source: &'static str,
    pub test: &'static str,
    pub anchor: &'static str,
}
pub fn units() -> Vec<UnitSpec> {
    super::contract::UNITS
        .into_iter()
        .chain(super::state::UNITS)
        .chain(super::concurrency::UNITS)
        .chain(super::mutation::UNITS)
        .collect()
}
pub fn source_digest() -> String {
    bytes_digest(include_bytes!("instrumented.rs"))
}
fn instrumentation_digest() -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for bytes in [
        PROFILE.as_bytes(),
        include_bytes!("advanced.rs"),
        include_bytes!("contract.rs"),
        include_bytes!("state.rs"),
        include_bytes!("concurrency.rs"),
        include_bytes!("mutation.rs"),
    ] {
        h.update((bytes.len() as u64).to_be_bytes());
        h.update(bytes)
    }
    format!("{:x}", h.finalize())
}
fn admit(value: &impl Serialize, weight: usize) -> Result<(), String> {
    struct Counter {
        total: usize,
        weight: usize,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.total = self
                .total
                .saturating_add(b.len().saturating_mul(self.weight));
            if self.total > 4 * 1024 * 1024 {
                return Err(std::io::Error::other("advanced coverage budget"));
            }
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Counter { total: 0, weight }, value)
        .map_err(|_| "advanced coverage budget".into())
}
#[derive(Clone, Serialize)]
pub struct UnitSelection {
    pub id: String,
    pub excluded_reason: Option<String>,
}
#[derive(Clone, Serialize)]
pub struct MetricScope {
    pub kind: MetricKind,
    pub scope: String,
    pub filter: String,
    pub units: Vec<UnitSelection>,
}
pub struct AdvancedPlan {
    plan: FrozenPlan,
    scopes: Vec<MetricScope>,
    digest: String,
    source: String,
    instrumentation: String,
}
impl AdvancedPlan {
    pub fn freeze(
        plan: &FrozenPlan,
        profile: &str,
        mut scopes: Vec<MetricScope>,
    ) -> Result<Self, String> {
        if profile != PROFILE || scopes.len() != 4 || scopes.iter().any(|s| s.units.len() > 32) {
            return Err("unsupported advanced instrumentation profile/count".into());
        }
        admit(&(plan, &scopes), 8)?;
        super::preflight(plan, None, &[], &[])?;
        plan.validate()?;
        let source = source_digest();
        if plan.binding().source_digest != source {
            return Err("advanced source does not match actual fixture source".into());
        }
        let mut kinds = BTreeSet::new();
        for s in &mut scopes {
            if !kinds.insert(s.kind)
                || s.scope.is_empty()
                || s.scope.len() > 256
                || s.filter.is_empty()
                || s.filter.len() > 256
            {
                return Err("duplicate metric or invalid scope/filter".into());
            }
            let mut ids = BTreeSet::new();
            for u in &s.units {
                if u.id.is_empty()
                    || u.id.len() > 256
                    || !ids.insert(&u.id)
                    || u.excluded_reason
                        .as_ref()
                        .is_some_and(|r| r.trim().is_empty() || r.len() > 256)
                {
                    return Err("invalid unit or exclusion reason".into());
                }
            }
            if units()
                .iter()
                .any(|u| u.kind == s.kind && !s.units.iter().any(|v| v.id == u.id))
            {
                return Err(
                    "every known metric unit needs inclusion or an explicit exclusion reason"
                        .into(),
                );
            }
            s.units.sort_by(|a, b| a.id.cmp(&b.id));
        }
        if plan.instances().len() > 1024 || plan.obligations().sources.len() > 1024 {
            return Err("advanced trace expansion budget".into());
        }
        scopes.sort_by_key(|s| s.kind);
        let instrumentation = instrumentation_digest();
        let digest = canonical_digest(&(PROFILE, plan, &scopes, &source, &instrumentation))?;
        Ok(Self {
            plan: plan.clone(),
            scopes,
            digest,
            source,
            instrumentation,
        })
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn frozen_plan(&self) -> &FrozenPlan {
        &self.plan
    }
}
/// Only the fixed instrumented execution below creates this receipt.
/// ```compile_fail
/// use testguard::coverage::advanced::FixtureReceipt;
/// let forged = FixtureReceipt { attempt: todo!(), artifact: vec![], events: vec![], plan: String::new() };
/// ```
pub struct FixtureReceipt {
    plan: String,
    attempt: AttemptRecord,
    artifact: Vec<u8>,
    events: Vec<Event>,
}
impl FixtureReceipt {
    pub fn attempt(&self) -> &AttemptRecord {
        &self.attempt
    }
    pub fn artifact(&self) -> &[u8] {
        &self.artifact
    }
}
/// Explicit execution of eight trusted, repository-owned scenarios only. No user code, shell or sandbox.
pub fn run_fixture(
    plan: &AdvancedPlan,
    attempt_id: &str,
    tests: &BTreeSet<String>,
) -> Result<FixtureReceipt, String> {
    if attempt_id.len() > 128 || tests.len() > 8 {
        return Err("fixture execution budget".into());
    }
    admit(&tests, 8)?;
    ArtifactRef::from_bytes(attempt_id, "instrumentation", b"")?;
    let registry = units();
    if tests.iter().any(|t| !registry.iter().any(|u| u.test == t)) {
        return Err("unknown instrumented test".into());
    }
    let mut events = Vec::new();
    for unit in &registry {
        if tests.contains(unit.test) {
            events.push(super::instrumented::execute(unit));
        }
    }
    let plan_digest = canonical_digest(&plan.plan)?;
    #[derive(Serialize)]
    struct Artifact<'a> {
        profile: &'static str,
        source_digest: &'a str,
        instrumentation_digest: &'a str,
        plan_digest: &'a str,
        advanced_plan_digest: &'a str,
        attempt_id: &'a str,
        events: &'a [Event],
    }
    let data = Artifact {
        profile: PROFILE,
        source_digest: &plan.source,
        instrumentation_digest: &plan.instrumentation,
        plan_digest: &plan_digest,
        advanced_plan_digest: &plan.digest,
        attempt_id,
        events: &events,
    };
    admit(&data, 4)?;
    let artifact = serde_json::to_vec(&data).map_err(|_| "fixture artifact encoding")?;
    let reference = ArtifactRef::from_bytes(attempt_id, "instrumentation", &artifact)?;
    let attempt = AttemptRecord {
        schema_version: crate::obligation::VERSION.into(),
        attempt_id: attempt_id.into(),
        plan_digest,
        exit_code: Some(if events.iter().all(Event::covered) {
            0
        } else {
            1
        }),
        finished: true,
        observations: events
            .iter()
            .map(|e| CaseObservation {
                test_id: e.test.clone(),
                native_id: e.test.clone(),
                parameters: String::new(),
                target: PROFILE.into(),
                features: vec![],
                environment: ENVIRONMENT.into(),
                status: if e.covered() {
                    CaseStatus::Pass
                } else {
                    CaseStatus::Fail
                },
                discovered: true,
                started: true,
                finished: true,
                artifact_uri: reference.uri.clone(),
            })
            .collect(),
        artifacts: vec![reference],
    };
    attempt.validate()?;
    Ok(FixtureReceipt {
        plan: plan.digest.clone(),
        attempt,
        artifact,
        events,
    })
}
#[derive(Serialize)]
pub struct AdvancedTrace {
    pub unit: String,
    pub source: String,
    pub source_anchor: String,
    pub requirement: String,
    pub obligation: String,
    pub test: String,
    pub environment: String,
    pub attempt: String,
    pub artifact_uri: Option<String>,
    pub artifact_digest: Option<String>,
    pub observed: bool,
    pub satisfied: bool,
}
#[derive(Serialize)]
pub struct UnitCoverage {
    pub id: String,
    pub covered: bool,
    pub excluded_reason: Option<String>,
    pub unmapped_reason: Option<String>,
    pub uncovered_reason: Option<String>,
}
#[derive(Serialize)]
pub struct AdvancedMetric {
    pub kind: MetricKind,
    pub scope: String,
    pub filter: String,
    pub counts: Metric,
    pub units: Vec<UnitCoverage>,
    pub extra: Vec<String>,
    pub traces: Vec<AdvancedTrace>,
    pub not_applicable_reason: Option<String>,
}
#[derive(Serialize)]
pub struct AdvancedCoverage {
    pub profile: &'static str,
    pub source_digest: String,
    pub instrumentation_digest: String,
    pub advanced_plan_digest: String,
    pub attempt_id: String,
    pub execution: super::DomainAssessment,
    pub metrics: Vec<AdvancedMetric>,
}
/// No numeric counts or raw instrumentation report can create accepted advanced evidence.
pub fn assess(plan: &AdvancedPlan, receipt: &FixtureReceipt) -> Result<AdvancedCoverage, String> {
    if receipt.plan != plan.digest {
        return Err("foreign advanced attempt/plan".into());
    }
    receipt.attempt.artifacts[0].verify(&receipt.artifact)?;
    let execution = super::assess(&plan.plan, &receipt.attempt, &[])?;
    let registry = units();
    let mut metric_reports = Vec::new();
    let mut output_charge = 0usize;
    let mut link_count = 0usize;
    for scope in &plan.scopes {
        let mut rows = Vec::new();
        let mut traces = Vec::new();
        let mut denominator = 0;
        let mut numerator = 0;
        for selected in &scope.units {
            if selected.excluded_reason.is_none() {
                denominator += 1;
            }
            let known = registry
                .iter()
                .find(|u| u.id == selected.id && u.kind == scope.kind);
            let mut unmapped = None;
            let mut covered = false;
            if let Some(unit) = known {
                let source = plan.plan.obligations().sources.iter().find(|s| {
                    s.id == unit.source
                        && matches!(s.kind, crate::obligation::SourceKind::Invariant)
                });
                let obligations = plan
                    .plan
                    .obligations()
                    .obligations
                    .iter()
                    .filter(|o| o.source_ids.iter().any(|s| s == unit.source))
                    .collect::<Vec<_>>();
                if let Some(source) = source.filter(|_| !obligations.is_empty()) {
                    covered = true;
                    for obligation in obligations {
                        for instance in plan
                            .plan
                            .instances()
                            .iter()
                            .filter(|i| i.obligation_id == obligation.id)
                        {
                            let event = receipt.events.iter().find(|e| {
                                e.unit == unit.id
                                    && e.test == instance.test_id
                                    && e.environment == instance.environment
                            });
                            let observed = instance.test_id == unit.test
                                && instance.environment == ENVIRONMENT
                                && event.is_some_and(Event::covered)
                                && receipt.attempt.exit_code == Some(0);
                            if instance.test_id != unit.test || instance.environment != ENVIRONMENT
                            {
                                unmapped = Some("unsupported test or environment mapping".into());
                            }
                            covered &= observed;
                            link_count += 1;
                            if link_count > 2048 {
                                return Err("advanced trace count budget".into());
                            }
                            let metadata = (
                                &selected.id,
                                unit.source,
                                unit.anchor,
                                &source.requirement_id,
                                &instance.obligation_id,
                                &instance.test_id,
                                &instance.environment,
                                &receipt.attempt.attempt_id,
                                &receipt.attempt.artifacts[0],
                            );
                            admit(&metadata, 8)?;
                            // Conservative expansion admission before each owned trace string is cloned.
                            output_charge = output_charge.saturating_add(
                                (selected.id.len()
                                    + unit.source.len()
                                    + unit.anchor.len()
                                    + source.requirement_id.len()
                                    + instance.obligation_id.len()
                                    + instance.test_id.len()
                                    + instance.environment.len()
                                    + receipt.attempt.attempt_id.len()
                                    + receipt.attempt.artifacts[0].uri.len()
                                    + 128)
                                    * 8,
                            );
                            if output_charge > 4 * 1024 * 1024 {
                                return Err("advanced trace byte budget".into());
                            }
                            traces.push(AdvancedTrace {
                                unit: selected.id.clone(),
                                source: unit.source.into(),
                                source_anchor: unit.anchor.into(),
                                requirement: source.requirement_id.clone(),
                                obligation: instance.obligation_id.clone(),
                                test: instance.test_id.clone(),
                                environment: instance.environment.clone(),
                                attempt: receipt.attempt.attempt_id.clone(),
                                artifact_uri: event
                                    .map(|_| receipt.attempt.artifacts[0].uri.clone()),
                                artifact_digest: event
                                    .map(|_| receipt.attempt.artifacts[0].digest.clone()),
                                observed: event.is_some(),
                                satisfied: observed,
                            });
                        }
                    }
                } else {
                    unmapped = Some("missing complete fixture source/obligation mapping".into());
                }
            } else {
                unmapped = Some("unknown instrumentation unit".into());
            }
            if selected.excluded_reason.is_some() || unmapped.is_some() {
                covered = false;
            }
            if covered {
                numerator += 1;
            }
            rows.push(UnitCoverage {
                id: selected.id.clone(),
                covered,
                excluded_reason: selected.excluded_reason.clone(),
                uncovered_reason: if !covered && selected.excluded_reason.is_none() && unmapped.is_none() { Some("required instrumentation assertion/variant or test/environment is missing or unsatisfied".into()) } else { None },
                unmapped_reason: unmapped,
            });
        }
        let extra = receipt
            .events
            .iter()
            .filter(|e| {
                registry
                    .iter()
                    .any(|u| u.kind == scope.kind && u.id == e.unit)
                    && scope.units.iter().find(|s| s.id == e.unit).is_none_or(|s| {
                        s.excluded_reason.is_some()
                            || rows
                                .iter()
                                .any(|r| r.id == e.unit && r.unmapped_reason.is_some())
                    })
            })
            .map(|e| e.unit.clone())
            .collect();
        metric_reports.push(AdvancedMetric {
            kind: scope.kind,
            scope: scope.scope.clone(),
            filter: scope.filter.clone(),
            counts: Metric {
                numerator,
                denominator,
            },
            units: rows,
            extra,
            traces,
            not_applicable_reason: if denominator == 0 {
                Some("N/A: frozen applicable unit denominator is empty".into())
            } else {
                None
            },
        });
    }
    Ok(AdvancedCoverage {
        profile: PROFILE,
        source_digest: plan.source.clone(),
        instrumentation_digest: plan.instrumentation.clone(),
        advanced_plan_digest: plan.digest.clone(),
        attempt_id: receipt.attempt.attempt_id.clone(),
        execution,
        metrics: metric_reports,
    })
}
