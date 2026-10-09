use testguard::report::normalize::ArtifactRef;
use testguard::report::{AttemptRecord, CaseObservation, CaseStatus};
fn attempt(status: CaseStatus) -> AttemptRecord {
    let artifact = ArtifactRef::from_bytes("run1", "output", b"native output").unwrap();
    AttemptRecord {
        schema_version: "testguard.local/v1".into(),
        attempt_id: "run1".into(),
        plan_digest: "a".repeat(64),
        exit_code: Some(0),
        finished: true,
        observations: vec![CaseObservation {
            test_id: "t".into(),
            native_id: "suite::case[1]".into(),
            parameters: "1".into(),
            target: "lib".into(),
            features: vec![],
            environment: "linux".into(),
            status,
            discovered: true,
            started: true,
            finished: true,
            artifact_uri: artifact.uri.clone(),
        }],
        artifacts: vec![artifact],
    }
}
#[test]
fn finished_pass_is_success() {
    assert!(attempt(CaseStatus::Pass).validate().is_ok());
    assert!(attempt(CaseStatus::Pass).is_success());
}
#[test]
fn skip_unknown_and_fail_never_pass() {
    for status in [CaseStatus::Skip, CaseStatus::Unknown, CaseStatus::Fail] {
        assert!(!attempt(status).is_success());
    }
}
#[test]
fn absent_end_record_never_passes() {
    let mut a = attempt(CaseStatus::Pass);
    a.observations[0].finished = false;
    assert!(a.validate().is_err());
    assert!(!a.is_success());
    a.finished = false;
    assert!(!a.is_success());
}
#[test]
fn dangling_artifact_or_foreign_attempt_rejected() {
    let mut a = attempt(CaseStatus::Pass);
    a.observations[0].artifact_uri = "evidence://old/output".into();
    assert!(a.validate().is_err());
    let mut a = attempt(CaseStatus::Pass);
    a.attempt_id = "new".into();
    assert!(a.validate().is_err());
}
#[test]
fn version_exit_conflict_duplicate_and_zero_rejected() {
    let mut a = attempt(CaseStatus::Pass);
    a.schema_version = "other".into();
    assert!(a.validate().is_err());
    let mut a = attempt(CaseStatus::Pass);
    a.exit_code = Some(101);
    assert!(a.validate().is_err());
    let mut a = attempt(CaseStatus::Pass);
    a.observations.push(a.observations[0].clone());
    assert!(a.validate().is_err());
    let mut a = attempt(CaseStatus::Pass);
    a.observations.clear();
    assert!(!a.is_success());
}
#[test]
fn finding_contract_requires_version_and_source_refs() {
    let value = serde_json::json!({"schema_version":"testguard.local/v1","code":"missing-instance","obligation_id":"O1","source_ids":["AC1"],"message":"required instance absent"});
    assert!(testguard::report::DomainFinding::parse(&value.to_string()).is_ok());
    let mut bad = value.clone();
    bad["source_ids"] = serde_json::json!([]);
    assert!(testguard::report::DomainFinding::parse(&bad.to_string()).is_err());
    let mut bad = value;
    bad["schema_version"] = "other".into();
    assert!(testguard::report::DomainFinding::parse(&bad.to_string()).is_err());
}
