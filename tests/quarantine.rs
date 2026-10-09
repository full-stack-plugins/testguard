mod common;
use guardengine::integration::{GuardRunEnvelope, eligibility::*};
use std::collections::{BTreeMap, BTreeSet};
use testguard::{
    plan::FrozenPlan,
    policy::{Decision, quarantine::*},
    report::{CaseStatus, normalize::canonical_digest},
};
fn plan() -> FrozenPlan {
    common::engine_plan()
}
fn request(p: &FrozenPlan) -> QuarantineRequest {
    QuarantineRequest {
        owner: "maintainer".into(),
        reason: "diagnose intermittent native failure".into(),
        expires_at: 200,
        approval_reference: "approval:quarantine-1".into(),
        alternative: AlternativeSafeguard {
            description: "additional controller-reviewed regression coverage".into(),
            evidence_reference: "evidence://substitute/output".into(),
            digest: format!("sha256:{}", "d".repeat(64)),
        },
        targets: vec![p.instances()[0].clone()],
    }
}
fn authority(p: &FrozenPlan) -> QuarantineAuthority {
    let invocation = common::invocation(p);
    QuarantineAuthority {
        policy: EligibilityPolicy {
            binding: invocation.binding,
            producer: guardengine::integration::Producer {
                guard: "TestGuard".into(),
                version: "0.1.0".into(),
                analyzer_id: "testguard-local-evidence".into(),
                analyzer_version: "testguard.engine-map/v1".into(),
            },
            required_scopes: testguard::report::engine_adapter::required_scopes(p).unwrap(),
            contract_digest: format!("sha256:{}", "e".repeat(64)),
            action: "protected-quarantine-input".into(),
            producer_principals: BTreeSet::from(["controller-runner".into()]),
            approval_principals: BTreeMap::from([(
                PURPOSE.into(),
                BTreeSet::from(["reviewer".into()]),
            )]),
        },
        owners: BTreeSet::from(["maintainer".into()]),
        candidate_principals: BTreeSet::from(["candidate-author".into()]),
        max_lifetime_seconds: 100,
    }
}
struct Provider(Option<ApprovalRecord>);
impl AuthorityProvider for Provider {
    fn verify_producer(
        &self,
        _: &GuardRunEnvelope,
        _: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        Err(AuthorityError::Unavailable)
    }
    fn verify_approval(&self, r: &str) -> Result<ApprovalRecord, AuthorityError> {
        if r != "approval:quarantine-1" {
            return Err(AuthorityError::Untrusted);
        }
        self.0.clone().ok_or(AuthorityError::Unavailable)
    }
}
fn provider(q: &ProtectedQuarantine) -> Provider {
    let p = q.approval_policy();
    Provider(Some(ApprovalRecord {
        principal: "reviewer".into(),
        purpose: PURPOSE.into(),
        action: p.action.clone(),
        binding: p.binding.clone(),
        contract_digest: p.contract_digest.clone(),
        validity: Validity {
            issued_at: 99,
            expires_at: 201,
            revoked: false,
        },
    }))
}
#[test]
fn approved_descriptor_keeps_required_failure_and_exact_frozen_denominator() {
    let p = plan();
    let original = canonical_digest(&p).unwrap();
    let q = ProtectedQuarantine::freeze(&p, request(&p), &authority(&p), 100).unwrap();
    let port = provider(&q);
    let receipt = q.authorize(&p, &port, 100).unwrap();
    receipt.refresh(&q, &p, &port, 150).unwrap();
    let mut attempt = common::attempt(&p);
    attempt.observations[0].status = CaseStatus::Fail;
    attempt.exit_code = Some(1);
    let a = testguard::coverage::assess(&p, &attempt, &[]).unwrap();
    assert_eq!(a.decision, Decision::Block);
    assert_eq!(a.coverage.execution.denominator, 6);
    assert_eq!(a.coverage.execution.numerator, 0); // Existing nonzero-exit policy stays strict.
    assert_eq!(a.coverage.obligations.denominator, 3);
    assert_eq!(canonical_digest(&p).unwrap(), original);
    assert_eq!(
        q.request().alternative.evidence_reference,
        "evidence://substitute/output"
    );
}
#[test]
fn required_fields_targets_and_controller_limits_reject_before_qualification() {
    let p = plan();
    let original = serde_json::to_value(request(&p)).unwrap();
    for name in [
        "owner",
        "reason",
        "expires_at",
        "approval_reference",
        "alternative",
        "targets",
    ] {
        let mut v = original.clone();
        v.as_object_mut().unwrap().remove(name);
        assert!(QuarantineRequest::from_json(&serde_json::to_vec(&v).unwrap()).is_err());
    }
    for name in ["description", "evidence_reference", "digest"] {
        let mut v = original.clone();
        v["alternative"].as_object_mut().unwrap().remove(name);
        assert!(QuarantineRequest::from_json(&serde_json::to_vec(&v).unwrap()).is_err());
    }
    for case in 0..12 {
        let mut r = request(&p);
        match case {
            0 => r.owner.clear(),
            1 => r.reason = " ".into(),
            2 => r.approval_reference.clear(),
            3 => r.alternative.description.clear(),
            4 => r.alternative.evidence_reference.clear(),
            5 => r.alternative.digest = "bad".into(),
            6 => r.targets.clear(),
            7 => r.targets.push(r.targets[0].clone()),
            8 => r.targets[0].environment = "foreign-environment".into(),
            9 => r.expires_at = 100,
            10 => r.expires_at = 201,
            _ => r.reason = "x".repeat(4097),
        };
        assert!(
            ProtectedQuarantine::freeze(&p, r, &authority(&p), 100).is_err(),
            "case {case}"
        );
    }
    let mut v = original;
    v["approved"] = true.into();
    assert!(QuarantineRequest::from_json(&serde_json::to_vec(&v).unwrap()).is_err());
    assert!(QuarantineRequest::from_json(&vec![b' '; 65537]).is_err());
}
#[test]
fn authority_and_time_are_rechecked_and_candidate_cannot_self_approve() {
    let p = plan();
    let q = ProtectedQuarantine::freeze(&p, request(&p), &authority(&p), 100).unwrap();
    let mut port = provider(&q);
    let receipt = q.authorize(&p, &port, 100).unwrap();
    let original = port.0.clone().unwrap();
    for case in 0..9 {
        let mut changed = original.clone();
        match case {
            0 => changed.validity.revoked = true,
            1 => changed.validity.expires_at = 150,
            2 => changed.validity.issued_at = 151,
            3 => changed.principal = "candidate-author".into(),
            4 => changed.purpose = "other".into(),
            5 => changed.action = "other".into(),
            6 => changed.binding.candidate_oid = "f".repeat(40),
            7 => changed.contract_digest = format!("sha256:{}", "f".repeat(64)),
            _ => changed.principal = "unknown".into(),
        };
        port.0 = Some(changed);
        assert!(receipt.refresh(&q, &p, &port, 150).is_err(), "case {case}");
    }
    port.0 = None;
    assert!(receipt.refresh(&q, &p, &port, 150).is_err());
    port.0 = Some(original);
    for now in [99, 200, 201] {
        assert!(receipt.refresh(&q, &p, &port, now).is_err());
    }
    let later = receipt.refresh(&q, &p, &port, 150).unwrap();
    assert!(later.refresh(&q, &p, &port, 149).is_err());
    let mut a = authority(&p);
    a.policy
        .approval_principals
        .get_mut(PURPOSE)
        .unwrap()
        .insert("candidate-author".into());
    assert!(ProtectedQuarantine::freeze(&p, request(&p), &a, 100).is_err());
    a = authority(&p);
    a.candidate_principals.clear();
    assert!(ProtectedQuarantine::freeze(&p, request(&p), &a, 100).is_err());
    a = authority(&p);
    a.owners.clear();
    assert!(ProtectedQuarantine::freeze(&p, request(&p), &a, 100).is_err());
}
#[test]
fn metadata_scope_and_protected_policy_changes_require_fresh_exact_approval() {
    let p = plan();
    let original = canonical_digest(&p).unwrap();
    let q = ProtectedQuarantine::freeze(&p, request(&p), &authority(&p), 100).unwrap();
    let port = provider(&q);
    let receipt = q.authorize(&p, &port, 100).unwrap();
    for case in 0..7 {
        let mut r = request(&p);
        match case {
            0 => r.reason.push('x'),
            1 => r.expires_at -= 1,
            2 => r.targets = vec![p.instances()[1].clone()],
            3 => r.alternative.description.push('x'),
            4 => r.alternative.evidence_reference.push('x'),
            5 => r.alternative.digest = format!("sha256:{}", "f".repeat(64)),
            _ => r.approval_reference = "different-approval".into(),
        };
        let changed = ProtectedQuarantine::freeze(&p, r, &authority(&p), 100).unwrap();
        assert_ne!(q.approval_policy().action, changed.approval_policy().action);
        assert!(changed.authorize(&p, &port, 150).is_err());
        assert!(
            receipt
                .refresh(&changed, &p, &provider(&changed), 150)
                .is_err()
        );
    }
    for case in 0..6 {
        let mut a = authority(&p);
        match case {
            0 => a.policy.binding.repo_id = "other".into(),
            1 => a.policy.binding.base_oid = "f".repeat(40),
            2 => a.policy.binding.source_snapshot_digest = format!("sha256:{}", "f".repeat(64)),
            3 => a.policy.binding.baseline_digest = None,
            4 => a.policy.binding.requirement_ids = vec!["other".into()],
            _ => a.policy.required_scopes.clear(),
        };
        assert!(ProtectedQuarantine::freeze(&p, request(&p), &a, 100).is_err());
    }
    let mut a = authority(&p);
    a.max_lifetime_seconds = 101;
    let changed = ProtectedQuarantine::freeze(&p, request(&p), &a, 100).unwrap();
    assert!(
        receipt
            .refresh(&changed, &p, &provider(&changed), 150)
            .is_err()
    );
    assert_eq!(canonical_digest(&p).unwrap(), original);
}

