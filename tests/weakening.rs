use testguard::policy::{ScopePolicy, detect_weakening};
#[test]
fn detects_threshold_filter_test_and_known_assertion_removal() {
    let base = ScopePolicy {
        threshold: 80,
        filters: vec!["generated".into()],
        tests: vec!["t1".into(), "t2".into()],
        assertions: vec!["assert1".into()],
    };
    let mut candidate = base.clone();
    candidate.threshold = 70;
    candidate.filters.push("src/**".into());
    candidate.tests.pop();
    candidate.assertions.clear();
    assert_eq!(detect_weakening(&base, &candidate).len(), 4);
    assert!(detect_weakening(&base, &base).is_empty());
}
