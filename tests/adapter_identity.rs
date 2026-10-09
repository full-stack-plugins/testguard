use testguard::{
    adapters::{ExecutorProfile, RawArtifactSet, cargo::parse},
    report::normalize::ArtifactRef,
};
fn input() -> (RawArtifactSet, ExecutorProfile) {
    let output = include_str!("../fixtures/cargo/captured/pass/report.input");
    (
        RawArtifactSet {
            attempt_id: "a1".into(),
            inventory: include_str!("../fixtures/cargo/captured/pass/inventory.raw").into(),
            output: output.into(),
            artifact: ArtifactRef::from_bytes("a1", "stdout", output.as_bytes()).unwrap(),
            exit_code: Some(0),
            interrupted: false,
        },
        ExecutorProfile {
            tool: "cargo".into(),
            version: "1.99.0".into(),
            protocol: "libtest-pretty-v1".into(),
            target: "lib".into(),
            features: vec!["a".into(), "b".into()],
            parameters: "value=1".into(),
            environment: "linux".into(),
        },
    )
}
#[test]
fn parameters_environment_target_features_disambiguate_stable_identity() {
    let (raw, profile) = input();
    let original = parse(&raw, &profile).unwrap()[0].clone();
    for field in ["environment", "parameters", "target", "features"] {
        let mut changed = profile.clone();
        match field {
            "environment" => changed.environment = "windows".into(),
            "parameters" => changed.parameters = "value=2".into(),
            "target" => changed.target = "integration".into(),
            _ => changed.features.push("c".into()),
        };
        assert_ne!(original.test_id, parse(&raw, &changed).unwrap()[0].test_id);
    }
    let mut reordered = profile;
    reordered.features.reverse();
    assert_eq!(
        original.test_id,
        parse(&raw, &reordered).unwrap()[0].test_id
    );
}
#[test]
fn duplicate_inventory_and_exit_or_digest_conflicts_reject() {
    let (raw, profile) = input();
    let mut duplicate = raw.clone();
    duplicate.inventory.push_str(&raw.inventory);
    assert!(parse(&duplicate, &profile).is_err());
    let mut contradiction = raw.clone();
    contradiction.exit_code = Some(101);
    assert!(parse(&contradiction, &profile).is_err());
    let mut tampered = raw;
    tampered.output.push_str("tampered");
    assert!(parse(&tampered, &profile).is_err());
}
#[test]
fn cargo_adapter_rejects_a_supported_junit_profile_for_real_cargo_bytes() {
    let (raw, mut profile) = input();
    profile.tool = "maven-surefire".into();
    profile.version = "3.5.2".into();
    profile.protocol = "junit-xml-v1".into();
    assert!(testguard::supports_profile(
        &profile.tool,
        &profile.version,
        &profile.protocol
    ));
    assert!(parse(&raw, &profile).is_err());
}
