use testguard::{
    adapters::{ExecutorProfile, RawArtifactSet, junit::parse},
    report::{CaseStatus, normalize::ArtifactRef},
};
fn profile() -> ExecutorProfile {
    ExecutorProfile {
        tool: "maven-surefire".into(),
        version: "3.5.2".into(),
        protocol: "junit-xml-v1".into(),
        target: "CasesTest".into(),
        features: vec![],
        parameters: String::new(),
        environment: "java21-linux".into(),
    }
}
fn raw(xml: &str, exit: i32) -> RawArtifactSet {
    RawArtifactSet {
        attempt_id: "junit1".into(),
        inventory: String::new(),
        output: xml.into(),
        artifact: ArtifactRef::from_bytes("junit1", "report.xml", xml.as_bytes()).unwrap(),
        exit_code: Some(exit),
        interrupted: false,
    }
}
#[test]
fn preserves_native_parameterized_case_identity_and_counts() {
    let xml = r#"<testsuite name="Suite" tests="3" failures="1" errors="0" skipped="1"><testcase classname="Clazz" name="case[0]"/><testcase classname="Clazz" name="case[1]"><failure>assertion</failure></testcase><testcase classname="Clazz" name="skip"><skipped/></testcase></testsuite>"#;
    let cases = parse(&raw(xml, 1), &profile()).unwrap();
    assert_eq!(cases.len(), 3);
    assert_eq!(cases[0].status, CaseStatus::Pass);
    assert_eq!(cases[1].status, CaseStatus::Fail);
    assert_eq!(cases[2].status, CaseStatus::Skip);
    assert_ne!(cases[0].test_id, cases[1].test_id);
    assert!(cases[0].native_id.contains("case[0]"));
}
#[test]
fn rejects_collision_entity_malformed_counts_stale_and_exit_conflict() {
    for xml in [
        r#"<testsuite name="Suite" tests="2" failures="0" errors="0" skipped="0"><testcase classname="C" name="x"/><testcase classname="C" name="x"/></testsuite>"#,
        r#"<!DOCTYPE a [<!ENTITY x SYSTEM "file:///etc/passwd">]><testsuite/>"#,
        r#"<testsuite name="s" tests="1" failures="0" errors="0" skipped="0"/>"#,
        "<broken>",
    ] {
        assert!(parse(&raw(xml, 0), &profile()).is_err());
    }
    let xml = r#"<testsuite name="Suite" tests="1" failures="0" errors="0" skipped="0"><testcase classname="C" name="x"/></testsuite>"#;
    assert!(parse(&raw(xml, 1), &profile()).is_err());
    let mut stale = raw(xml, 0);
    stale.attempt_id = "new".into();
    assert!(parse(&stale, &profile()).is_err());
}
#[test]
fn real_maven_capture_matrix_matches_native_outcomes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/junit/captured");
    for name in [
        "pass",
        "fail",
        "skip",
        "zero",
        "partial",
        "timeout",
        "missing-report",
        "malformed",
    ] {
        let d = root.join(name);
        let metadata: serde_json::Value =
            serde_json::from_slice(&std::fs::read(d.join("capture.json")).unwrap()).unwrap();
        let xml = std::fs::read_to_string(d.join("report.input.xml")).unwrap();
        let mut input = raw(&xml, metadata["exit_code"].as_i64().unwrap() as i32);
        input.interrupted = metadata["timed_out"].as_bool().unwrap();
        let result = parse(&input, &profile());
        match name {
            "pass" | "partial" => assert_eq!(result.unwrap()[0].status, CaseStatus::Pass),
            "fail" => assert_eq!(result.unwrap()[0].status, CaseStatus::Fail),
            "skip" => assert_eq!(result.unwrap()[0].status, CaseStatus::Skip),
            _ => assert!(result.is_err(), "{name} cannot imply ALLOW"),
        }
        for file in [
            "stdout.raw",
            "stderr.raw",
            "report.raw.xml",
            "report.input.xml",
        ] {
            assert_eq!(
                testguard::report::normalize::bytes_digest(&std::fs::read(d.join(file)).unwrap()),
                metadata["sha256"][file].as_str().unwrap()
            );
        }
    }
}
#[test]
fn unknown_native_status_cannot_silently_become_pass() {
    let xml = r#"<testsuite name="Suite" tests="1" failures="0" errors="0" skipped="0"><testcase classname="C" name="x"><unknown-status/></testcase></testsuite>"#;
    assert!(parse(&raw(xml, 0), &profile()).is_err());
}
