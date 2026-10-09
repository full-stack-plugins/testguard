#[test]
fn explicit_native_protocol_versions_only() {
    assert!(testguard::supports_profile(
        "cargo",
        "1.99.0",
        "libtest-pretty-v1"
    ));
    assert!(!testguard::supports_profile(
        "cargo",
        "1.98.0",
        "libtest-pretty-v1"
    ));
    assert!(!testguard::supports_profile("cargo", "1.99.0", "json"));
    assert!(!testguard::supports_profile("junit", "unknown", "xml"));
}
#[test]
fn pinned_surefire_parser_profile_is_explicit() {
    assert!(testguard::supports_profile(
        "maven-surefire",
        "3.5.2",
        "junit-xml-v1"
    ));
    assert!(!testguard::supports_profile(
        "gradle",
        "8.0",
        "junit-xml-v1"
    ));
    assert!(!testguard::supports_profile(
        "maven-surefire",
        "3.5.3",
        "junit-xml-v1"
    ));
}