#[test]
fn unapproved_replacement_plan_and_oversized_authority_cannot_reuse_exception() {
    let p = plan();
    let q = ProtectedQuarantine::freeze(&p, request(&p), &authority(&p), 100).unwrap();
    let port = provider(&q);
    let receipt = q.authorize(&p, &port, 100).unwrap();
    let mut obligations = common::obligations();
    obligations["obligations"].as_array_mut().unwrap().pop();
    obligations["baseline_digest"] = "f".repeat(64).into();
    obligations["approval_ref"] = "candidate:self-approval".into();
    let changed = FrozenPlan::freeze(
        &testguard::obligation::ObligationSet::parse(&obligations.to_string()).unwrap(),
        p.binding().clone(),
    )
    .unwrap();
    assert!(receipt.refresh(&q, &changed, &port, 150).is_err());
    assert!(ProtectedQuarantine::freeze(&changed, request(&changed), &authority(&p), 100).is_err());
    assert_eq!(p.instances().len(), 6);
    let mut a = authority(&p);
    a.policy.binding.task_id = "x".repeat(4097);
    assert!(ProtectedQuarantine::freeze(&p, request(&p), &a, 100).is_err());
    let mut r = request(&p);
    r.targets = vec![r.targets[0].clone(); 257];
    assert!(ProtectedQuarantine::freeze(&p, r, &authority(&p), 100).is_err());
}
