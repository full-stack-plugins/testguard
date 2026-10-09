use crate::{
    plan::{FrozenPlan, Instance},
    policy::{Decision, Weakening},
    report::AttemptRecord,
};
use serde::{Deserialize, Serialize};
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metric {
    pub numerator: usize,
    pub denominator: usize,
}
impl Metric {
    pub fn fraction(&self) -> Option<f64> {
        if self.denominator == 0 {
            None
        } else {
            Some(self.numerator as f64 / self.denominator as f64)
        }
    }
}
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageEvidence {
    pub schema_version: String,
    pub execution: Metric,
    pub obligations: Metric,
    pub missing: Vec<Instance>,
    pub unsatisfied: Vec<Instance>,
    pub source_metrics: Vec<SourceMetric>,
    pub not_applicable_reasons: Vec<String>,
}
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainAssessment {
    pub schema_version: String,
    pub profile: String,
    pub decision: Decision,
    pub coverage: CoverageEvidence,
    pub findings: Vec<String>,
}
pub fn assess(
    plan: &FrozenPlan,
    attempt: &AttemptRecord,
    changes: &[Weakening],
) -> Result<DomainAssessment, String> {
    use crate::obligation::{VERSION, require};
    use crate::report::{CaseStatus, normalize::canonical_digest};
    plan.validate()?;
    attempt.validate()?;
    require(
        attempt.plan_digest == canonical_digest(plan)?,
        "attempt belongs to a different frozen plan",
    )?;
    let mut missing = Vec::new();
    let mut unsatisfied = Vec::new();
    let mut passed = 0;
    for i in plan.instances() {
        let found = attempt
            .observations
            .iter()
            .find(|o| o.test_id == i.test_id && o.environment == i.environment);
        match found {
            None => {
                missing.push(i.clone());
                unsatisfied.push(i.clone());
            }
            Some(o)
                if o.status == CaseStatus::Pass
                    && o.finished
                    && attempt.finished
                    && attempt.exit_code == Some(0) =>
            {
                passed += 1;
            }
            _ => unsatisfied.push(i.clone()),
        }
    }
    let satisfied = plan
        .obligations()
        .obligations
        .iter()
        .filter(|o| !unsatisfied.iter().any(|i| i.obligation_id == o.id))
        .count();
    let complete = unsatisfied.is_empty()
        && !plan.instances().is_empty()
        && attempt.finished
        && attempt.exit_code == Some(0);
    let decision = if !complete {
        Decision::Block
    } else if !changes.is_empty() {
        Decision::RequireApproval
    } else {
        Decision::Allow
    };
    Ok(DomainAssessment {
        schema_version: VERSION.into(),
        profile: "local-fixture-advisory".into(),
        decision,
        coverage: CoverageEvidence {
            schema_version: VERSION.into(),
            execution: Metric {
                numerator: passed,
                denominator: plan.instances().len(),
            },
            obligations: Metric {
                numerator: satisfied,
                denominator: plan.obligations().obligations.len(),
            },
            missing,
            unsatisfied,
            source_metrics: vec![],
            not_applicable_reasons: if plan.instances().is_empty() {
                vec!["execution and obligation coverage N/A: no required instances".into()]
            } else {
                vec![]
            },
        },
        findings: changes
            .iter()
            .map(|c| format!("baseline review required: {c:?}"))
            .collect(),
    })
}
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceMetric {
    pub kind: String,
    pub scope: String,
    pub filter: String,
    pub counts: Metric,
}
pub fn assess_with_metrics(
    plan: &FrozenPlan,
    attempt: &AttemptRecord,
    changes: &[Weakening],
    metrics: Vec<SourceMetric>,
) -> Result<DomainAssessment, String> {
    let mut keys = std::collections::BTreeSet::new();
    for m in &metrics {
        crate::obligation::require(
            ["statement", "branch"].contains(&m.kind.as_str())
                && !m.scope.is_empty()
                && !m.filter.is_empty()
                && m.counts.numerator <= m.counts.denominator
                && keys.insert((&m.kind, &m.scope)),
            "invalid or duplicate source metric",
        )?;
    }
    let mut result = assess(plan, attempt, changes)?;
    for m in &metrics {
        if m.counts.denominator == 0 {
            result.coverage.not_applicable_reasons.push(format!(
                "{} coverage N/A: empty denominator in {}",
                m.kind, m.scope
            ));
        }
    }
    result.coverage.source_metrics = metrics;
    Ok(result)
}
