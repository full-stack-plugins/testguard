use crate::{plan::FrozenPlan, report::AttemptRecord};
use serde::{Deserialize, Serialize};
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceEntry {
    pub requirement_id: String,
    pub source_id: String,
    pub obligation_id: String,
    pub test_id: String,
    pub environment: String,
    pub revision: String,
    pub approval_ref: String,
    pub attempt_id: String,
    pub artifact_uri: Option<String>,
    pub status: String,
}
pub fn trace(plan: &FrozenPlan, attempt: &AttemptRecord) -> Result<Vec<TraceEntry>, String> {
    crate::coverage::assess(plan, attempt, &[])?;
    // Admit all expanded strings before any TraceEntry is cloned. The multiplier
    // and per-edge charge cover owned strings/rows as well as encoded field names.
    let mut budget = OutputBudget(0);
    for i in plan.instances() {
        let obligation = plan
            .obligations()
            .obligations
            .iter()
            .find(|o| o.id == i.obligation_id)
            .ok_or("dangling obligation")?;
        let observation = attempt
            .observations
            .iter()
            .find(|o| o.test_id == i.test_id && o.environment == i.environment);
        for id in &obligation.source_ids {
            let source = plan
                .obligations()
                .sources
                .iter()
                .find(|s| &s.id == id)
                .ok_or("dangling source")?;
            budget.charge(1024)?;
            budget.count(&(
                &source.requirement_id,
                id,
                &i.obligation_id,
                &i.test_id,
                &i.environment,
                &plan.obligations().revision,
                &plan.obligations().approval_ref,
                &attempt.attempt_id,
                observation.map(|o| &o.artifact_uri),
            ))?;
        }
    }
    let mut entries = Vec::new();
    for i in plan.instances() {
        let obligation = plan
            .obligations()
            .obligations
            .iter()
            .find(|o| o.id == i.obligation_id)
            .ok_or("dangling obligation")?;
        let observation = attempt
            .observations
            .iter()
            .find(|o| o.test_id == i.test_id && o.environment == i.environment);
        for id in &obligation.source_ids {
            let source = plan
                .obligations()
                .sources
                .iter()
                .find(|s| &s.id == id)
                .ok_or("dangling source")?;
            entries.push(TraceEntry {
                requirement_id: source.requirement_id.clone(),
                source_id: id.clone(),
                obligation_id: i.obligation_id.clone(),
                test_id: i.test_id.clone(),
                environment: i.environment.clone(),
                revision: plan.obligations().revision.clone(),
                approval_ref: plan.obligations().approval_ref.clone(),
                attempt_id: attempt.attempt_id.clone(),
                artifact_uri: observation.map(|o| o.artifact_uri.clone()),
                status: observation
                    .map(|o| format!("{:?}", o.status).to_lowercase())
                    .unwrap_or_else(|| "missing".into()),
            });
        }
    }
    Ok(entries)
}

struct OutputBudget(usize);
impl OutputBudget {
    fn charge(&mut self, bytes: usize) -> Result<(), String> {
        self.0 = self.0.saturating_add(bytes);
        if self.0 > guardengine::integration::MAX_ARTIFACT_BYTES {
            return Err("trace output budget exceeded".into());
        }
        Ok(())
    }
    fn count(&mut self, value: &impl Serialize) -> Result<(), String> {
        serde_json::to_writer(self, value).map_err(|_| "trace output budget exceeded".into())
    }
}
impl std::io::Write for OutputBudget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.charge(bytes.len().saturating_mul(2))
            .map_err(std::io::Error::other)?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
/// Select a forward requirement/source chain or reverse test-and-environment chain.
pub enum TraceQuery<'a> {
    All,
    Requirement(&'a str),
    Source(&'a str),
    Test {
        test_id: &'a str,
        environment: &'a str,
    },
}
#[derive(Debug, Serialize)]
pub struct TraceReport {
    pub links: Vec<TraceEntry>,
    pub sources: Vec<super::Source>,
    pub execution: crate::coverage::Metric,
    pub obligations: crate::coverage::Metric,
}
pub fn query(
    plan: &FrozenPlan,
    attempt: &AttemptRecord,
    scope: TraceQuery<'_>,
) -> Result<TraceReport, String> {
    let assessment = crate::coverage::assess(plan, attempt, &[])?;
    let known = match &scope {
        TraceQuery::All => true,
        TraceQuery::Requirement(id) => plan.obligations().requirements.iter().any(|v| v == id),
        TraceQuery::Source(id) => plan.obligations().sources.iter().any(|v| &v.id == id),
        TraceQuery::Test {
            test_id,
            environment,
        } => plan
            .instances()
            .iter()
            .any(|i| &i.test_id == test_id && &i.environment == environment),
    };
    if !known {
        return Err("unknown trace scope".into());
    }
    let links: Vec<_> = trace(plan, attempt)?
        .into_iter()
        .filter(|e| match &scope {
            TraceQuery::All => true,
            TraceQuery::Requirement(id) => &e.requirement_id == id,
            TraceQuery::Source(id) => &e.source_id == id,
            TraceQuery::Test {
                test_id,
                environment,
            } => &e.test_id == test_id && &e.environment == environment,
        })
        .collect();
    let executions: std::collections::BTreeSet<_> =
        links.iter().map(|e| (&e.test_id, &e.environment)).collect();
    let obligations: std::collections::BTreeSet<_> =
        links.iter().map(|e| &e.obligation_id).collect();
    let execution = crate::coverage::Metric {
        denominator: executions.len(),
        numerator: executions
            .iter()
            .filter(|(test, env)| {
                !assessment
                    .coverage
                    .unsatisfied
                    .iter()
                    .any(|i| &i.test_id == *test && &i.environment == *env)
            })
            .count(),
    };
    let obligations = crate::coverage::Metric {
        denominator: obligations.len(),
        numerator: obligations
            .iter()
            .filter(|id| {
                !assessment
                    .coverage
                    .unsatisfied
                    .iter()
                    .any(|i| &i.obligation_id == **id)
            })
            .count(),
    };
    let sources = plan
        .obligations()
        .sources
        .iter()
        .filter(|s| links.iter().any(|e| e.source_id == s.id))
        .cloned()
        .collect();
    let result = TraceReport {
        links,
        sources,
        execution,
        obligations,
    };
    OutputBudget(0).count(&result)?;
    Ok(result)
}
