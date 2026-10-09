use std::{path::PathBuf, process::Command};
use testguard::adapters::{ExecutorProfile, RawArtifactSet, cargo::parse};
use testguard::report::{CaseStatus, normalize::ArtifactRef};
fn native(filter: &str) -> RawArtifactSet {
    let manifest =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/cargo/native/Cargo.toml");
    let common = [
        "test",
        "--manifest-path",
        manifest.to_str().unwrap(),
        "--lib",
        filter,
        "--",
    ];
    let list = Command::new("cargo")
        .args(common)
        .args(["--list", "--format", "terse"])
        .output()
        .unwrap();
    assert!(list.status.success());
    let result = Command::new("cargo")
        .args(common)
        .args(["--format", "pretty", "--test-threads=1"])
        .output()
        .unwrap();
    let artifact = ArtifactRef::from_bytes("native1", "stdout", &result.stdout).unwrap();
    RawArtifactSet {
        attempt_id: "native1".into(),
        inventory: String::from_utf8(list.stdout).unwrap(),
        output: String::from_utf8(result.stdout).unwrap(),
        artifact,
        exit_code: result.status.code(),
        interrupted: false,
    }
}
fn profile() -> ExecutorProfile {
    ExecutorProfile {
        tool: "cargo".into(),
        version: "1.99.0".into(),
        protocol: "libtest-pretty-v1".into(),
        target: "lib".into(),
        features: vec![],
        parameters: String::new(),
        environment: "linux".into(),
    }
}
#[test]
fn real_cargo_pass_fail_ignored_zero_and_partial() {
    for (filter, status, count) in [
        ("cases::pass", CaseStatus::Pass, 1),
        ("cases::fail", CaseStatus::Fail, 1),
        ("cases::ignored", CaseStatus::Skip, 1),
        ("no_matching_case", CaseStatus::Unknown, 0),
    ] {
        let raw = native(filter);
        let observations = parse(&raw, &profile()).unwrap();
        assert_eq!(observations.len(), count);
        if count > 0 {
            assert_eq!(observations[0].status, status);
            assert_eq!(observations[0].artifact_uri, raw.artifact.uri);
        }
    }
    let mut raw = native("cases::pass");
    raw.inventory.push_str("cases::missing: test\n");
    assert!(parse(&raw, &profile()).is_err());
}
#[test]
fn malformed_missing_old_and_unknown_profile_fail_closed() {
    let raw = native("cases::pass");
    let mut p = profile();
    p.version = "unknown".into();
    assert!(parse(&raw, &p).is_err());
    let mut stale = raw.clone();
    stale.attempt_id = "other".into();
    assert!(parse(&stale, &profile()).is_err());
    for text in [
        "",
        "not a report",
        "running 1 test\ntest cases::pass ... ok\n",
    ] {
        let mut bad = raw.clone();
        bad.output = text.into();
        bad.artifact = ArtifactRef::from_bytes("native1", "stdout", text.as_bytes()).unwrap();
        assert!(parse(&bad, &profile()).is_err());
    }
}
#[test]
fn real_capture_matrix_retains_timeout_unknown_and_raw_evidence() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out = std::env::temp_dir().join(format!("testguard-cargo-capture-{}", std::process::id()));
    let status = Command::new("python3")
        .arg(root.join("fixtures/cargo/capture.py"))
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());
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
        let d = out.join(name);
        let metadata: serde_json::Value =
            serde_json::from_slice(&std::fs::read(d.join("capture.json")).unwrap()).unwrap();
        let output = std::fs::read_to_string(d.join("report.input")).unwrap();
        let artifact = ArtifactRef::from_bytes("native1", "stdout", output.as_bytes()).unwrap();
        let raw = RawArtifactSet {
            attempt_id: "native1".into(),
            inventory: std::fs::read_to_string(d.join("inventory.raw")).unwrap(),
            output,
            artifact,
            exit_code: metadata["exit_code"].as_i64().map(|n| n as i32),
            interrupted: metadata["timed_out"].as_bool().unwrap(),
        };
        let result = parse(&raw, &profile());
        if ["missing-report", "malformed"].contains(&name) {
            assert!(result.is_err(), "{name}");
        } else {
            let cases = result.unwrap();
            if name == "timeout" {
                assert_eq!(cases.len(), 1);
                assert_eq!(cases[0].status, CaseStatus::Unknown);
                assert!(!cases[0].finished);
            }
        }
    }
}
#[test]
fn real_feature_scoped_target_is_preserved() {
    let manifest =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/cargo/native/Cargo.toml");
    let args = [
        "test",
        "--manifest-path",
        manifest.to_str().unwrap(),
        "--features",
        "extra",
        "--lib",
        "cases::feature_case",
        "--",
    ];
    let listed = Command::new("cargo")
        .args(args)
        .args(["--list", "--format", "terse"])
        .output()
        .unwrap();
    assert!(listed.status.success());
    let executed = Command::new("cargo")
        .args(args)
        .args(["--format", "pretty"])
        .output()
        .unwrap();
    assert!(executed.status.success());
    let mut p = profile();
    p.features = vec!["extra".into()];
    let artifact = ArtifactRef::from_bytes("native1", "stdout", &executed.stdout).unwrap();
    let raw = RawArtifactSet {
        attempt_id: "native1".into(),
        inventory: String::from_utf8(listed.stdout).unwrap(),
        output: String::from_utf8(executed.stdout).unwrap(),
        artifact,
        exit_code: executed.status.code(),
        interrupted: false,
    };
    let observations = parse(&raw, &p).unwrap();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].features, ["extra"]);
    assert_eq!(observations[0].target, "lib");
    assert_eq!(observations[0].native_id, "cases::feature_case");
}
