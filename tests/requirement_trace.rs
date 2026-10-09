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

#[test]
fn repeated_revision_expansion_is_rejected_before_trace_output() {
    let mut value = common::obligations();
    value["revision"] = serde_json::json!("x".repeat(3_000_000));
    let set = testguard::obligation::ObligationSet::parse(&value.to_string()).unwrap();
    let plan = testguard::plan::FrozenPlan::freeze(
        &set,
        serde_json::from_value(common::binding()).unwrap(),
    )
    .unwrap();
    let attempt = common::attempt(&plan);
    assert!(
        trace(&plan, &attempt).is_err(),
        "six copies of revision exceed trace output budget"
    );
}

#[test]
fn bidirectional_scope_queries_deduplicate_shared_source_links() {
    use testguard::obligation::trace::{TraceQuery, query};
    let mut v = common::obligations();
    v["sources"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"id":"AC-2","requirement_id":"REQ-1","kind":"acceptance"}));
    v["obligations"][0]["source_ids"] = serde_json::json!(["AC-1", "AC-2"]);
    let set = testguard::obligation::ObligationSet::parse(&v.to_string()).unwrap();
    let plan = testguard::plan::FrozenPlan::freeze(
        &set,
        serde_json::from_value(common::binding()).unwrap(),
    )
    .unwrap();
    let mut a = common::attempt(&plan);
    a.observations.remove(0);
    let report = query(&plan, &a, TraceQuery::Requirement("REQ-1")).unwrap();
    assert_eq!(report.execution.denominator, 4);
    assert_eq!(report.execution.numerator, 3);
    assert_eq!(report.obligations.denominator, 2);
    assert_eq!(report.obligations.numerator, 1);
    assert!(
        report
            .links
            .iter()
            .all(|e| e.requirement_id == "REQ-1" && e.revision == "rev1")
    );
    let back = query(
        &plan,
        &a,
        TraceQuery::Test {
            test_id: "case1",
            environment: "linux",
        },
    )
    .unwrap();
    assert_eq!(back.links.len(), 2);
    assert_eq!(back.execution.denominator, 1);
    assert_eq!(back.execution.numerator, 0);
    assert_eq!(
        query(&plan, &a, TraceQuery::Source("INV-1"))
            .unwrap()
            .execution
            .denominator,
        2
    );
    assert!(query(&plan, &a, TraceQuery::Requirement("foreign")).is_err());
    assert!(
        query(
            &plan,
            &a,
            TraceQuery::Test {
                test_id: "case1",
                environment: "foreign"
            }
        )
        .is_err()
    );
}

#[test]
fn shared_tests_do_not_duplicate_denominators_and_dangling_scope_is_rejected() {
    use testguard::obligation::trace::{TraceQuery, query};
    let mut v = common::obligations();
    v["obligations"][2]["test_id"] = "case1".into();
    let set = testguard::obligation::ObligationSet::parse(&v.to_string()).unwrap();
    let plan = testguard::plan::FrozenPlan::freeze(
        &set,
        serde_json::from_value(common::binding()).unwrap(),
    )
    .unwrap();
    let mut a = common::attempt(&plan);
    let mut seen = std::collections::BTreeSet::new();
    a.observations
        .retain(|o| seen.insert((o.test_id.clone(), o.environment.clone())));
    let all = query(&plan, &a, TraceQuery::All).unwrap();
    assert_eq!(all.execution.denominator, 4);
    assert_eq!(all.execution.numerator, 4);
    assert_eq!(all.obligations.denominator, 3);
    assert_eq!(all.obligations.numerator, 3);
    let reverse = query(
        &plan,
        &a,
        TraceQuery::Test {
            test_id: "case2",
            environment: "linux",
        },
    )
    .unwrap();
    assert!(reverse.links.iter().all(|l| l.requirement_id == "REQ-2"));
    assert!(matches!(
        reverse.sources[0].kind,
        testguard::obligation::SourceKind::Invariant
    ));
    let mut forged = serde_json::to_value(&plan).unwrap();
    forged["obligations"]["sources"][0]["requirement_id"] = "foreign".into();
    let forged: testguard::plan::FrozenPlan = serde_json::from_value(forged).unwrap();
    a.plan_digest = testguard::report::normalize::canonical_digest(&forged).unwrap();
    assert!(query(&forged, &a, TraceQuery::All).is_err());
    v["obligations"][0]["source_ids"] = serde_json::json!(["missing-source"]);
    assert!(testguard::obligation::ObligationSet::parse(&v.to_string()).is_err());
}
