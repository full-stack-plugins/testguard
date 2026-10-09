//! Same-run fixture freshness only. Never replaces store.consume, producer authentication,
//! or current protected-controller eligibility checks. Configuration is controller-declared.
use crate::{
    plan::FrozenPlan,
    policy::Weakening,
    report::{normalize::canonical_digest, transport::FixtureInvocation},
    runner::scheduler::{Completion, measure},
};
use guardengine::integration::{
    Coverage,
    eligibility::{AuthorityProvider, EligibilityPolicy, validate_approval_record},
};
use serde::Serialize;
use std::collections::BTreeMap;
/// Independently supplied protected expectations; never derive these from uploaded evidence.
#[derive(Clone, Serialize)]
pub struct FreshnessContext {
    pub invocation: FixtureInvocation,
    pub changes: Vec<Weakening>,
    pub advice: Vec<String>,
    pub policy: EligibilityPolicy,
    /// Protected controller configuration, not proof of native runner configuration.
    pub config_digest: String,
    pub coverage: Coverage,
    /// Must cover exactly every purpose required by protected policy.
    pub approvals: BTreeMap<String, String>,
}
/// Immutable same-run key and historical bytes. No cross-run cache mode exists.
pub struct FrozenFreshness {
    key: String,
    raw_digest: String,
    raw: Vec<u8>,
    frozen_at: i64,
}
/// Audit of freshness conditions only: no technical ALLOW or authentication grant.
/// ```compile_fail
/// let _: testguard::report::freshness::FreshnessReceipt = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Serialize)]
pub struct FreshnessReceipt {
    key: String,
    checked_at: i64,
}
fn text(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 4096 && !s.contains('\0')
}
fn budget(plan: &FrozenPlan, c: &FreshnessContext) -> Result<(), String> {
    // Borrowed counting serialization stops before allocation/hash/clone; counts bound
    // even empty-string collections. Native domain preflight adds relation budgets.
    let p = &c.policy;
    if c.approvals.len() > 32
        || p.approval_principals.len() > 32
        || p.producer_principals.is_empty()
        || p.producer_principals.len() > 64
        || p.approval_principals
            .values()
            .any(|s| s.is_empty() || s.len() > 64)
        || p.required_scopes.len() > 4096
        || c.coverage.required_scopes.len() > 4096
        || c.coverage.observed_scopes.len() > 4096
        || c.coverage.missing_scopes.len() > 4096
        || c.changes.len() > 64
        || c.advice.len() > 64
        || !text(&p.action)
        || !c
            .config_digest
            .strip_prefix("sha256:")
            .is_some_and(crate::obligation::digest)
        || !p.producer_principals.iter().all(|s| text(s))
        || !p
            .approval_principals
            .iter()
            .all(|(k, v)| text(k) && v.iter().all(|s| text(s)))
        || !c.approvals.iter().all(|(k, v)| text(k) && text(v))
        || !c.approvals.keys().eq(p.approval_principals.keys())
    {
        return Err("invalid freshness context or budget".into());
    }
    measure(&(plan, c), 256 * 1024)?;
    crate::coverage::preflight(plan, None, &c.changes, &c.advice)?;
    plan.validate()?;
    Ok(())
}
fn key(plan: &FrozenPlan, c: &FreshnessContext, raw_digest: &str) -> Result<String, String> {
    canonical_digest(&("testguard.same-run-freshness/v1alpha1", plan, c, raw_digest))
}
impl FrozenFreshness {
    pub fn freeze(
        plan: &FrozenPlan,
        completion: &Completion,
        current: &FreshnessContext,
        now: i64,
    ) -> Result<Self, String> {
        budget(plan, current)?;
        if now < 0 {
            return Err("invalid freshness clock".into());
        }
        completion.verify_work(plan, &current.invocation, &current.changes, &current.advice)?;
        let bundle = completion.bundle();
        measure(bundle, 4 * 1024 * 1024)?;
        let e = &bundle.envelope;
        if current.policy.binding != e.binding
            || current.policy.producer != e.producer
            || current.coverage != e.coverage
            || current.policy.required_scopes != e.coverage.required_scopes
            || e.artifacts.contract.as_ref().map(|r| &r.digest)
                != Some(&current.policy.contract_digest)
        {
            return Err("freshness expected policy/coverage differs from actual completion".into());
        }
        super::envelope::verify_bundle(plan, bundle)?;
        let raw = serde_json::to_vec(bundle).map_err(|_| "freshness serialization")?;
        let raw_digest = super::normalize::bytes_digest(&raw);
        Ok(Self {
            key: key(plan, current, &raw_digest)?,
            raw_digest,
            raw,
            frozen_at: now,
        })
    }
    /// Caller must independently obtain current protected expectations and consume required
    /// artifacts via EvidenceStore. This receipt does not assert either boundary succeeded.
    pub fn check(
        &self,
        plan: &FrozenPlan,
        current: &FreshnessContext,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<FreshnessReceipt, String> {
        budget(plan, current)?;
        if now < self.frozen_at || key(plan, current, &self.raw_digest)? != self.key {
            return Err("freshness context/run changed or clock regressed; rerun required".into());
        }
        // Required purposes derive from the independently protected policy frozen in key.
        for (purpose, reference) in &current.approvals {
            let record = provider
                .verify_approval(reference)
                .map_err(|_| "freshness approval unavailable/untrusted")?;
            validate_approval_record(&record, &current.policy, purpose, now)
                .map_err(|_| "freshness approval invalid or inactive")?;
        }
        Ok(FreshnessReceipt {
            key: self.key.clone(),
            checked_at: now,
        })
    }
    pub fn historical_bytes(&self) -> &[u8] {
        &self.raw
    }
}
impl FreshnessReceipt {
    pub fn refresh(
        &self,
        frozen: &FrozenFreshness,
        plan: &FrozenPlan,
        current: &FreshnessContext,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<Self, String> {
        if self.key != frozen.key || now < self.checked_at {
            return Err("freshness receipt changed or clock regressed".into());
        }
        frozen.check(plan, current, provider, now)
    }
}
