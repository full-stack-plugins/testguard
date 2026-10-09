mod common;
use testguard::{
    policy::Weakening,
    report::{envelope::verify_bundle, transport::prepare},
};
#[test]
fn completed_actual_coverage_and_engine_report_match_for_partial_and_review() {
    let plan = common::engine_plan();
    for state in ["pass", "partial", "review"] {
        let mut a = common::attempt(&plan);
        let changes = if state == "review" {
            vec![Weakening::AssertionRemoved]
        } else {
            vec![]
        };
        if state == "partial" {
            a.observations.pop();
        }
        let bundle = prepare(&plan, common::invocation(&plan))
            .unwrap()
            .complete(&a, &changes, &[])
            .unwrap();
        assert_eq!(
            bundle.exit_code(),
            match state {
                "pass" => 0,
                "partial" => 2,
                _ => 3,
            }
        );
        assert_eq!(
            bundle.envelope.coverage.observed_scopes.len(),
            if state == "partial" { 5 } else { 6 }
        );
        let report = verify_bundle(&plan, &bundle).unwrap();
        assert_eq!(Some(report.decision), bundle.envelope.decision);
        assert!(bundle.envelope.artifacts.contract.is_some());
        assert!(bundle.envelope.artifacts.facts.is_some());
        assert!(bundle.envelope.artifacts.report.is_some());
        let wire = serde_json::to_vec(&bundle.envelope).unwrap();
        guardengine::integration::load_envelope_json(
            &wire,
            guardengine::integration::EvidenceProfile::EngineBacked,
        )
        .unwrap();
    }
}
#[test]
fn unknown_fields_versions_wrong_scope_attempt_and_tampered_bytes_are_rejected() {
    let plan = common::engine_plan();
    let mut attempt = common::attempt(&plan);
    attempt.attempt_id = "foreign".into();
    let foreign = prepare(&plan, common::invocation(&plan))
        .unwrap()
        .complete(&attempt, &[], &[])
        .unwrap();
    assert_eq!(foreign.exit_code(), 4);
    assert_eq!(foreign.envelope.decision, None);
    let bundle = prepare(&plan, common::invocation(&plan))
        .unwrap()
        .complete(&common::attempt(&plan), &[], &[])
        .unwrap();
    let mut bad = bundle.clone();
    bad.envelope.api_version = "future".into();
    assert!(verify_bundle(&plan, &bad).is_err());
    let mut bad = bundle.clone();
    bad.envelope.coverage.required_scopes = vec!["different".into()];
    assert!(verify_bundle(&plan, &bad).is_err());
    let mut bad = bundle.clone();
    let uri = bad.envelope.artifacts.report.as_ref().unwrap().uri.clone();
    bad.artifacts.get_mut(&uri).unwrap().push(b' ');
    assert!(verify_bundle(&plan, &bad).is_err());
    let mut wire = serde_json::to_value(bundle).unwrap();
    wire["productionGate"] = true.into();
    assert!(serde_json::from_value::<testguard::report::envelope::FixtureBundle>(wire).is_err());
}
#[test]
fn oversized_domain_input_cannot_issue_an_unverifiable_completed_bundle() {
    let plan = common::engine_plan();
    let mut a = common::attempt(&plan);
    a.observations[0].native_id = "x".repeat(guardengine::integration::MAX_ARTIFACT_BYTES);
    let bundle = prepare(&plan, common::invocation(&plan))
        .unwrap()
        .complete(&a, &[], &[])
        .unwrap();
    assert_eq!(bundle.exit_code(), 4);
    assert!(bundle.envelope.artifacts.report.is_none());
}
#[test]
fn structurally_valid_scope_reduction_and_missing_engine_refs_cannot_verify() {
    let plan = common::engine_plan();
    let bundle = prepare(&plan, common::invocation(&plan))
        .unwrap()
        .complete(&common::attempt(&plan), &[], &[])
        .unwrap();
    let mut reduced = bundle.clone();
    reduced.envelope.coverage.required_scopes.pop();
    reduced.envelope.coverage.observed_scopes.pop();
    // GE shape validation cannot infer TestGuard's frozen domain requirements.
    reduced
        .envelope
        .validate(guardengine::integration::EvidenceProfile::EngineBacked)
        .unwrap();
    assert!(verify_bundle(&plan, &reduced).is_err());
    for reference in ["contract", "facts", "report"] {
        let mut missing = bundle.clone();
        match reference {
            "contract" => missing.envelope.artifacts.contract = None,
            "facts" => missing.envelope.artifacts.facts = None,
            _ => missing.envelope.artifacts.report = None,
        }
        assert!(verify_bundle(&plan, &missing).is_err(), "{reference}");
    }
}
