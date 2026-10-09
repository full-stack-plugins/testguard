mod common;
use guardengine::integration::{Coverage, CoverageStatus, GuardRunEnvelope, eligibility::*};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
};
use testguard::{
    report::freshness::*,
    runner::scheduler::{Admission, Completion, Scheduler},
};
fn fixture() -> (testguard::plan::FrozenPlan, Completion, FreshnessContext) {
    let p = common::engine_plan();
    let inv = common::invocation(&p);
    // Protected controller derives expected mapping bytes independently before admission.
    let expected = testguard::report::engine_adapter::project(
        &p,
        &common::attempt(&p),
        &[],
        &[],
        testguard::report::engine_adapter::CAPABILITY,
    )
    .unwrap();
    let contract_digest = format!(
        "sha256:{}",
        testguard::report::normalize::bytes_digest(&expected.contract_bytes)
    );
    let s = Scheduler::new();
    let lease = match s.admit("request", &p, inv.clone(), &[], &[]).unwrap() {
        Admission::Start(l) => l,
        _ => panic!(),
    };
    let done = lease.complete(&common::attempt(&p)).unwrap();
    let scopes = testguard::report::engine_adapter::required_scopes(&p).unwrap();
    let context = FreshnessContext {
        invocation: inv.clone(),
        changes: vec![],
        advice: vec![],
        policy: EligibilityPolicy {
            binding: inv.binding,
            producer: guardengine::integration::Producer {
                guard: "TestGuard".into(),
                version: "0.1.0".into(),
                analyzer_id: "testguard-local-evidence".into(),
                analyzer_version: "testguard.engine-map/v1".into(),
            },
            required_scopes: scopes.clone(),
            contract_digest,
            action: "test.consume".into(),
            producer_principals: BTreeSet::from(["fixture-producer".into()]),
            approval_principals: BTreeMap::from([(
                "test.review".into(),
                BTreeSet::from(["reviewer".into()]),
            )]),
        },
        config_digest: format!("sha256:{}", "e".repeat(64)),
        coverage: Coverage {
            status: CoverageStatus::Complete,
            required_scopes: scopes.clone(),
            observed_scopes: scopes,
            missing_scopes: vec![],
        },
        approvals: BTreeMap::from([("test.review".into(), "approval:1".into())]),
    };
    (p, done, context)
}
struct Provider {
    record: Option<ApprovalRecord>,
    calls: Cell<usize>,
}
impl AuthorityProvider for Provider {
    fn verify_producer(
        &self,
        _: &GuardRunEnvelope,
        _: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        Err(AuthorityError::Unavailable)
    }
    fn verify_approval(&self, r: &str) -> Result<ApprovalRecord, AuthorityError> {
        self.calls.set(self.calls.get() + 1);
        if r != "approval:1" {
            return Err(AuthorityError::Untrusted);
        }
        self.record.clone().ok_or(AuthorityError::Unavailable)
    }
}
fn provider(c: &FreshnessContext) -> Provider {
    Provider {
        calls: Cell::new(0),
        record: Some(ApprovalRecord {
            principal: "reviewer".into(),
            purpose: "test.review".into(),
            action: c.policy.action.clone(),
            binding: c.policy.binding.clone(),
            contract_digest: c.policy.contract_digest.clone(),
            validity: Validity {
                issued_at: 99,
                expires_at: 200,
                revoked: false,
            },
        }),
    }
}
#[test]
fn same_run_refresh_revalidates_authority_without_changing_history() {
    let (p, done, c) = fixture();
    let original = serde_json::to_vec(done.bundle()).unwrap();
    let f = FrozenFreshness::freeze(&p, &done, &c, 100).unwrap();
    let port = provider(&c);
    let receipt = f.check(&p, &c, &port, 100).unwrap();
    receipt.refresh(&f, &p, &c, &port, 101).unwrap();
    assert_eq!(port.calls.get(), 2);
    assert_eq!(f.historical_bytes(), original);
    assert_eq!(serde_json::to_vec(done.bundle()).unwrap(), original);
}
#[test]
fn every_current_binding_component_invalidates_and_cross_run_is_never_cached() {
    let (p, done, c) = fixture();
    let f = FrozenFreshness::freeze(&p, &done, &c, 100).unwrap();
    let original = f.historical_bytes().to_vec();
    for n in 0..17 {
        let mut changed = c.clone();
        match n {
            0 => changed.invocation.binding.candidate_oid = "3".repeat(40),
            1 => changed.invocation.binding.base_oid = "4".repeat(40),
            2 => changed.invocation.binding.merge_group_id = Some("new-group".into()),
            3 => {
                changed.invocation.binding.source_snapshot_digest =
                    format!("sha256:{}", "d".repeat(64))
            }
            4 => changed.invocation.binding.baseline_digest = None,
            5 => changed.policy.action = "new-policy".into(),
            6 => changed.policy.producer.analyzer_version = "new-analyzer".into(),
            7 => changed.config_digest = format!("sha256:{}", "f".repeat(64)),
            8 => changed.coverage.observed_scopes.pop().map(|_| ()).unwrap(),
            9 => {
                changed
                    .approvals
                    .insert("test.review".into(), "approval:2".into());
            }
            10 => changed.invocation.run_id = "next-run".into(),
            11 => changed.policy.approval_principals.clear(),
            12 => changed.approvals.clear(),
            13 => changed.invocation.binding.task_id = "other".into(),
            14 => changed
                .changes
                .push(testguard::policy::Weakening::TestRemoved),
            15 => changed.advice.push("different advice".into()),
            _ => changed.policy.producer.version = "next".into(),
        };
        assert!(
            f.check(&p, &changed, &provider(&c), 101).is_err(),
            "component {n}"
        );
        assert_eq!(f.historical_bytes(), original);
    }
}
#[test]
fn unchanged_reference_cannot_hide_revocation_expiry_or_unavailability() {
    let (p, done, c) = fixture();
    let f = FrozenFreshness::freeze(&p, &done, &c, 100).unwrap();
    let mut port = provider(&c);
    let receipt = f.check(&p, &c, &port, 100).unwrap();
    port.record.as_mut().unwrap().validity.revoked = true;
    assert!(receipt.refresh(&f, &p, &c, &port, 101).is_err());
    port.record.as_mut().unwrap().validity.revoked = false;
    assert!(receipt.refresh(&f, &p, &c, &port, 200).is_err());
    assert!(receipt.refresh(&f, &p, &c, &port, 99).is_err());
    port.record = None;
    assert!(receipt.refresh(&f, &p, &c, &port, 102).is_err());
    assert_eq!(port.calls.get(), 4);
}
#[test]
fn freeze_requires_actual_full_work_and_producer_and_bounded_controller_inputs() {
    let (p, done, c) = fixture();
    for n in 0..7 {
        let mut changed = c.clone();
        match n {
            0 => changed
                .changes
                .push(testguard::policy::Weakening::TestRemoved),
            1 => changed.advice.push("not-executed".into()),
            2 => changed.policy.producer.analyzer_id = "other".into(),
            3 => changed.coverage.required_scopes.clear(),
            4 => changed.approvals.clear(),
            5 => changed.config_digest = "a".repeat(300_000),
            _ => changed.policy.contract_digest = format!("sha256:{}", "f".repeat(64)),
        };
        assert!(
            FrozenFreshness::freeze(&p, &done, &changed, 100).is_err(),
            "case {n}"
        );
    }
}
#[test]
fn changed_plan_policy_and_approval_record_fields_cannot_reuse_a_receipt() {
    let (p, done, c) = fixture();
    let f = FrozenFreshness::freeze(&p, &done, &c, 100).unwrap();
    let port = provider(&c);
    let receipt = f.check(&p, &c, &port, 100).unwrap();
    let mut binding = p.binding().clone();
    binding.policy_digest = "d".repeat(64);
    let changed = testguard::plan::FrozenPlan::freeze(p.obligations(), binding).unwrap();
    assert!(receipt.refresh(&f, &changed, &c, &port, 101).is_err());
    for n in 0..6 {
        let mut port = provider(&c);
        let r = port.record.as_mut().unwrap();
        match n {
            0 => r.binding.worktree_id = "other".into(),
            1 => r.purpose = "other".into(),
            2 => r.action = "other".into(),
            3 => r.principal = "candidate".into(),
            4 => r.contract_digest = format!("sha256:{}", "b".repeat(64)),
            _ => r.validity.issued_at = 102,
        };
        assert!(
            receipt.refresh(&f, &p, &c, &port, 101).is_err(),
            "record {n}"
        );
        assert_eq!(port.calls.get(), 1);
    }
}
