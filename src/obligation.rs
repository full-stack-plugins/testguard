use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
pub const VERSION: &str = "testguard.local/v1";
pub(crate) fn require(ok: bool, message: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(message.into()) }
}
pub(crate) fn unique(values: &[String]) -> bool {
    values.iter().all(|s| !s.trim().is_empty())
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}
pub(crate) fn digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub requirement_id: String,
    pub kind: SourceKind,
}
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Requirement,
    Acceptance,
    Invariant,
}
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Obligation {
    pub id: String,
    pub source_ids: Vec<String>,
    pub test_id: String,
    pub environments: Vec<String>,
}
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObligationSet {
    pub schema_version: String,
    pub capability: String,
    pub approval_ref: String,
    pub revision: String,
    pub baseline_digest: String,
    pub requirements: Vec<String>,
    pub sources: Vec<Source>,
    pub environments: Vec<String>,
    pub obligations: Vec<Obligation>,
}
impl ObligationSet {
    pub fn parse(input: &str) -> Result<Self, String> {
        let set: Self = serde_json::from_str(input).map_err(|e| e.to_string())?;
        set.validate()?;
        Ok(set)
    }
    pub fn validate(&self) -> Result<(), String> {
        require(self.schema_version == VERSION, "unsupported schema_version")?;
        require(
            self.capability == "local-fixture",
            "unsupported capability: production export unavailable",
        )?;
        require(
            !self.approval_ref.trim().is_empty()
                && !self.revision.trim().is_empty()
                && digest(&self.baseline_digest),
            "missing approval/revision or invalid baseline digest",
        )?;
        require(
            unique(&self.requirements) && unique(&self.environments),
            "duplicate or empty requirement/environment",
        )?;
        let source_ids: Vec<_> = self.sources.iter().map(|s| s.id.clone()).collect();
        require(unique(&source_ids), "duplicate or empty source id")?;
        require(
            unique(
                &self
                    .obligations
                    .iter()
                    .map(|o| o.id.clone())
                    .collect::<Vec<_>>(),
            ),
            "duplicate or empty obligation id",
        )?;
        for source in &self.sources {
            require(
                self.requirements.contains(&source.requirement_id),
                "dangling requirement",
            )?;
        }
        for o in &self.obligations {
            require(
                !o.test_id.trim().is_empty() && !o.source_ids.is_empty() && unique(&o.source_ids),
                "missing test/source or duplicate source",
            )?;
            require(
                o.source_ids.iter().all(|s| source_ids.contains(s)),
                "dangling source",
            )?;
            require(
                !o.environments.is_empty()
                    && unique(&o.environments)
                    && o.environments.iter().all(|e| self.environments.contains(e)),
                "missing, duplicate or unknown environment",
            )?;
        }
        Ok(())
    }
}

pub mod trace;

pub mod specguard;
