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

#[test]
fn domain_artifact_reference_bounds_apply_before_use() {
    assert!(ArtifactRef::from_bytes(&"r".repeat(257), "out", b"x").is_err());
    assert!(ArtifactRef::from_bytes("run", &"n".repeat(257), b"x").is_err());
    assert!(ArtifactRef::from_bytes("run", "out", &vec![0; 16 * 1024 * 1024 + 1]).is_err());
    let mut a = ArtifactRef::from_bytes("run", "out", b"x").unwrap();
    a.size = 16 * 1024 * 1024 + 1;
    assert!(a.validate("run").is_err());
    a.size = 1;
    for media in [
        "x".repeat(257),
        "text/plain\nforged: value".into(),
        "文本/plain".into(),
    ] {
        a.media_type = media;
        assert!(a.validate("run").is_err());
    }
}

mod common;

#[test]
fn typed_domain_contracts_repeat_ten_times_without_rewriting_raw_artifacts() {
    use testguard::report::normalize::bytes_digest;
    use testguard::{coverage, obligation::ObligationSet, report::DomainFinding};
    let obligations = ObligationSet::parse(&common::obligations().to_string()).unwrap();
    let plan = common::plan();
    plan.validate().unwrap();
    let attempt = common::attempt(&plan);
    attempt.validate().unwrap();
    let assessment = coverage::assess(&plan, &attempt, &[]).unwrap();
    let finding = DomainFinding::parse(r#"{"schema_version":"testguard.local/v1","code":"MISSING","obligation_id":"O1","source_ids":["AC-1"],"message":"missing required execution"}"#).unwrap();
    let values = [
        serde_json::to_value(obligations).unwrap(),
        serde_json::to_value(plan).unwrap(),
        serde_json::to_value(attempt).unwrap(),
        serde_json::to_value(assessment.coverage).unwrap(),
        serde_json::to_value(finding).unwrap(),
    ];
    for value in values {
        let expected = canonical_digest(&value).unwrap();
        for _ in 0..10 {
            let parsed: serde_json::Value =
                serde_json::from_slice(&serde_json::to_vec_pretty(&value).unwrap()).unwrap();
            assert_eq!(canonical_digest(&parsed).unwrap(), expected);
        }
    }
    let raw_a = b"{\"a\":1}";
    let raw_b = b"{ \"a\" : 1 }\n";
    assert_ne!(bytes_digest(raw_a), bytes_digest(raw_b));
    let artifact = ArtifactRef::from_bytes("run", "report.json", raw_a).unwrap();
    assert!(artifact.verify(raw_b).is_err());
    assert_ne!(
        canonical_digest(&vec![1, 2]).unwrap(),
        canonical_digest(&vec![2, 1]).unwrap()
    );
}

#[test]
fn strict_bounded_reference_parser_separates_domain_and_integration_refs() {
    use testguard::report::normalize::{MAX_COMPONENT_BYTES, MAX_REFERENCE_BYTES};
    let run = "r".repeat(MAX_COMPONENT_BYTES);
    let name = "n".repeat(MAX_COMPONENT_BYTES);
    let reference = ArtifactRef::from_bytes(&run, &name, b"original").unwrap();
    let encoded = serde_json::to_vec(&reference).unwrap();
    let parsed = ArtifactRef::parse(&encoded, &run).unwrap();
    parsed.verify(b"original").unwrap();
    assert!(ArtifactRef::parse(&encoded, "foreign").is_err());
    assert!(ArtifactRef::parse(&vec![b' '; MAX_REFERENCE_BYTES + 1], &run).is_err());
    let mut unknown = serde_json::to_value(&reference).unwrap();
    unknown["trusted"] = true.into();
    assert!(ArtifactRef::parse(&serde_json::to_vec(&unknown).unwrap(), &run).is_err());
    let duplicate = format!(
        "{{\"size\":8,{}",
        std::str::from_utf8(&encoded)
            .unwrap()
            .trim_start_matches('{')
    );
    assert!(ArtifactRef::parse(duplicate.as_bytes(), &run).is_err());
    let mut wrong_digest = reference.clone();
    wrong_digest.digest = "0".repeat(64);
    assert!(wrong_digest.verify(b"original").is_err());
    let mut wrong_size = reference.clone();
    wrong_size.size += 1;
    assert!(wrong_size.verify(b"original").is_err());
    assert!(serde_json::from_slice::<guardengine::integration::ArtifactRef>(&encoded).is_err());
    let engine = guardengine::integration::ArtifactRef {
        uri: "artifact:run/report".into(),
        digest: format!("sha256:{}", reference.digest),
        media_type: "application/json".into(),
    };
    assert!(ArtifactRef::parse(&serde_json::to_vec(&engine).unwrap(), &run).is_err());
}
