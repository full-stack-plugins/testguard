mod common;
use testguard::coverage::{Metric, SourceMetric, assess_with_metrics};
#[test]
fn full_statement_and_branch_coverage_cannot_replace_a_missing_requirement() {
    let plan = common::plan();
    let mut a = common::attempt(&plan);
    a.observations.pop();
    let metrics = vec![
        SourceMetric {
            kind: "statement".into(),
            scope: "src/**".into(),
            filter: "generated excluded".into(),
            counts: Metric {
                numerator: 100,
                denominator: 100,
            },
        },
        SourceMetric {
            kind: "branch".into(),
            scope: "src/**".into(),
            filter: "none".into(),
            counts: Metric {
                numerator: 10,
                denominator: 10,
            },
        },
    ];
    let result = assess_with_metrics(&plan, &a, &[], metrics).unwrap();
    assert_eq!(result.decision, testguard::policy::Decision::Block);
    assert_eq!(result.coverage.execution.denominator, 6);
    assert_eq!(result.coverage.source_metrics.len(), 2);
}
#[test]
fn source_metrics_reject_unknown_kind_overcount_and_duplicates() {
    let p = common::plan();
    let a = common::attempt(&p);
    for (kind, num, den) in [("statement", 2, 1), ("unknown", 0, 0)] {
        assert!(
            assess_with_metrics(
                &p,
                &a,
                &[],
                vec![SourceMetric {
                    kind: kind.into(),
                    scope: "src".into(),
                    filter: "none".into(),
                    counts: Metric {
                        numerator: num,
                        denominator: den
                    }
                }]
            )
            .is_err()
        );
    }
}
