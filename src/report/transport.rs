use super::{
    engine_adapter::{ANALYZER_ID, CAPABILITY, MAPPING_VERSION, required_scopes},
    envelope::FixtureBundle,
};
use crate::plan::FrozenPlan;
use guardengine::integration::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureInvocation {
    pub capability: String,
    pub run_id: String,
    pub binding: RunBinding,
    pub started_at: String,
    pub finished_at: String,
}
pub enum FailureKind {
    Parser,
    Runtime,
    Cancelled,
}
pub struct BoundFixtureAttempt {
    pub(crate) bound: BoundAttempt,
    pub(crate) plan: FrozenPlan,
    pub(crate) invocation: FixtureInvocation,
    pub(crate) initial_coverage: Coverage,
}
pub(crate) fn diagnostic(code: &str, message: &str) -> TransportDiagnostic {
    TransportDiagnostic {
        code: code.into(),
        message: message.into(),
    }
}
pub(crate) fn producer() -> Producer {
    Producer {
        guard: "TestGuard".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        analyzer_id: ANALYZER_ID.into(),
        analyzer_version: MAPPING_VERSION.into(),
    }
}
pub fn prepare(
    plan: &FrozenPlan,
    invocation: FixtureInvocation,
) -> Result<BoundFixtureAttempt, TransportDiagnostic> {
    let invalid = |s: &str| diagnostic("invocation.invalid", s);
    crate::coverage::preflight(plan, None, &[], &[]).map_err(|e| invalid(&e))?;
    plan.validate().map_err(|e| invalid(&e))?;
    if invocation.capability != CAPABILITY {
        return Err(invalid("unsupported fixture capability"));
    }
    let binding = &invocation.binding;
    let local = plan.binding();
    let mut requirements = plan.obligations().requirements.clone();
    requirements.sort();
    if binding.repo_id != local.repository
        || binding.candidate_oid != local.candidate
        || binding.base_oid != local.base
        || binding.source_snapshot_digest != format!("sha256:{}", local.source_digest)
        || binding.baseline_digest != Some(format!("sha256:{}", plan.obligations().baseline_digest))
        || binding.requirement_ids != requirements
    {
        return Err(invalid("invocation differs from frozen domain binding"));
    }
    let required = required_scopes(plan).map_err(|e| invalid(&e))?;
    let coverage = Coverage {
        status: CoverageStatus::Partial,
        required_scopes: required.clone(),
        observed_scopes: vec![],
        missing_scopes: required,
    };
    // Validate every supplied context field before binding; this internal error template is never emitted.
    let template = GuardRunEnvelope {
        api_version: INTEGRATION_VERSION.into(),
        kind: "GuardRunEnvelope".into(),
        run_id: invocation.run_id.clone(),
        producer: producer(),
        binding: binding.clone(),
        run_status: RunStatus::Error,
        decision: None,
        coverage: coverage.clone(),
        artifacts: Artifacts {
            contract: None,
            facts: None,
            report: None,
            domain: vec![],
        },
        approval_refs: vec![],
        diagnostics: vec![Diagnostic {
            code: "fixture.context".into(),
            message: "context validation".into(),
            retryable: false,
            source: None,
        }],
        started_at: invocation.started_at.clone(),
        finished_at: invocation.finished_at.clone(),
        expires_at: None,
    };
    template
        .validate(EvidenceProfile::EngineBacked)
        .map_err(|e| invalid(&e.to_string()))?;
    let bound = prepare_attempt(InvocationDraft {
        run_id: invocation.run_id.clone(),
        producer: Some(producer()),
        binding: Some(binding.clone()),
        coverage: Some(coverage.clone()),
        profile: Some(EvidenceProfile::EngineBacked),
        started_at: invocation.started_at.clone(),
    })?;
    Ok(BoundFixtureAttempt {
        bound,
        plan: plan.clone(),
        invocation,
        initial_coverage: coverage,
    })
}
impl BoundFixtureAttempt {
    pub fn fail(
        self,
        kind: FailureKind,
        input: &[u8],
    ) -> Result<FixtureBundle, TransportDiagnostic> {
        let (status, code) = match kind {
            FailureKind::Parser => (RunStatus::Error, "parser.failed"),
            FailureKind::Runtime => (RunStatus::Error, "runtime.failed"),
            FailureKind::Cancelled => (RunStatus::Cancelled, "runtime.cancelled"),
        };
        let coverage = serde_json::from_slice::<crate::report::AttemptRecord>(input)
            .ok()
            .filter(|a| a.attempt_id == self.invocation.run_id)
            .and_then(|a| super::engine_adapter::observed_coverage(&self.plan, &a).ok())
            .unwrap_or(self.initial_coverage);
        let mut artifacts = std::collections::BTreeMap::new();
        let input_ref = super::envelope::store(
            &mut artifacts,
            &self.invocation.run_id,
            "failed-input.bin",
            input.to_vec(),
            "application/octet-stream",
        );
        let plan_bytes = serde_json::to_vec(&self.plan)
            .map_err(|_| diagnostic("serialization.failed", "cannot preserve frozen plan"))?;
        let plan_ref = super::envelope::store(
            &mut artifacts,
            &self.invocation.run_id,
            "plan.json",
            plan_bytes,
            "application/json",
        );
        let mut refs = vec![input_ref, plan_ref];
        refs.sort_by(|a, b| a.uri.cmp(&b.uri));
        let envelope = self.bound.finish(AttemptOutput {
            coverage,
            run_status: status,
            decision: None,
            artifacts: Artifacts {
                contract: None,
                facts: None,
                report: None,
                domain: refs,
            },
            approval_refs: vec![],
            diagnostics: vec![Diagnostic {
                code: code.into(),
                message: "local fixture attempt did not complete; original input retained".into(),
                retryable: false,
                source: None,
            }],
            finished_at: self.invocation.finished_at,
            expires_at: None,
        })?;
        Ok(FixtureBundle {
            capability: CAPABILITY.into(),
            envelope,
            artifacts,
        })
    }
}
impl BoundFixtureAttempt {
    pub fn complete(
        self,
        attempt: &crate::report::AttemptRecord,
        changes: &[crate::policy::Weakening],
        advice: &[String],
    ) -> Result<FixtureBundle, TransportDiagnostic> {
        let raw_attempt = serde_json::to_vec(attempt)
            .map_err(|_| diagnostic("serialization.failed", "cannot preserve attempt"))?;
        if attempt.attempt_id != self.invocation.run_id || raw_attempt.len() > MAX_ARTIFACT_BYTES {
            return self.fail(FailureKind::Runtime, &raw_attempt);
        }
        let projection = match super::engine_adapter::project(
            &self.plan, attempt, changes, advice, CAPABILITY,
        ) {
            Ok(p) => p,
            Err(_) => return self.fail(FailureKind::Runtime, &raw_attempt),
        };
        let mut artifacts = std::collections::BTreeMap::new();
        let run = &self.invocation.run_id;
        let contract = super::envelope::store(
            &mut artifacts,
            run,
            "contract.json",
            projection.contract_bytes,
            "application/json",
        );
        let facts = super::envelope::store(
            &mut artifacts,
            run,
            "facts.json",
            projection.facts_bytes,
            "application/json",
        );
        let report = super::envelope::store(
            &mut artifacts,
            run,
            "report.json",
            projection.report_bytes,
            "application/json",
        );
        let domain = super::envelope::store(
            &mut artifacts,
            run,
            "domain.json",
            projection.domain_bytes,
            "application/json",
        );
        let inputs = super::envelope::ProjectionInput {
            attempt: attempt.clone(),
            changes: changes.to_vec(),
            advice: advice.to_vec(),
        };
        let input_bytes = serde_json::to_vec(&inputs)
            .map_err(|_| diagnostic("serialization.failed", "cannot preserve inputs"))?;
        let input = super::envelope::store(
            &mut artifacts,
            run,
            "inputs.json",
            input_bytes,
            "application/json",
        );
        let plan_bytes = serde_json::to_vec(&self.plan)
            .map_err(|_| diagnostic("serialization.failed", "cannot preserve plan"))?;
        let plan = super::envelope::store(
            &mut artifacts,
            run,
            "plan.json",
            plan_bytes,
            "application/json",
        );
        if artifacts
            .values()
            .any(|bytes| bytes.len() > MAX_ARTIFACT_BYTES)
        {
            return self.fail(FailureKind::Runtime, &raw_attempt);
        }
        // Exercise GE's exact byte/recomputation budget before issuing a completed outcome.
        let preview = GuardRunEnvelope {
            api_version: INTEGRATION_VERSION.into(),
            kind: "GuardRunEnvelope".into(),
            run_id: self.invocation.run_id.clone(),
            producer: producer(),
            binding: self.invocation.binding.clone(),
            run_status: RunStatus::Completed,
            decision: Some(projection.report.decision.clone()),
            coverage: projection.coverage.clone(),
            artifacts: Artifacts {
                contract: Some(contract.clone()),
                facts: Some(facts.clone()),
                report: Some(report.clone()),
                domain: vec![domain.clone(), input.clone(), plan.clone()],
            },
            approval_refs: vec![],
            diagnostics: vec![],
            started_at: self.invocation.started_at.clone(),
            finished_at: self.invocation.finished_at.clone(),
            expires_at: None,
        };
        if verify_engine_artifacts(
            &preview,
            &artifacts[&contract.uri],
            &artifacts[&facts.uri],
            &artifacts[&report.uri],
        )
        .is_err()
        {
            return self.fail(FailureKind::Runtime, &raw_attempt);
        }
        let envelope = self.bound.finish(AttemptOutput {
            coverage: projection.coverage,
            run_status: RunStatus::Completed,
            decision: Some(projection.report.decision),
            artifacts: Artifacts {
                contract: Some(contract),
                facts: Some(facts),
                report: Some(report),
                domain: vec![domain, input, plan],
            },
            approval_refs: vec![],
            diagnostics: vec![],
            finished_at: self.invocation.finished_at,
            expires_at: None,
        })?;
        Ok(FixtureBundle {
            capability: CAPABILITY.into(),
            envelope,
            artifacts,
        })
    }
}
