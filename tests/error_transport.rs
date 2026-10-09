mod common;
use testguard::report::transport::{FailureKind, prepare};
#[test]
fn invalid_binding_or_unfrozen_coverage_has_only_transport_diagnostic() {
    let plan = common::engine_plan();
    let mut invocation = common::invocation(&plan);
    invocation.binding.candidate_oid.clear();
    let error = prepare(&plan, invocation).err().unwrap();
    let wire = serde_json::to_value(error).unwrap();
    assert!(wire.get("binding").is_none());
    assert!(wire.get("kind").is_none());
    let mut invocation = common::invocation(&plan);
    invocation.binding.source_snapshot_digest = format!("sha256:{}", "f".repeat(64));
    assert!(prepare(&plan, invocation).is_err());
    let mut invocation = common::invocation(&plan);
    invocation.finished_at = "2026-10-08T10:00:00Z".into();
    assert!(prepare(&plan, invocation).is_err());
}
#[test]
fn bound_parser_runtime_and_cancel_errors_have_null_decision_and_preserve_input() {
    let plan = common::engine_plan();
    for kind in [
        FailureKind::Parser,
        FailureKind::Runtime,
        FailureKind::Cancelled,
    ] {
        let bound = prepare(&plan, common::invocation(&plan)).unwrap();
        let bytes = b"known native failure before parser error";
        let output = bound.fail(kind, bytes).unwrap();
        assert_eq!(output.envelope.decision, None);
        assert_eq!(output.exit_code(), 4);
        assert!(output.artifacts.values().any(|v| v == bytes));
        output
            .envelope
            .validate(guardengine::integration::EvidenceProfile::EngineBacked)
            .unwrap();
    }
}
#[test]
fn bound_error_keeps_finished_failed_scope_and_raw_failure_without_claiming_completion() {
    let plan = common::engine_plan();
    let mut attempt = common::attempt(&plan);
    attempt.observations.truncate(1);
    attempt.observations[0].status = testguard::report::CaseStatus::Fail;
    attempt.finished = false;
    attempt.exit_code = None;
    let bytes = serde_json::to_vec(&attempt).unwrap();
    let bundle = prepare(&plan, common::invocation(&plan))
        .unwrap()
        .fail(FailureKind::Runtime, &bytes)
        .unwrap();
    assert_eq!(bundle.envelope.coverage.observed_scopes.len(), 1);
    assert_eq!(bundle.envelope.coverage.missing_scopes.len(), 5);
    assert_eq!(bundle.envelope.decision, None);
    assert!(bundle.envelope.artifacts.report.is_none());
    assert!(bundle.artifacts.values().any(|v| v == &bytes));
}
#[test]
fn invalid_oid_and_empty_required_scope_never_produce_a_bound_attempt() {
    let local_only = common::plan();
    assert!(prepare(&local_only, common::invocation(&local_only)).is_err());
    let mut source =
        testguard::obligation::ObligationSet::parse(&common::obligations().to_string()).unwrap();
    source.obligations.clear();
    let plan =
        testguard::plan::FrozenPlan::freeze(&source, common::engine_plan().binding().clone())
            .unwrap();
    assert!(prepare(&plan, common::invocation(&plan)).is_err());
    let plan = common::engine_plan();
    let mut invocation = common::invocation(&plan);
    invocation.capability = "production".into();
    assert!(prepare(&plan, invocation).is_err());
}
