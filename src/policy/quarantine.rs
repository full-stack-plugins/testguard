//! Current protected-controller quarantine annotations, never technical waivers.
use crate::{
    plan::{FrozenPlan, Instance},
    report::normalize::canonical_digest,
};
use guardengine::integration::eligibility::{
    AuthorityProvider, EligibilityPolicy, validate_approval_record,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
pub const PURPOSE: &str = "test.quarantine";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlternativeSafeguard {
    pub description: String,
    pub evidence_reference: String,
    pub digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuarantineRequest {
    pub owner: String,
    pub reason: String,
    pub expires_at: i64,
    pub approval_reference: String,
    pub alternative: AlternativeSafeguard,
    pub targets: Vec<Instance>,
}
impl QuarantineRequest {
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 65536 {
            return Err("quarantine input budget".into());
        }
        serde_json::from_slice(bytes).map_err(|_| "invalid quarantine request".into())
    }
}
/// Supplied independently by a protected controller, never candidate configuration.
pub struct QuarantineAuthority {
    pub policy: EligibilityPolicy,
    pub owners: BTreeSet<String>,
    pub candidate_principals: BTreeSet<String>,
    pub max_lifetime_seconds: i64,
}
pub struct ProtectedQuarantine {
    request: QuarantineRequest,
    policy: EligibilityPolicy,
    plan_digest: String,
    digest: String,
    frozen_at: i64,
}
/// Historical audit only. Call refresh with current protected inputs before use.
/// ```compile_fail
/// let _: testguard::policy::quarantine::QuarantineReceipt = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Debug, Serialize)]
pub struct QuarantineReceipt {
    protected_digest: String,
    plan_digest: String,
    checked_at: i64,
    expires_at: i64,
    approval_principal: String,
}
fn bounded(text: &str, limit: usize) -> bool {
    !text.trim().is_empty() && text.len() <= limit && !text.contains('\0')
}
fn digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(crate::obligation::digest)
}
fn principal_set(set: &BTreeSet<String>) -> bool {
    !set.is_empty() && set.len() <= 64 && set.iter().all(|s| bounded(s, 256))
}
fn authority_budget(a: &QuarantineAuthority) -> Result<(), String> {
    let p = &a.policy;
    if !principal_set(&a.owners)
        || !principal_set(&a.candidate_principals)
        || !principal_set(&p.producer_principals)
        || p.approval_principals.len() != 1
        || !p
            .approval_principals
            .get(PURPOSE)
            .is_some_and(principal_set)
        || p.approval_principals
            .values()
            .any(|s| !s.is_disjoint(&a.candidate_principals))
        || p.required_scopes.is_empty()
        || p.required_scopes.len() > 4096
        || p.binding.requirement_ids.is_empty()
        || p.binding.requirement_ids.len() > 256
        || !digest(&p.contract_digest)
        || a.max_lifetime_seconds <= 0
    {
        return Err("invalid protected quarantine authority".into());
    }
    let mut total = 0usize;
    for s in [
        &p.binding.repo_id,
        &p.binding.task_id,
        &p.binding.worktree_id,
        &p.binding.candidate_oid,
        &p.binding.base_oid,
        &p.binding.source_snapshot_digest,
        &p.producer.guard,
        &p.producer.version,
        &p.producer.analyzer_id,
        &p.producer.analyzer_version,
        &p.action,
        &p.contract_digest,
    ]
    .into_iter()
    .chain(p.binding.merge_group_id.iter())
    .chain(p.binding.baseline_digest.iter())
    .chain(p.binding.requirement_ids.iter())
    .chain(p.required_scopes.iter())
    .chain(p.producer_principals.iter())
    .chain(p.approval_principals.keys())
    .chain(p.approval_principals.values().flatten())
    .chain(a.owners.iter())
    .chain(a.candidate_principals.iter())
    {
        if !bounded(s, 4096) {
            return Err("quarantine authority string budget".into());
        }
        total = total
            .checked_add(s.len())
            .ok_or("quarantine authority budget")?;
        if total > 131072 {
            return Err("quarantine authority byte budget".into());
        }
    }
    Ok(())
}
impl ProtectedQuarantine {
    pub fn freeze(
        plan: &FrozenPlan,
        mut request: QuarantineRequest,
        authority: &QuarantineAuthority,
        now: i64,
    ) -> Result<Self, String> {
        // Inspect borrowed metadata before policy clones or canonical serialization.
        authority_budget(authority)?;
        if now < 0
            || request
                .expires_at
                .checked_sub(now)
                .is_none_or(|s| s <= 0 || s > authority.max_lifetime_seconds)
            || !bounded(&request.owner, 256)
            || !authority.owners.contains(&request.owner)
            || !bounded(&request.reason, 4096)
            || !bounded(&request.approval_reference, 256)
            || !bounded(&request.alternative.description, 4096)
            || !bounded(&request.alternative.evidence_reference, 4096)
            || !digest(&request.alternative.digest)
            || request.targets.is_empty()
            || request.targets.len() > 256
            || request.targets.iter().any(|i| {
                [&i.obligation_id, &i.test_id, &i.environment]
                    .iter()
                    .any(|s| !bounded(s, 256))
            })
        {
            return Err("invalid quarantine descriptor or lifetime".into());
        }
        crate::coverage::preflight(plan, None, &[], &[])?;
        plan.validate()?;
        request.targets.sort();
        if request.targets.windows(2).any(|v| v[0] == v[1])
            || request
                .targets
                .iter()
                .any(|i| plan.instances().binary_search(i).is_err())
        {
            return Err("quarantine target is not an exact frozen obligation edge".into());
        }
        let b = &authority.policy.binding;
        let local = plan.binding();
        let baseline = format!("sha256:{}", plan.obligations().baseline_digest);
        let requirements = plan
            .obligations()
            .requirements
            .iter()
            .collect::<BTreeSet<_>>();
        if b.repo_id != local.repository
            || b.candidate_oid != local.candidate
            || b.base_oid != local.base
            || b.source_snapshot_digest != format!("sha256:{}", local.source_digest)
            || b.baseline_digest.as_deref() != Some(&baseline)
            || b.requirement_ids.len() != requirements.len()
            || b.requirement_ids.iter().collect::<BTreeSet<_>>() != requirements
            || authority.policy.required_scopes
                != crate::report::engine_adapter::required_scopes(plan)?
            || authority.policy.producer != crate::report::transport::producer()
        {
            return Err("quarantine policy differs from frozen plan/profile".into());
        }
        let plan_digest = canonical_digest(plan)?;
        let identity = canonical_digest(&(
            "testguard.quarantine/v1alpha1",
            &plan_digest,
            &request,
            authority.policy.content_digest(),
            &authority.owners,
            &authority.candidate_principals,
            authority.max_lifetime_seconds,
            now,
        ))?;
        let mut policy = authority.policy.clone();
        policy.action = format!("test.quarantine:{identity}");
        Ok(Self {
            request,
            policy,
            plan_digest,
            digest: identity,
            frozen_at: now,
        })
    }
    /// Expected approval policy derives from protected inputs, never returned approval data.
    pub fn approval_policy(&self) -> &EligibilityPolicy {
        &self.policy
    }
    pub fn request(&self) -> &QuarantineRequest {
        &self.request
    }
    /// Approves only the exact descriptor. No technical decision or coverage changes.
    pub fn authorize(
        &self,
        plan: &FrozenPlan,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<QuarantineReceipt, String> {
        if now < self.frozen_at || now >= self.request.expires_at {
            return Err("quarantine inactive or clock regression".into());
        }
        crate::coverage::preflight(plan, None, &[], &[])?;
        plan.validate()?;
        if canonical_digest(plan)? != self.plan_digest {
            return Err("quarantine frozen plan changed".into());
        }
        let record = provider
            .verify_approval(&self.request.approval_reference)
            .map_err(|_| "quarantine approval unavailable/untrusted")?;
        validate_approval_record(&record, &self.policy, PURPOSE, now)
            .map_err(|_| "quarantine approval invalid or inactive")?;
        Ok(QuarantineReceipt {
            protected_digest: self.digest.clone(),
            plan_digest: self.plan_digest.clone(),
            checked_at: now,
            expires_at: self.request.expires_at,
            approval_principal: record.principal,
        })
    }
}
impl QuarantineReceipt {
    pub fn refresh(
        &self,
        current: &ProtectedQuarantine,
        plan: &FrozenPlan,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<Self, String> {
        if self.protected_digest != current.digest
            || self.plan_digest != current.plan_digest
            || now < self.checked_at
        {
            return Err("quarantine receipt context or clock changed".into());
        }
        current.authorize(plan, provider, now)
    }
}
