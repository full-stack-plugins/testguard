#![cfg(target_os = "linux")]
mod common;
use std::collections::BTreeMap;
use testguard::{
    policy::{
        Decision,
        retry::{RetryClass, RetrySession},
    },
    report::{AttemptRecord, CaseStatus, normalize::ArtifactRef, store::*},
};
fn provenance(parent: Option<&str>) -> Provenance {
    Provenance {
        execution_source: "fixture:executor".into(),
        tool: "cargo".into(),
        tool_version: "1.99.0".into(),
        analyzer: "fixture".into(),
        analyzer_version: "1".into(),
        config_digest: "d".repeat(64),
        parent_attempt: parent.map(str::to_owned),
        command: vec!["cargo".into(), "test".into()],
        environment: BTreeMap::from([("TOKEN".into(), "secret-value".into())]),
        approval_refs: vec!["fixture:approval".into()],
    }
}
fn setup() -> (tempfile::TempDir, EvidenceStore, Access, RawAuditAccess) {
    let root = tempfile::tempdir().unwrap();
    let (store, admin) = EvidenceStore::open(
        root.path(),
        StorePolicy {
            id: "retry-test".into(),
            retention_seconds: 200,
            evidence_seconds: 100,
            max_attempts: 32,
            max_total_bytes: 32 * 1024 * 1024,
        },
        vec!["secret-value".into()],
    )
    .unwrap();
    let (access, raw) = store.grant(&admin, "retry").unwrap();
    (root, store, access, raw)
}
fn attempt(
    plan: &testguard::plan::FrozenPlan,
    id: &str,
    fail: bool,
) -> (AttemptRecord, ArtifactRef, Vec<u8>) {
    let mut attempt = common::attempt(plan);
    attempt.attempt_id = id.into();
    let bytes = if fail {
        b"original fail".to_vec()
    } else {
        b"original pass".to_vec()
    };
    let reference = ArtifactRef::from_bytes(id, "native", &bytes).unwrap();
    for o in &mut attempt.observations {
        o.artifact_uri = reference.uri.clone();
        o.status = if fail {
            CaseStatus::Fail
        } else {
            CaseStatus::Pass
        };
    }
    attempt.exit_code = Some(if fail { 1 } else { 0 });
    attempt.artifacts = vec![reference.clone()];
    (attempt, reference, bytes)
}
#[test]
fn retries_keep_original_failures_and_never_replace_mandatory_failure() {
    for sequence in [[false, true], [true, false], [true, true]] {
        let (_root, store, access, raw) = setup();
        let plan = common::plan();
        let mut session =
            RetrySession::new(&store, access.clone(), &plan, &provenance(None), 2).unwrap();
        for (i, fail) in sequence.iter().enumerate() {
            let id = format!("run{i}");
            let parent = (i > 0).then(|| format!("run{}", i - 1));
            let p = provenance(parent.as_deref());
            let (a, r, b) = attempt(&plan, &id, *fail);
            session
                .record(
                    AppendInput {
                        plan: &plan,
                        attempt: &a,
                        provenance: &p,
                        artifacts: &[InputArtifact {
                            reference: &r,
                            bytes: &b,
                            kind: ArtifactKind::Evidence,
                        }],
                    },
                    1000 + i as u64,
                )
                .unwrap();
        }
        let assessment = session.assess(1002).unwrap();
        assert_eq!(assessment.decision, Decision::Block);
        assert!(assessment.cases.iter().all(|c| c.classification
            == if sequence[0] == sequence[1] {
                RetryClass::StableFailure
            } else {
                RetryClass::Flaky
            }));
        let receipts: Vec<_> = session.receipts().collect();
        assert_eq!(receipts.len(), 2);
        for (i, r) in receipts.iter().enumerate() {
            assert_eq!(
                store
                    .raw_artifact(&raw, r, &format!("evidence://run{i}/native"))
                    .unwrap(),
                if sequence[i] {
                    b"original fail".to_vec()
                } else {
                    b"original pass".to_vec()
                }
            );
            let p: serde_json::Value =
                serde_json::from_slice(&store.raw_provenance(&raw, r).unwrap()).unwrap();
            assert_eq!(p["environment"]["TOKEN"], "secret-value");
        }
        assert!(session.assess(1101).is_err());
    }
}

