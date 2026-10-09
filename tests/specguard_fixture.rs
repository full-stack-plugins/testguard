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
