use testguard::{
    obligation::specguard::{CaseMapping, import_fixture},
    plan::Binding,
};
fn input() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../fixtures/integration/specguard-obligations.json"
    ))
    .unwrap()
}
fn binding() -> Binding {
    Binding {
        repository: "fixture:repo".into(),
        candidate: "2".repeat(40),
        base: "fixture:base".into(),
        source_digest: "1".repeat(64),
        policy_digest: "a".repeat(64),
    }
}
fn mapping() -> Vec<CaseMapping> {
    vec![CaseMapping {
        obligation_id: input()["obligations"][0]["id"].as_str().unwrap().into(),
        test_id: "native-case-1".into(),
        environments: vec!["linux".into(), "windows".into()],
    }]
}
#[test]
fn consumes_actual_specguard_local_export_and_freezes_external_mapping() {
    let imported = import_fixture(&input().to_string(), &mapping(), binding()).unwrap();
    assert_eq!(imported.plan.instances().len(), 2);
    assert_eq!(imported.plan.obligations().capability, "local-fixture");
    assert_eq!(imported.source.obligations[0].source.path, "specs/a.md");
}
#[test]
fn rejects_unsupported_partial_production_scope_drift_and_candidate_missing_mapping() {
    for (field, value) in [
        ("apiVersion", serde_json::json!("other")),
        ("complete", serde_json::json!(false)),
        ("authenticationProfile", serde_json::json!("production")),
        ("candidateOid", serde_json::json!("3".repeat(40))),
        ("extra", serde_json::json!(true)),
    ] {
        let mut v = input();
        v[field] = value;
        assert!(
            import_fixture(&v.to_string(), &mapping(), binding()).is_err(),
            "{field}"
        );
    }
    assert!(import_fixture(&input().to_string(), &[], binding()).is_err());
    let mut v = input();
    v["scope"] = serde_json::json!([]);
    assert!(import_fixture(&v.to_string(), &mapping(), binding()).is_err());
}

#[test]
fn export_and_mapping_admission_reject_before_expansion() {
    let mut oversized = input().to_string();
    oversized.push_str(&" ".repeat(1024 * 1024));
    assert!(import_fixture(&oversized, &mapping(), binding()).is_err());
    let mut mappings = mapping();
    mappings[0].environments = (0..65).map(|i| format!("env-{i}")).collect();
    assert!(import_fixture(&input().to_string(), &mappings, binding()).is_err());
}

fn protected_fixture() -> (
    String,
    testguard::obligation::specguard::ExpectedFixtureExport,
    Vec<CaseMapping>,
    Binding,
) {
    use testguard::obligation::specguard::ExpectedFixtureExport;
    let text = include_str!("../fixtures/integration/specguard-plan/obligations.json").to_owned();
    let source: serde_json::Value = serde_json::from_str(&text).unwrap();
    let provenance: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/integration/specguard-plan/PROVENANCE.json"
    ))
    .unwrap();
    let p = &provenance["expected"];
    let expected = ExpectedFixtureExport {
        artifact_digest: p["artifact_digest"].as_str().unwrap().into(),
        baseline_digest: p["baseline_digest"].as_str().unwrap().into(),
        source_digest: p["source_digest"].as_str().unwrap().into(),
        candidate_oid: p["candidate_oid"].as_str().unwrap().into(),
        scope: serde_json::from_value(p["scope"].clone()).unwrap(),
    };
    let mapping = source["obligations"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, o)| CaseMapping {
            obligation_id: o["id"].as_str().unwrap().into(),
            test_id: format!("case{i}"),
            environments: vec!["linux".into(), "windows".into()],
        })
        .collect();
    let binding = Binding {
        repository: "fixture:testguard-import".into(),
        candidate: expected.candidate_oid.clone(),
        base: expected.candidate_oid.clone(),
        source_digest: expected
            .source_digest
            .strip_prefix("sha256:")
            .unwrap()
            .into(),
        policy_digest: "a".repeat(64),
    };
    (text, expected, mapping, binding)
}
#[test]
fn pinned_actual_export_freezes_three_by_two_before_candidate_changes() {
    use testguard::obligation::specguard::PreparedFixtureImport;
    let (text, expected, mut mappings, binding) = protected_fixture();
    let prepared = PreparedFixtureImport::freeze(expected, &mappings, binding).unwrap();
    mappings.clear();
    let imported = prepared.import(&text).unwrap();
    assert_eq!(imported.plan.instances().len(), 6);
    assert_eq!(imported.plan.obligations().requirements.len(), 2);
    let full = common::attempt(&imported.plan);
    for removed in 0..6 {
        let mut partial = full.clone();
        partial.observations.remove(removed);
        let assessment = testguard::coverage::assess(&imported.plan, &partial, &[]).unwrap();
        assert_eq!(assessment.decision, testguard::policy::Decision::Block);
        assert_eq!(assessment.coverage.execution.denominator, 6);
        assert_eq!(assessment.coverage.execution.numerator, 5);
    }
    for field in [
        "baselineDigest",
        "sourceDigest",
        "candidateOid",
        "scope",
        "obligations",
        "authenticationProfile",
        "apiVersion",
    ] {
        let mut changed: serde_json::Value = serde_json::from_str(&text).unwrap();
        changed[field] = serde_json::json!("changed");
        assert!(prepared.import(&changed.to_string()).is_err(), "{field}");
    }
}
#[test]
fn matching_artifact_hash_cannot_replace_frozen_baseline_scope_or_mappings() {
    use testguard::obligation::specguard::PreparedFixtureImport;
    use testguard::report::normalize::bytes_digest;
    for field in [
        "baselineDigest",
        "scope",
        "obligations",
        "authenticationProfile",
    ] {
        let (text, mut expected, mappings, binding) = protected_fixture();
        let mut changed: serde_json::Value = serde_json::from_str(&text).unwrap();
        match field {
            "baselineDigest" => changed[field] = format!("sha256:{}", "f".repeat(64)).into(),
            "scope" | "obligations" => {
                changed[field].as_array_mut().unwrap().pop();
            }
            _ => changed[field] = "production".into(),
        }
        let changed = changed.to_string();
        expected.artifact_digest = format!("sha256:{}", bytes_digest(changed.as_bytes()));
        let prepared = PreparedFixtureImport::freeze(expected, &mappings, binding).unwrap();
        assert!(prepared.import(&changed).is_err(), "{field}");
    }
}
mod common;
