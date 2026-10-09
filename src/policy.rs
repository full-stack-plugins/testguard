use serde::{Deserialize, Serialize};
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Decision {
    Allow,
    Block,
    RequireApproval,
}
impl Decision {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Allow => 0,
            Self::Block => 2,
            Self::RequireApproval => 3,
        }
    }
}
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weakening {
    FilterChanged,
    ThresholdLowered,
    AssertionRemoved,
    TestRemoved,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopePolicy {
    pub threshold: u8,
    pub filters: Vec<String>,
    pub tests: Vec<String>,
    pub assertions: Vec<String>,
}
pub fn detect_weakening(baseline: &ScopePolicy, candidate: &ScopePolicy) -> Vec<Weakening> {
    let mut changes = Vec::new();
    if candidate.threshold < baseline.threshold {
        changes.push(Weakening::ThresholdLowered);
    }
    let filters = |p: &ScopePolicy| {
        p.filters
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
    };
    if filters(baseline) != filters(candidate) {
        changes.push(Weakening::FilterChanged);
    }
    if baseline.tests.iter().any(|t| !candidate.tests.contains(t)) {
        changes.push(Weakening::TestRemoved);
    }
    if baseline
        .assertions
        .iter()
        .any(|a| !candidate.assertions.contains(a))
    {
        changes.push(Weakening::AssertionRemoved);
    }
    changes
}
