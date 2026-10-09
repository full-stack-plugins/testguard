use std::path::{Path, PathBuf};
use testguard::{
    adapters::{ExecutorProfile, RawArtifactSet, vitest},
    report::{CaseStatus, normalize::ArtifactRef},
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/vitest/captured")
}
fn profile() -> ExecutorProfile {
    ExecutorProfile {
        tool: "vitest".into(),
        version: "4.0.18".into(),
        protocol: "vitest-json-v4.0.18".into(),
        target: "cases.test.mjs".into(),
        features: vec![],
        parameters: "pool=forks;retry=0;maxWorkers=1".into(),
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
        attempt_id: format!("vitest-{name}"),
        inventory: std::fs::read_to_string(d.join("inventory.json")).unwrap(),
        artifact: ArtifactRef::from_bytes(
            &format!("vitest-{name}"),
            "native-json",
            output.as_bytes(),
        )
        .unwrap(),
        output,
        exit_code: if exit < 0 { None } else { Some(exit as i32) },
        interrupted: m["interrupted"].as_bool().unwrap(),
    }
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
        "static-skip",
    ] {
        let raw = raw_at(dir, name);
        let result = vitest::parse(&raw, &profile());
        if ["missing-report", "malformed", "collision", "static-skip"].contains(&name) {
            assert!(result.is_err(), "{name}");
            continue;
        }
        let cases = result.unwrap();
        let expected = match name {
            "zero" => 0,
            "partial" | "timeout" | "hard-timeout" => 2,
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
        // Freeze from separate native discovery BEFORE interpreting execution. Zero retains a
        // separately declared required obligation rather than constructing a zero denominator.
        let mut discovery = raw.clone();
        discovery.output.clear();
        discovery.artifact = ArtifactRef::from_bytes(&raw.attempt_id, "native-json", b"").unwrap();
        discovery.interrupted = true;
        discovery.exit_code = None;
        let known = vitest::parse(&discovery, &profile()).unwrap();
        let ids = if known.is_empty() {
            vec!["required-but-not-discovered".to_string()]
        } else {
            known.iter().map(|c| c.test_id.clone()).collect()
        };
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
fn actual_vitest_matrix_never_promotes_skip_partial_or_timeout() {
    matrix(&root());
}
#[test]
fn native_count_exit_identity_and_scope_mutations_are_rejected() {
    let good = raw_at(&root(), "pass");
    for n in 0..8 {
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
            _ => bad.attempt_id = "old-attempt".into(),
        }
        bad.output = serde_json::to_string(&v).unwrap();
        bad.artifact =
            ArtifactRef::from_bytes(&good.attempt_id, "native-json", bad.output.as_bytes())
                .unwrap();
        assert!(vitest::parse(&bad, &profile()).is_err(), "mutation {n}");
    }
    let mut p = profile();
    p.version = "4.0.17".into();
    assert!(vitest::parse(&good, &p).is_err());
    p = profile();
    p.environment = "other-node".into();
    assert!(vitest::parse(&good, &p).is_err());
}
#[test]
fn report_bytes_and_budget_fail_closed() {
    let good = raw_at(&root(), "pass");
    let mut bad = good.clone();
    bad.output.push(' ');
    assert!(vitest::parse(&bad, &profile()).is_err());
    let mut bad = good;
    bad.output = " ".repeat(4 * 1024 * 1024 + 1);
    assert!(vitest::parse(&bad, &profile()).is_err());
}
#[test]
#[ignore = "requires declared installed Vitest4.0.18 and Node24.19.0"]
fn rerun_real_vitest_capture_and_parse_every_case() {
    let out = tempfile::tempdir().unwrap();
    let s = std::process::Command::new("python3")
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/vitest/capture.py"))
        .arg(out.path())
        .status()
        .unwrap();
    assert!(s.success());
    matrix(out.path());
    let original = vitest::parse(&raw_at(&root(), "pass"), &profile()).unwrap();
    let fresh = vitest::parse(&raw_at(out.path(), "pass"), &profile()).unwrap();
    assert_eq!(original[0].test_id, fresh[0].test_id);
}

#[test]
fn structural_budgets_and_inventory_drift_fail_closed() {
    let good = raw_at(&root(), "pass");
    for n in 0..4 {
        let mut bad = good.clone();
        let mut v: serde_json::Value = serde_json::from_str(&bad.inventory).unwrap();
        match n {
            0 => {
                let d = v[0].clone();
                v.as_array_mut().unwrap().push(d);
            }
            1 => v[0]["file"] = "/root/../escape/cases.test.mjs".into(),
            2 => v[0]["name"] = "x".repeat(17000).into(),
            _ => v[0]["file"] = "/old-work/cases.test.mjs".into(),
        }
        bad.inventory = serde_json::to_string(&v).unwrap();
        assert!(vitest::parse(&bad, &profile()).is_err());
    }
    for input in [
        format!("{}0{}", "[".repeat(65), "]".repeat(65)),
        format!("[{}0]", "0,".repeat(131073)),
    ] {
        let mut bad = good.clone();
        bad.inventory = input;
        assert!(vitest::parse(&bad, &profile()).is_err());
    }
}
