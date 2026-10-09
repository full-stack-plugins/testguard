mod common;
use serde_json::{Value, json};
use testguard::{
    adapters::{ExecutorProfile, RawArtifactSet, junit::parse},
    obligation::ObligationSet,
    plan::FrozenPlan,
    policy::Decision,
    report::{
        AttemptRecord,
        normalize::{ArtifactRef, bytes_digest, canonical_digest},
    },
};
fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/junit/matrix")
}
fn load(tool: &str, name: &str) -> (Value, RawArtifactSet, ExecutorProfile) {
    let d = root().join("captured").join(tool).join(name);
    let m: Value = serde_json::from_slice(&std::fs::read(d.join("capture.json")).unwrap()).unwrap();
    for file in [
        "stdout.raw",
        "stderr.raw",
        "report.raw.xml",
        "report.input.xml",
    ] {
        assert_eq!(
            bytes_digest(&std::fs::read(d.join(file)).unwrap()),
            m["sha256"][file].as_str().unwrap()
        );
    }
    if !m["native_xml"].is_null() {
        let original = std::fs::read_to_string(d.join("report.raw.xml")).unwrap();
        let document = roxmltree::Document::parse(&original).unwrap();
        let suite = document.root_element();
        for key in ["tests", "failures", "errors", "skipped"] {
            assert_eq!(
                suite.attribute(key).unwrap().parse::<u64>().unwrap(),
                m["native_xml"]["counts"][key].as_u64().unwrap()
            );
        }
        let stdout = std::fs::read_to_string(d.join("stdout.raw")).unwrap();
        let c = &m["native_console_counts"];
        let marker = if tool == "maven" {
            format!(
                "Tests run: {}, Failures: {}, Errors: {}, Skipped: {}",
                c["tests"], c["failures"], c["errors"], c["skipped"]
            )
        } else {
            format!(
                "TG_NATIVE_COUNTS tests={} failures={} skipped={} successful={}",
                c["tests"], c["failures"], c["skipped"], c["successful"]
            )
        };
        assert!(
            stdout.contains(&marker),
            "{tool}/{name}: native counter line absent"
        );
    }
    assert_eq!(m["cleanup_complete"], true);
    assert_eq!(m["fresh_report_root"], true);
    let output = std::fs::read_to_string(d.join("report.input.xml")).unwrap();
    let attempt_id = format!("{tool}-{name}");
    let artifact = ArtifactRef::from_bytes(&attempt_id, "report.xml", output.as_bytes()).unwrap();
    let raw = RawArtifactSet {
        attempt_id,
        inventory: String::new(),
        output,
        artifact,
        exit_code: Some(m["exit_code"].as_i64().unwrap() as i32),
        interrupted: name == "timeout",
    };
    let p = ExecutorProfile {
        tool: m["profile"]["tool"].as_str().unwrap().into(),
        version: m["profile"]["version"].as_str().unwrap().into(),
        protocol: "junit-xml-v1".into(),
        target: format!("{tool}:matrix"),
        features: vec![],
        parameters: String::new(),
        environment: "java21-linux".into(),
    };
    (m, raw, p)
}
#[test]
fn actual_gradle_pass_profile_is_supported() {
    let (_, r, p) = load("gradle", "pass");
    assert_eq!(parse(&r, &p).unwrap().len(), 2);
}
#[test]
fn native_matrix_preserves_frozen_denominators_and_never_false_allows() {
    for tool in ["maven", "gradle"] {
        let (_, baseline, p) = load(tool, "pass");
        let cases = parse(&baseline, &p).unwrap();
        let mut o = common::obligations();
        o["requirements"] = json!(["REQ-1"]);
        o["sources"] = json!([{"id":"AC-1","requirement_id":"REQ-1","kind":"acceptance"}]);
        o["environments"] = json!(["java21-linux"]);
        o["obligations"]=json!(cases.iter().enumerate().map(|(i,c)|json!({"id":format!("O{i}"),"source_ids":["AC-1"],"test_id":c.test_id,"environments":["java21-linux"]})).collect::<Vec<_>>());
        let plan = FrozenPlan::freeze(
            &ObligationSet::parse(&o.to_string()).unwrap(),
            serde_json::from_value(common::binding()).unwrap(),
        )
        .unwrap();
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
            let (m, r, p) = load(tool, name);
            let parsed = parse(&r, &p);
            let valid = matches!(name, "pass" | "fail" | "skip" | "partial");
            assert_eq!(parsed.is_ok(), valid, "{tool}/{name}: {parsed:?}");
            assert_eq!(
                r.exit_code,
                Some(match name {
                    "fail" => 1,
                    "timeout" => -9,
                    _ => 0,
                })
            );
            if name == "timeout" {
                assert_eq!(m["testcase_ready"], true);
                assert!(!m["termination"].is_null());
            }
            if matches!(name, "missing-report" | "malformed") {
                assert!(m["fault_injection"].is_string());
            } else {
                assert!(m["fault_injection"].is_null());
            }
            if !m["native_xml"].is_null() {
                for key in ["tests", "failures", "skipped"] {
                    assert_eq!(
                        m["native_xml"]["counts"][key], m["native_console_counts"][key],
                        "{tool}/{name}/{key}"
                    );
                }
            }
            let attempt = AttemptRecord {
                schema_version: "testguard.local/v1".into(),
                attempt_id: r.attempt_id,
                plan_digest: canonical_digest(&plan).unwrap(),
                exit_code: r.exit_code,
                finished: valid,
                observations: parsed.unwrap_or_default(),
                artifacts: vec![r.artifact],
            };
            let a = testguard::coverage::assess(&plan, &attempt, &[]).unwrap();
            assert_eq!(a.coverage.execution.denominator, 2);
            assert_eq!(a.coverage.obligations.denominator, 2);
            assert_eq!(
                a.decision,
                if name == "pass" {
                    Decision::Allow
                } else {
                    Decision::Block
                },
                "{tool}/{name}"
            );
            if name == "partial" {
                assert_eq!(a.coverage.execution.numerator, 1);
                assert_eq!(a.coverage.obligations.numerator, 1);
                assert_eq!(a.coverage.missing.len(), 1);
            }
        }
    }
}
#[test]
fn native_parameter_identity_collision_and_foreign_attempt_ref_are_rejected() {
    for tool in ["maven", "gradle"] {
        let (_, r, p) = load(tool, "parameter-unique");
        let c = parse(&r, &p).unwrap();
        assert_eq!(c.len(), 2);
        assert_ne!(c[0].test_id, c[1].test_id);
        let (_, r, p) = load(tool, "parameter-collision");
        assert!(parse(&r, &p).unwrap_err().contains("duplicate"));
        let (_, mut r, p) = load(tool, "pass");
        r.attempt_id = "next-attempt".into();
        assert!(parse(&r, &p).is_err());
        let (_, mut r, p) = load(tool, "pass");
        r.exit_code = Some(1);
        assert!(parse(&r, &p).is_err());
        let (_, r, mut p) = load(tool, "pass");
        p.version = "unverified".into();
        assert!(parse(&r, &p).is_err());
    }
}
#[test]
fn captured_source_hashes_match_committed_inputs() {
    let m: Value =
        serde_json::from_slice(&std::fs::read(root().join("captured/provenance.json")).unwrap())
            .unwrap();
    for (file, hash) in m["sources"].as_object().unwrap() {
        assert_eq!(
            bytes_digest(&std::fs::read(root().join(file)).unwrap()),
            hash.as_str().unwrap()
        );
    }
    assert_eq!(
        bytes_digest(&std::fs::read(root().join("capture.py")).unwrap()),
        m["capture_script_sha256"].as_str().unwrap()
    );
    assert_eq!(
        m["tools"]["gradle_distribution"]["sha256"],
        "bd71102213493060956ec229d946beee57158dbd89d0e62b91bca0fa2c5f3531"
    );
}
