use crate::obligation::ObligationSet;
use serde::{Deserialize, Serialize};
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub repository: String,
    pub candidate: String,
    pub base: String,
    pub source_digest: String,
    pub policy_digest: String,
}
#[derive(
    schemars::JsonSchema, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(deny_unknown_fields)]
pub struct Instance {
    pub obligation_id: String,
    pub test_id: String,
    pub environment: String,
}
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenPlan {
    schema_version: String,
    obligations: ObligationSet,
    binding: Binding,
    instances: Vec<Instance>,
}
impl FrozenPlan {
    pub fn freeze(obligations: &ObligationSet, binding: Binding) -> Result<Self, String> {
        obligations.validate()?;
        crate::obligation::require(
            [&binding.repository, &binding.candidate, &binding.base]
                .iter()
                .all(|s| !s.trim().is_empty())
                && crate::obligation::digest(&binding.source_digest)
                && crate::obligation::digest(&binding.policy_digest),
            "invalid local fixture binding",
        )?;
        let mut instances: Vec<_> = obligations
            .obligations
            .iter()
            .flat_map(|o| {
                o.environments.iter().map(|e| Instance {
                    obligation_id: o.id.clone(),
                    test_id: o.test_id.clone(),
                    environment: e.clone(),
                })
            })
            .collect();
        instances.sort();
        Ok(Self {
            schema_version: crate::obligation::VERSION.into(),
            obligations: obligations.clone(),
            binding,
            instances,
        })
    }
    pub fn parse(input: &str) -> Result<Self, String> {
        let plan: Self = serde_json::from_str(input).map_err(|e| e.to_string())?;
        crate::obligation::require(
            plan.schema_version == crate::obligation::VERSION,
            "unsupported plan version",
        )?;
        let expected = Self::freeze(&plan.obligations, plan.binding.clone())?;
        crate::obligation::require(
            plan.instances == expected.instances,
            "frozen matrix mismatch",
        )?;
        Ok(plan)
    }
    pub fn validate(&self) -> Result<(), String> {
        crate::obligation::require(
            self.schema_version == crate::obligation::VERSION,
            "unsupported plan version",
        )?;
        let expected = Self::freeze(&self.obligations, self.binding.clone())?;
        crate::obligation::require(
            self.instances == expected.instances,
            "frozen matrix mismatch",
        )
    }
    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }
    pub fn obligations(&self) -> &ObligationSet {
        &self.obligations
    }
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
}
