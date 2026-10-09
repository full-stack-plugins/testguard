mod common;
use testguard::{
    coverage::{Metric, assess},
    policy::{Decision, Weakening},
};
#[test]
fn all_six_pass_and_each_missing_instance_blocks_despite_extra_pass() {
    let plan = common::plan();
    let full = common::attempt(&plan);
    assert_eq!(assess(&plan, &full, &[]).unwrap().decision, Decision::Allow);
    for n in 0..6 {
        let mut a = full.clone();
        a.observations.remove(n);
        let mut extra = full.observations[n].clone();
        extra.test_id = "extra".into();
        a.observations.push(extra);
        let result = assess(&plan, &a, &[]).unwrap();
        assert_eq!(result.decision, Decision::Block);
        assert_eq!(result.coverage.execution.denominator, 6);
        assert_eq!(result.coverage.execution.numerator, 5);
        assert_eq!(result.coverage.obligations.numerator, 2);
        assert_eq!(result.coverage.missing.len(), 1);
    }
}
#[test]
fn zero_denominator_is_not_a_percentage() {
    assert_eq!(
        Metric {
            numerator: 0,
            denominator: 0
        }
        .fraction(),
        None
    );
    assert_eq!(
        Metric {
            numerator: 1,
            denominator: 2
        }
        .fraction(),
        Some(0.5)
    );
}
#[test]
fn skip_unknown_and_incomplete_attempt_block() {
    let plan = common::plan();
    for status in [
        testguard::report::CaseStatus::Skip,
        testguard::report::CaseStatus::Unknown,
    ] {
        let mut a = common::attempt(&plan);
        a.observations[0].status = status;
        assert_eq!(assess(&plan, &a, &[]).unwrap().decision, Decision::Block);
    }
    let mut a = common::attempt(&plan);
    a.finished = false;
    a.exit_code = None;
    assert_eq!(assess(&plan, &a, &[]).unwrap().decision, Decision::Block);
}
#[test]
fn weakening_requires_review_but_never_repairs_missing_coverage() {
    let plan = common::plan();
    let mut a = common::attempt(&plan);
    let changes = [
        Weakening::FilterChanged,
        Weakening::ThresholdLowered,
        Weakening::AssertionRemoved,
    ];
    assert_eq!(
        assess(&plan, &a, &changes).unwrap().decision,
        Decision::RequireApproval
    );
    a.observations.pop();
    assert_eq!(
        assess(&plan, &a, &changes).unwrap().decision,
        Decision::Block
    );
    assert_eq!(
        assess(&plan, &a, &changes)
            .unwrap()
            .coverage
            .execution
            .denominator,
        6
    );
}
#[test]
fn stale_plan_digest_rejected() {
    let plan = common::plan();
    let mut a = common::attempt(&plan);
    a.plan_digest = "f".repeat(64);
    assert!(assess(&plan, &a, &[]).is_err());
}
#[test]
fn empty_denominators_include_an_explicit_na_reason_in_output() {
    let mut source =
        testguard::obligation::ObligationSet::parse(&common::obligations().to_string()).unwrap();
    source.obligations.clear();
    let plan = testguard::plan::FrozenPlan::freeze(
        &source,
        serde_json::from_value(common::binding()).unwrap(),
    )
    .unwrap();
    let result = assess(&plan, &common::attempt(&plan), &[]).unwrap();
    let output = serde_json::to_value(result).unwrap();
    assert!(
        output["coverage"]["not_applicable_reasons"]
            .as_array()
            .is_some_and(|a| !a.is_empty())
    );
}
