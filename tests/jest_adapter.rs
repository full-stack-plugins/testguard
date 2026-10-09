use std::path::{Path, PathBuf};
use testguard::{
    adapters::{ExecutorProfile, RawArtifactSet, jest},
    report::{CaseStatus, normalize::ArtifactRef},
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/jest/captured")
}
fn profile() -> ExecutorProfile {
    ExecutorProfile {
        tool: "jest".into(),
        version: "30.2.0".into(),
        protocol: "jest-json-distribution-v30.2.0".into(),
        target: "cases.test.cjs".into(),
        features: vec![],
        parameters: "runInBand;retry=0;cli=30.1.3".into(),
        environment: "linux-node-24.19.0".into(),
    }
}
fn raw_at(root: &Path, name: &str) -> RawArtifactSet {
    let d = root.join(name);
    let m: serde_json::Value =
        serde_json::from_slice(&std::fs::read(d.join("capture.json")).unwrap()).unwrap();
    let output = std::fs::read_to_string(d.join("report.input.json")).unwrap();
    let exit = m["exit_code"].as_i64().unwrap();
    RawArtifactSet {
        attempt_id: format!("jest-{name}"),
        inventory: std::fs::read_to_string(d.join("inventory.json")).unwrap(),
        artifact: ArtifactRef::from_bytes(
            &format!("jest-{name}"),
            "native-json",
            output.as_bytes(),
        )
        .unwrap(),
        output,
        exit_code: if exit < 0 { None } else { Some(exit as i32) },
        interrupted: m["interrupted"].as_bool().unwrap(),
    }
}
fn prepared(dir: &Path, name: &str) -> Result<jest::PreparedCases, String> {
    let manifest =
        std::fs::read_to_string(dir.join(name).join("controller-manifest.json")).unwrap();
    let m: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join(name).join("capture.json")).unwrap())
            .unwrap();
    let file = format!("{}/cases.test.cjs", m["native_cwd"].as_str().unwrap());
    jest::PreparedCases::freeze(&manifest, &file, &profile())
}
fn matrix(dir: &Path) {
    for name in [
        "pass",
        "fail",
        "skip",
        "zero",
        "partial",
        "timeout",
        "missing-report",
        "malformed",
        "hard-timeout",
        "collision",
    ] {
        let raw = raw_at(dir, name);
        let protected = prepared(dir, name);
        if name == "collision" {
            assert!(protected.is_err());
            continue;
        }
        let protected = protected.unwrap();
        let result = jest::parse(&raw, &profile(), &protected);
        if ["missing-report", "malformed", "collision"].contains(&name) {
            assert!(result.is_err(), "{name}");
            continue;
        }
        let cases = result.unwrap();
        let expected = match name {
            "zero" | "hard-timeout" => 0,
            "partial" | "timeout" => 2,
            _ => 1,
        };
        assert_eq!(cases.len(), expected, "{name}");
        if name == "pass" {
            assert_eq!(cases[0].status, CaseStatus::Pass);
            assert!(cases[0].started && cases[0].finished);
        }
        if name == "fail" {
            assert_eq!(cases[0].status, CaseStatus::Fail);
        }
        if name == "skip" {
            assert_eq!(cases[0].status, CaseStatus::Skip);
            assert!(!cases[0].started);
        }
        if name == "partial" {
            assert_eq!(
                cases
                    .iter()
                    .filter(|c| c.status == CaseStatus::Skip && c.finished)
                    .count(),
                1
            );
        }
        if name == "timeout" {
            assert_eq!(
                cases
                    .iter()
                    .filter(|c| c.status == CaseStatus::Pass)
                    .count(),
                1
            );
            assert_eq!(
                cases
                    .iter()
                    .filter(|c| c.status == CaseStatus::Fail)
                    .count(),
                1
            );
        }
        if name == "hard-timeout" {
            assert!(
                cases
                    .iter()
                    .all(|c| c.status == CaseStatus::Unknown && !c.finished)
            );
        }
        for c in &cases {
            assert_eq!(c.environment, profile().environment);
            assert_eq!(c.target, profile().target);
            assert_eq!(c.artifact_uri, raw.artifact.uri);
            assert!(!c.native_id.is_empty());
        }
        // Required IDs come only from the controller manifest frozen BEFORE execution.
        // Native listTests discovers files, never cases; no report may shrink this set.
        let ids: Vec<String> = protected.required_test_ids().map(str::to_owned).collect();
        let set=testguard::obligation::ObligationSet::parse(&serde_json::json!({"schema_version":"testguard.local/v1","capability":"local-fixture","approval_ref":"fixture:approval","revision":"fixed","baseline_digest":"a".repeat(64),"requirements":["R"],"sources":[{"id":"S","requirement_id":"R","kind":"acceptance"}],"environments":[profile().environment],"obligations":ids.iter().enumerate().map(|(i,id)|serde_json::json!({"id":format!("O{i}"),"source_ids":["S"],"test_id":id,"environments":[profile().environment]})).collect::<Vec<_>>()}).to_string()).unwrap();
        let binding=serde_json::from_value(serde_json::json!({"repository":"fixture:repo","candidate":"1".repeat(40),"base":"2".repeat(40),"source_digest":"b".repeat(64),"policy_digest":"c".repeat(64)})).unwrap();
        let plan = testguard::plan::FrozenPlan::freeze(&set, binding).unwrap();
        let attempt = testguard::report::AttemptRecord {
            schema_version: "testguard.local/v1".into(),
            attempt_id: raw.attempt_id.clone(),
            plan_digest: testguard::report::normalize::canonical_digest(&plan).unwrap(),
            exit_code: raw.exit_code,
            finished: !raw.interrupted,
            observations: cases,
            artifacts: vec![raw.artifact],
        };
        let assessed = testguard::coverage::assess(&plan, &attempt, &[]).unwrap();
        assert_eq!(assessed.coverage.execution.denominator, ids.len());
        assert_eq!(
            assessed.decision,
            if name == "pass" {
                testguard::policy::Decision::Allow
            } else {
                testguard::policy::Decision::Block
            },
            "{name}"
        );
    }
}
#[test]
fn actual_jest_matrix_never_promotes_skip_partial_or_timeout() {
    matrix(&root());
}
#[test]
fn native_count_exit_identity_and_scope_mutations_are_rejected() {
    let good = raw_at(&root(), "pass");
    for n in 0..13 {
        let mut bad = good.clone();
        let mut v: serde_json::Value = serde_json::from_str(&bad.output).unwrap();
        match n {
            0 => v["numPassedTests"] = 2.into(),
            1 => v["numPassedTestSuites"] = 2.into(),
            2 => v["testResults"][0]["assertionResults"][0]["status"] = "unknown".into(),
            3 => v["testResults"][0]["assertionResults"][0]["fullName"] = "changed".into(),
            4 => v["success"] = false.into(),
            5 => v["unknown"] = true.into(),
            6 => bad.exit_code = Some(1),
            7 => bad.attempt_id = "old-attempt".into(),
            8 => v["testResults"][0]["assertionResults"][0]["invocations"] = 2.into(),
            9 => v["testResults"][0]["endTime"] = 0.into(),
            10 => v["testResults"][0]["assertionResults"][0]["duration"] = u64::MAX.into(),
            11 => v["testResults"][0]["assertionResults"][0]["startAt"] = 0.into(),
            _ => {
                let duplicate = v["testResults"][0]["assertionResults"][0].clone();
                v["testResults"][0]["assertionResults"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
        }
        bad.output = serde_json::to_string(&v).unwrap();
        bad.artifact =
            ArtifactRef::from_bytes(&good.attempt_id, "native-json", bad.output.as_bytes())
                .unwrap();
        assert!(
            jest::parse(&bad, &profile(), &prepared(&root(), "pass").unwrap()).is_err(),
            "mutation {n}"
        );
    }
    let mut p = profile();
    p.version = "30.1.3".into();
    assert!(jest::parse(&good, &p, &prepared(&root(), "pass").unwrap()).is_err());
    p = profile();
    p.environment = "other-node".into();
    assert!(jest::parse(&good, &p, &prepared(&root(), "pass").unwrap()).is_err());
}
#[test]
fn report_bytes_and_budget_fail_closed() {
    let good = raw_at(&root(), "pass");
    let mut bad = good.clone();
    bad.output.push(' ');
    assert!(jest::parse(&bad, &profile(), &prepared(&root(), "pass").unwrap()).is_err());
    let mut bad = good;
    bad.output = " ".repeat(4 * 1024 * 1024 + 1);
    assert!(jest::parse(&bad, &profile(), &prepared(&root(), "pass").unwrap()).is_err());
}
#[test]
#[ignore = "requires declared installed Jest30.2.0 and Node24.19.0"]
fn rerun_real_jest_capture_and_parse_every_case() {
    let out = tempfile::tempdir().unwrap();
    let s = std::process::Command::new("python3")
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/jest/capture.py"))
        .arg(out.path())
        .status()
        .unwrap();
    assert!(s.success());
    matrix(out.path());
    let original = jest::parse(
        &raw_at(&root(), "pass"),
        &profile(),
        &prepared(&root(), "pass").unwrap(),
    )
    .unwrap();
    let fresh = jest::parse(
        &raw_at(out.path(), "pass"),
        &profile(),
        &prepared(out.path(), "pass").unwrap(),
    )
    .unwrap();
    assert_eq!(original[0].test_id, fresh[0].test_id);
}

#[test]
fn protected_manifest_file_discovery_and_resource_limits_fail_closed() {
    let good = raw_at(&root(), "pass");
    let protected = prepared(&root(), "pass").unwrap();
    for input in [
        "[]".to_owned(),
        "[\"/old-work/cases.test.cjs\"]".to_owned(),
        format!("{}0{}", "[".repeat(65), "]".repeat(65)),
        format!("[{}0]", "0,".repeat(131073)),
    ] {
        let mut bad = good.clone();
        bad.inventory = input;
        assert!(jest::parse(&bad, &profile(), &protected).is_err());
    }
    let source = std::fs::read_to_string(root().join("pass/controller-manifest.json")).unwrap();
    let native: Vec<String> = serde_json::from_str(&good.inventory).unwrap();
    for n in 0..5 {
        let mut v: serde_json::Value = serde_json::from_str(&source).unwrap();
        match n {
            0 => v["required_names"] = serde_json::json!(["same", "same"]),
            1 => v["schema_version"] = "unknown".into(),
            2 => v["required_names"] = serde_json::json!(["x".repeat(17000)]),
            3 => v["source_digest"] = "invalid".into(),
            _ => v["cli_version"] = "30.2.0".into(),
        }
        assert!(jest::PreparedCases::freeze(&v.to_string(), &native[0], &profile()).is_err());
    }
}

#[test]
fn file_discovery_does_not_fabricate_native_case_discovery() {
    let raw = raw_at(&root(), "hard-timeout");
    let protected = prepared(&root(), "hard-timeout").unwrap();
    assert!(
        jest::parse(&raw, &profile(), &protected)
            .unwrap()
            .is_empty()
    );
}
