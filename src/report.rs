pub mod normalize;
use serde::{Deserialize, Serialize};
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    Pass,
    Fail,
    Skip,
    Unknown,
}
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseObservation {
    pub test_id: String,
    pub native_id: String,
    pub parameters: String,
    pub target: String,
    pub features: Vec<String>,
    pub environment: String,
    pub status: CaseStatus,
    pub discovered: bool,
    pub started: bool,
    pub finished: bool,
    pub artifact_uri: String,
}
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptRecord {
    pub schema_version: String,
    pub attempt_id: String,
    pub plan_digest: String,
    pub exit_code: Option<i32>,
    pub finished: bool,
    pub observations: Vec<CaseObservation>,
    pub artifacts: Vec<normalize::ArtifactRef>,
}
impl AttemptRecord {
    pub fn validate(&self) -> Result<(), String> {
        use crate::obligation::{VERSION, digest, require, unique};
        require(
            self.schema_version == VERSION
                && !self.attempt_id.trim().is_empty()
                && digest(&self.plan_digest),
            "invalid execution version/id/plan digest",
        )?;
        require(
            unique(
                &self
                    .artifacts
                    .iter()
                    .map(|a| a.uri.clone())
                    .collect::<Vec<_>>(),
            ),
            "duplicate artifacts",
        )?;
        for a in &self.artifacts {
            a.validate(&self.attempt_id)?;
        }
        let mut ids = std::collections::BTreeSet::new();
        for o in &self.observations {
            require(
                !o.test_id.is_empty()
                    && !o.native_id.is_empty()
                    && !o.target.is_empty()
                    && !o.environment.is_empty()
                    && unique(&o.features),
                "invalid native identity",
            )?;
            require(
                ids.insert((&o.test_id, &o.environment)),
                "duplicate test/environment identity",
            )?;
            require(
                self.artifacts.iter().any(|a| a.uri == o.artifact_uri),
                "dangling artifact",
            )?;
            require(
                o.discovered
                    && (!o.started || o.discovered)
                    && (o.status != CaseStatus::Pass || (o.started && o.finished)),
                "pass lacks discovery/start/end",
            )?;
        }
        if self.finished {
            require(self.exit_code.is_some(), "completed execution lacks exit")?;
            let failed = self
                .observations
                .iter()
                .any(|o| o.status == CaseStatus::Fail);
            require(
                !(self.exit_code == Some(0) && failed),
                "zero exit contradicts failed case",
            )?;
            require(
                !(self.exit_code != Some(0)
                    && !self.observations.is_empty()
                    && self
                        .observations
                        .iter()
                        .all(|o| o.status == CaseStatus::Pass)),
                "nonzero exit contradicts all-pass report",
            )?;
        }
        Ok(())
    }
    pub fn is_success(&self) -> bool {
        self.validate().is_ok()
            && self.finished
            && self.exit_code == Some(0)
            && !self.observations.is_empty()
            && self
                .observations
                .iter()
                .all(|o| o.status == CaseStatus::Pass && o.finished)
    }
}
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainFinding {
    pub schema_version: String,
    pub code: String,
    pub obligation_id: String,
    pub source_ids: Vec<String>,
    pub message: String,
}
impl DomainFinding {
    pub fn parse(input: &str) -> Result<Self, String> {
        let finding: Self = serde_json::from_str(input).map_err(|e| e.to_string())?;
        crate::obligation::require(
            finding.schema_version == crate::obligation::VERSION
                && [&finding.code, &finding.obligation_id, &finding.message]
                    .iter()
                    .all(|s| !s.trim().is_empty())
                && !finding.source_ids.is_empty()
                && crate::obligation::unique(&finding.source_ids),
            "invalid finding contract",
        )?;
        Ok(finding)
    }
}
