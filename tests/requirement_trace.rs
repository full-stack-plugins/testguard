mod common;
use testguard::obligation::trace::trace;
#[test]
fn every_missing_or_failed_instance_traces_to_approved_revision_and_separate_requirement() {
    let plan = common::plan();
    let mut attempt = common::attempt(&plan);
    attempt.observations.remove(0);
    attempt.observations[0].status = testguard::report::CaseStatus::Fail;
    attempt.exit_code = Some(101);
    let links = trace(&plan, &attempt).unwrap();
    assert_eq!(links.len(), 6);
    for link in &links {
        assert_eq!(link.revision, "rev1");
        assert_eq!(link.approval_ref, "fixture:approval/1");
        if link.obligation_id == "O2" {
            assert_eq!(link.requirement_id, "REQ-2");
        } else {
            assert_eq!(link.requirement_id, "REQ-1");
        }
    }
    assert_eq!(links.iter().filter(|l| l.artifact_uri.is_none()).count(), 1);
    assert_eq!(links.iter().filter(|l| l.status == "fail").count(), 1);
}
#[test]
fn stale_attempt_cannot_supply_trace() {
    let plan = common::plan();
    let mut a = common::attempt(&plan);
    a.plan_digest = "f".repeat(64);
    assert!(trace(&plan, &a).is_err());
}
