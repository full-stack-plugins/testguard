#![cfg(target_os = "linux")]
mod common;
use std::collections::BTreeMap;
use testguard::report::{
    normalize::{ArtifactRef, canonical_digest},
    store::*,
};
fn policy() -> StorePolicy {
    StorePolicy {
        id: "fixture-policy-v1".into(),
        retention_seconds: 200,
        evidence_seconds: 100,
        max_attempts: 8,
        max_total_bytes: 32 * 1024 * 1024,
    }
}
fn provenance() -> Provenance {
    Provenance {
        execution_source: "fixture:executor".into(),
        tool: "cargo".into(),
        tool_version: "1.99.0".into(),
        analyzer: "fixture-analyzer".into(),
        analyzer_version: "1".into(),
        config_digest: "d".repeat(64),
        parent_attempt: None,
        command: vec!["cargo".into(), "--token=top-secret".into()],
        environment: BTreeMap::from([("TOKEN".into(), "top-secret".into())]),
        approval_refs: vec!["fixture:approval".into()],
    }
}
fn setup() -> (
    tempfile::TempDir,
    EvidenceStore,
    Administration,
    Access,
    RawAuditAccess,
) {
    let root = tempfile::tempdir().unwrap();
    let (store, admin) =
        EvidenceStore::open(root.path(), policy(), vec!["top-secret".into()]).unwrap();
    let (access, raw) = store.grant(&admin, "project-a").unwrap();
    (root, store, admin, access, raw)
}
fn append(store: &EvidenceStore, access: &Access, id: &str) -> Receipt {
    try_append(store, access, id).unwrap()
}
fn try_append(store: &EvidenceStore, access: &Access, id: &str) -> Result<Receipt, String> {
    let plan = common::plan();
    let mut attempt = common::attempt(&plan);
    attempt.attempt_id = id.into();
    let bytes = b"native failed report";
    let log = b"TOKEN=top-secret prior failure";
    let report = ArtifactRef::from_bytes(id, "native", bytes).unwrap();
    let logref = ArtifactRef::from_bytes(id, "log", log).unwrap();
    for o in &mut attempt.observations {
        o.artifact_uri = report.uri.clone();
        o.status = testguard::report::CaseStatus::Fail;
    }
    attempt.exit_code = Some(1);
    attempt.artifacts = vec![report.clone(), logref.clone()];
    let artifacts = [
        InputArtifact {
            reference: &report,
            bytes,
            kind: ArtifactKind::Evidence,
        },
        InputArtifact {
            reference: &logref,
            bytes: log,
            kind: ArtifactKind::Log,
        },
    ];
    store.append(
        access,
        AppendInput {
            plan: &plan,
            attempt: &attempt,
            provenance: &provenance(),
            artifacts: &artifacts,
        },
        1000,
    )
}
#[test]
fn append_preserves_failure_and_duplicate_attempt_never_overwrites() {
    let (_root, s, _admin, a, _) = setup();
    let r = append(&s, &a, "run1");
    let c = s.consume(&a, &r, 1001).unwrap();
    assert_eq!(c.attempt.exit_code, Some(1));
    assert_eq!(c.attempt.plan_digest, canonical_digest(&c.plan).unwrap());
    let before = s.view(&a, &r).unwrap();
    assert_eq!(before["exit_code"], 1);
    assert_eq!(before["case_counts"]["fail"], 6);
    assert_eq!(before["approved_revision"], "rev1");
    assert_eq!(
        before["requirement_ids"],
        serde_json::json!(["REQ-1", "REQ-2"])
    );
    assert!(try_append(&s, &a, "run1").is_err());
    assert_eq!(s.view(&a, &r).unwrap(), before);
    append(&s, &a, "run2");
    assert_eq!(s.consume(&a, &r, 1001).unwrap().attempt.exit_code, Some(1));
}
#[test]
fn namespaces_and_store_handles_are_access_isolated() {
    let (_root, s, _admin, a, _) = setup();
    let r = append(&s, &a, "run1");
    let (b, raw_b) = s.grant(&_admin, "project-b").unwrap();
    assert!(s.consume(&b, &r, 1001).is_err());
    assert!(s.raw_artifact(&raw_b, &r, "evidence://run1/log").is_err());
    let (_other, other, _other_admin, other_a, _) = setup();
    assert!(s.consume(&other_a, &r, 1001).is_err());
    assert!(other.consume(&a, &r, 1001).is_err());
    assert!(other.grant(&_admin, "project-a").is_err());
}
#[test]
fn views_redact_but_raw_audit_preserves_original_digest_and_bytes() {
    let (_root, s, _admin, a, raw) = setup();
    let r = append(&s, &a, "run1");
    let view = s.view(&a, &r).unwrap();
    let text = view.to_string();
    assert!(!text.contains("top-secret"));
    assert!(text.contains("[REDACTED]"));
    assert!(
        text.contains("execution_source")
            && text.contains("tool_version")
            && text.contains("plan_digest")
    );
    assert_eq!(
        s.raw_artifact(&raw, &r, "evidence://run1/log").unwrap(),
        b"TOKEN=top-secret prior failure"
    );
    assert!(
        String::from_utf8(s.raw_provenance(&raw, &r).unwrap())
            .unwrap()
            .contains("top-secret")
    );
    assert_eq!(
        view["artifacts"][1]["reference"]["digest"],
        testguard::report::normalize::bytes_digest(b"TOKEN=top-secret prior failure")
    );
}
#[test]
fn expiry_and_future_clock_refuse_consumption_but_keep_audit() {
    let (_root, s, _admin, a, _) = setup();
    let r = append(&s, &a, "run1");
    assert!(s.consume(&a, &r, 999).is_err());
    assert!(s.consume(&a, &r, 1100).is_err());
    assert!(s.view(&a, &r).is_ok());
}
fn writable(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}
#[test]
fn missing_required_bytes_fail_closed_even_when_audit_metadata_survives() {
    let (root, s, _admin, a, _) = setup();
    let r = append(&s, &a, "run1");
    let dir = root.path().join(r.storage_key());
    writable(&dir, 0o700);
    std::fs::remove_file(dir.join("raw-0")).unwrap();
    assert!(s.consume(&a, &r, 1001).is_err());
    assert_eq!(s.view(&a, &r).unwrap()["attempt_id"], "run1");
}
#[test]
fn retention_purge_preserves_manifest_and_never_restores_eligibility() {
    let (root, s, _admin, a, raw) = setup();
    let r = append(&s, &a, "run1");
    assert!(s.purge(&_admin, &a, &r, 1199).is_err());
    assert!(s.consume(&a, &r, 1099).is_ok());
    s.purge(&_admin, &a, &r, 1200).unwrap();
    s.purge(&_admin, &a, &r, 1201).unwrap();
    assert!(s.consume(&a, &r, 1001).is_err());
    assert!(s.raw_artifact(&raw, &r, "evidence://run1/native").is_err());
    assert!(s.view(&a, &r).is_ok());
    assert!(
        root.path()
            .join(r.storage_key())
            .join("manifest.json")
            .exists()
    );
}
#[test]
fn digest_symlink_hardlink_and_fifo_substitution_are_rejected() {
    use std::os::unix::fs::symlink;
    for mode in 0..4 {
        let (root, s, _admin, a, _) = setup();
        let r = append(&s, &a, "run1");
        let dir = root.path().join(r.storage_key());
        let path = dir.join("raw-0");
        writable(&dir, 0o700);
        if mode == 0 {
            writable(&path, 0o600);
            std::fs::write(&path, b"tampered native bytes").unwrap();
        } else {
            std::fs::remove_file(&path).unwrap();
            let outside = tempfile::tempdir().unwrap();
            let target = outside.path().join("payload");
            std::fs::write(&target, b"native failed report").unwrap();
            writable(&target, 0o600);
            match mode {
                1 => symlink(&target, &path).unwrap(),
                2 => std::fs::hard_link(&target, &path).unwrap(),
                _ => {
                    use rustix::fs::{CWD, FileType, Mode, mknodat};
                    mknodat(CWD, &path, FileType::Fifo, Mode::RUSR | Mode::WUSR, 0).unwrap();
                }
            }
            assert!(s.consume(&a, &r, 1001).is_err());
            continue;
        }
        assert!(s.consume(&a, &r, 1001).is_err());
    }
}
#[test]
fn explicit_policy_quota_and_reopen_keep_original_receipt_binding() {
    let root = tempfile::tempdir().unwrap();
    let mut p = policy();
    p.retention_seconds = 99;
    assert!(EvidenceStore::open(root.path(), p, vec![]).is_err());
    let mut p = policy();
    p.max_attempts = 1;
    let (s, admin) =
        EvidenceStore::open(root.path(), p.clone(), vec!["top-secret".into()]).unwrap();
    let (a, _) = s.grant(&admin, "project-a").unwrap();
    let receipt = append(&s, &a, "run1");
    assert!(try_append(&s, &a, "run2").is_err());
    let encoded = serde_json::to_vec(&receipt).unwrap();
    drop(s);
    let (reopened, new_admin) =
        EvidenceStore::open(root.path(), p, vec!["top-secret".into()]).unwrap();
    let (new, _) = reopened.grant(&new_admin, "project-a").unwrap();
    let parsed = Receipt::from_json(&encoded).unwrap();
    assert!(reopened.consume(&new, &parsed, 1001).is_ok());
    assert!(reopened.consume(&a, &parsed, 1001).is_err());
}
#[test]
fn concurrent_duplicate_append_has_one_immutable_winner() {
    let (root, s, _admin, access, _) = setup();
    let shared = std::sync::Arc::new(s);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let s = shared.clone();
            let a = access.clone();
            let b = barrier.clone();
            std::thread::spawn(move || {
                b.wait();
                try_append(&s, &a, "run1")
            })
        })
        .collect();
    let mut receipts = vec![];
    for h in handles {
        if let Ok(r) = h.join().unwrap() {
            receipts.push(r)
        }
    }
    assert_eq!(receipts.len(), 1);
    assert!(shared.consume(&access, &receipts[0], 1001).is_ok());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 2);
}
#[test]
fn byte_quota_and_unsafe_root_reject_before_publishing() {
    let root = tempfile::tempdir().unwrap();
    let mut p = policy();
    p.max_total_bytes = 1024;
    let (s, admin) = EvidenceStore::open(root.path(), p, vec![]).unwrap();
    let (a, _) = s.grant(&admin, "a").unwrap();
    assert!(try_append(&s, &a, "run1").is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    writable(root.path(), 0o755);
    assert!(EvidenceStore::open(root.path(), policy(), vec![]).is_err());
}
#[test]
fn collector_bytes_can_be_stored_without_rewriting_native_failure() {
    use testguard::runner::collect::ArtifactCollector;
    let (root, s, _admin, a, _) = setup();
    let plan = common::plan();
    let mut attempt = common::attempt(&plan);
    let report = include_bytes!("../fixtures/junit/captured/fail/report.input.xml");
    let reference = ArtifactRef::from_bytes("run1", "native", report).unwrap();
    for o in &mut attempt.observations {
        o.status = testguard::report::CaseStatus::Fail;
        o.artifact_uri = reference.uri.clone();
    }
    attempt.exit_code = Some(1);
    attempt.artifacts = vec![reference.clone()];
    let capture = tempfile::tempdir().unwrap();
    std::fs::write(capture.path().join("report.xml"), report).unwrap();
    let artifact = ArtifactCollector::open(capture.path())
        .unwrap()
        .read(std::path::Path::new("report.xml"), &reference, "run1")
        .unwrap();
    let r = s
        .append(
            &a,
            AppendInput {
                plan: &plan,
                attempt: &attempt,
                provenance: &provenance(),
                artifacts: &[InputArtifact {
                    reference: &artifact.reference,
                    bytes: &artifact.bytes,
                    kind: ArtifactKind::Evidence,
                }],
            },
            1000,
        )
        .unwrap();
    let stored = s.consume(&a, &r, 1001).unwrap();
    assert_eq!(stored.evidence[0].bytes, report);
    assert_eq!(stored.attempt.exit_code, Some(1));
    assert!(root.path().join(r.storage_key()).exists());
}
