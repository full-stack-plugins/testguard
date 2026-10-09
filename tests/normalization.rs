use testguard::report::normalize::{ArtifactRef, canonical_digest};
#[test]
fn normalization_is_repeatable_and_key_order_independent() {
    let a = serde_json::json!({"z":[1,2],"a":{"x":true}});
    let d = canonical_digest(&a).unwrap();
    for _ in 0..10 {
        assert_eq!(d, canonical_digest(&a).unwrap());
    }
    assert_eq!(
        d,
        canonical_digest(
            &serde_json::from_str::<serde_json::Value>(r#"{"a":{"x":true},"z":[1,2]}"#).unwrap()
        )
        .unwrap()
    );
}
#[test]
fn artifact_rejects_conflicts_escape_and_unknown_fields() {
    let a = ArtifactRef::from_bytes("run1", "stdout.txt", b"hello").unwrap();
    assert!(a.verify(b"hello").is_ok());
    assert!(a.verify(b"changed").is_err());
    assert!(ArtifactRef::from_bytes("../run1", "stdout", b"x").is_err());
    assert!(ArtifactRef::from_bytes("run1", "../secret", b"x").is_err());
    let mut v = serde_json::to_value(a).unwrap();
    v["trusted"] = true.into();
    assert!(serde_json::from_value::<ArtifactRef>(v).is_err());
}