#[test]
fn retry_bounds_parent_and_execution_scope_fail_closed_without_overwriting() {
    for variant in 0..7 {
        let (_root, store, access, _raw) = setup();
        let plan = common::plan();
        assert!(RetrySession::new(&store, access.clone(), &plan, &provenance(None), 0).is_err());
        assert!(RetrySession::new(&store, access.clone(), &plan, &provenance(None), 9).is_err());
        let mut session = RetrySession::new(
            &store,
            access.clone(),
            &plan,
            &provenance(None),
            if variant == 0 { 1 } else { 2 },
        )
        .unwrap();
        let (a, r, b) = attempt(&plan, "first", false);
        let p = provenance(None);
        session
            .record(
                AppendInput {
                    plan: &plan,
                    attempt: &a,
                    provenance: &p,
                    artifacts: &[InputArtifact {
                        reference: &r,
                        bytes: &b,
                        kind: ArtifactKind::Evidence,
                    }],
                },
                1000,
            )
            .unwrap();
        assert_eq!(session.assess(1000).unwrap().decision, Decision::Allow);
        let before = store
            .view(&access, session.receipts().next().unwrap())
            .unwrap();
        let (mut a, r, b) = attempt(&plan, "second", true);
        let mut p = provenance(Some("first"));
        match variant {
            0 => {}
            1 => p.parent_attempt = Some("foreign".into()),
            2 => p.command.push("--filter=small".into()),
            3 => {
                p.environment
                    .insert("TOKEN".into(), "changed-secret".into());
            }
            4 => a.plan_digest = "f".repeat(64),
            5 => a.attempt_id = "first".into(),
            _ => p.config_digest = "e".repeat(64),
        }
        assert!(
            session
                .record(
                    AppendInput {
                        plan: &plan,
                        attempt: &a,
                        provenance: &p,
                        artifacts: &[InputArtifact {
                            reference: &r,
                            bytes: &b,
                            kind: ArtifactKind::Evidence
                        }]
                    },
                    1001
                )
                .is_err()
        );
        assert_eq!(session.receipts().count(), 1);
        assert_eq!(
            store
                .view(&access, session.receipts().next().unwrap())
                .unwrap(),
            before
        );
        assert!(
            session.assess(1001).is_err(),
            "rejected attempt cannot expose earlier green"
        );
    }
}
#[test]
fn skips_missing_and_artifact_loss_are_not_cached_as_pass() {
    for missing in [false, true] {
        let (_root, store, access, _raw) = setup();
        let plan = common::plan();
        let p = provenance(None);
        let mut session = RetrySession::new(&store, access, &plan, &p, 2).unwrap();
        assert_eq!(session.assess(1000).unwrap().decision, Decision::Block);
        let (mut a, r, b) = attempt(&plan, "first", false);
        if missing {
            a.observations.pop();
        } else {
            a.observations[0].status = CaseStatus::Skip;
        }
        session
            .record(
                AppendInput {
                    plan: &plan,
                    attempt: &a,
                    provenance: &p,
                    artifacts: &[InputArtifact {
                        reference: &r,
                        bytes: &b,
                        kind: ArtifactKind::Evidence,
                    }],
                },
                1000,
            )
            .unwrap();
        let (a, r, b) = attempt(&plan, "second", false);
        let p = provenance(Some("first"));
        session
            .record(
                AppendInput {
                    plan: &plan,
                    attempt: &a,
                    provenance: &p,
                    artifacts: &[InputArtifact {
                        reference: &r,
                        bytes: &b,
                        kind: ArtifactKind::Evidence,
                    }],
                },
                1001,
            )
            .unwrap();
        assert_eq!(session.assess(1002).unwrap().decision, Decision::Block);
        assert!(session.assess(999).is_err());
    }
    let (root, store, access, _raw) = setup();
    let plan = common::plan();
    let p = provenance(None);
    let mut session = RetrySession::new(&store, access, &plan, &p, 2).unwrap();
    let (a, r, b) = attempt(&plan, "first", false);
    session
        .record(
            AppendInput {
                plan: &plan,
                attempt: &a,
                provenance: &p,
                artifacts: &[InputArtifact {
                    reference: &r,
                    bytes: &b,
                    kind: ArtifactKind::Evidence,
                }],
            },
            1000,
        )
        .unwrap();
    assert_eq!(session.assess(1000).unwrap().decision, Decision::Allow);
    let key = session.receipts().next().unwrap().storage_key();
    let dir = root.path().join(key);
    // Test only: destroy a required raw blob, preserving immutable manifest.
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let file = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.file_name().unwrap().to_string_lossy().starts_with("raw-"))
        .unwrap();
    std::fs::remove_file(file).unwrap();
    assert!(session.assess(1001).is_err());
}
