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
