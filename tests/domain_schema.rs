mod common;
use testguard::obligation::ObligationSet;
fn valid(v: serde_json::Value) -> bool {
    ObligationSet::parse(&v.to_string()).is_ok()
}
#[test]
fn accepts_explicit_local_fixture() {
    assert!(valid(common::obligations()));
}
#[test]
fn rejects_unknown_field() {
    let mut v = common::obligations();
    v["trusted"] = true.into();
    assert!(!valid(v));
}
#[test]
fn rejects_unknown_version() {
    let mut v = common::obligations();
    v["schema_version"] = "v2".into();
    assert!(!valid(v));
}
#[test]
fn rejects_production_capability() {
    let mut v = common::obligations();
    v["capability"] = "sg-approved".into();
    assert!(!valid(v));
}
#[test]
fn rejects_duplicate_obligation() {
    let mut v = common::obligations();
    v["obligations"][1]["id"] = "O1".into();
    assert!(!valid(v));
}
#[test]
fn rejects_dangling_source() {
    let mut v = common::obligations();
    v["obligations"][0]["source_ids"][0] = "missing".into();
    assert!(!valid(v));
}
#[test]
fn rejects_missing_environment() {
    let mut v = common::obligations();
    v["obligations"][0]["environments"] = serde_json::json!([]);
    assert!(!valid(v));
}
#[test]
fn rejects_unknown_environment() {
    let mut v = common::obligations();
    v["obligations"][0]["environments"][0] = "other".into();
    assert!(!valid(v));
}
#[test]
fn rejects_missing_approval_ref() {
    let mut v = common::obligations();
    v.as_object_mut().unwrap().remove("approval_ref");
    assert!(!valid(v));
}
#[test]
fn rejects_dangling_requirement() {
    let mut v = common::obligations();
    v["sources"][0]["requirement_id"] = "missing".into();
    assert!(!valid(v));
}
#[test]
fn rejects_duplicate_source() {
    let mut v = common::obligations();
    v["sources"][1]["id"] = "AC-1".into();
    assert!(!valid(v));
}
#[test]
fn rejects_duplicate_environment() {
    let mut v = common::obligations();
    v["environments"][1] = "linux".into();
    assert!(!valid(v));
}
#[test]
fn rejects_empty_test_id() {
    let mut v = common::obligations();
    v["obligations"][0]["test_id"] = "".into();
    assert!(!valid(v));
}
#[test]
fn rejects_unknown_nested_field() {
    let mut v = common::obligations();
    v["sources"][0]["approved"] = true.into();
    assert!(!valid(v));
}
