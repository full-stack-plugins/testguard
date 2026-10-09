mod common;
use testguard::{
    policy::Weakening,
    report::{
        CaseStatus,
        engine_adapter::{CAPABILITY, project},
    },
};
#[test]
fn real_engine_mapping_matches_pass_fail_missing_review_and_advisory_vectors() {
    let p = common::plan();
    let a = common::attempt(&p);
    for (state, expected) in [
        ("pass", guardengine::Decision::Allow),
        ("fail", guardengine::Decision::Block),
        ("missing", guardengine::Decision::Block),
        ("review", guardengine::Decision::RequireApproval),
        ("advise", guardengine::Decision::Allow),
    ] {
        let mut a = a.clone();
        let mut changes = vec![];
        let mut advice = vec![];
        match state {
            "fail" => {
                a.observations[0].status = CaseStatus::Fail;
                a.exit_code = Some(101);
            }
            "missing" => {
                a.observations.pop();
            }
            "review" => changes.push(Weakening::FilterChanged),
            "advise" => advice.push("local advisory finding".to_string()),
            _ => {}
        }
        let output = project(&p, &a, &changes, &advice, CAPABILITY).unwrap();
        assert_eq!(output.report.decision, expected, "{state}");
        let contract = guardengine::load_contract_yaml(&output.contract_bytes).unwrap();
        let facts = guardengine::load_facts_json(&output.facts_bytes).unwrap();
        assert!(guardengine::verify_report(&output.report, &contract, &facts).unwrap());
        if state == "missing" {
            assert_eq!(facts.completeness, guardengine::Completeness::Partial);
            assert!(
                output
                    .report
                    .evaluations
                    .iter()
                    .all(|r| r.status == guardengine::RuleStatus::Indeterminate)
            );
        }
        if state == "advise" {
            assert!(
                output
                    .report
                    .evaluations
                    .iter()
                    .any(|r| r.enforcement == guardengine::Enforcement::Advise
                        && !r.matched_facts.is_empty()
                        && r.status == guardengine::RuleStatus::Pass)
            );
        }
    }
}
#[test]
fn unsupported_capability_and_unfinished_execution_fail_closed() {
    let p = common::plan();
    let mut a = common::attempt(&p);
    assert!(project(&p, &a, &[], &[], "production").is_err());
    a.finished = false;
    a.exit_code = None;
    assert!(project(&p, &a, &[], &[], CAPABILITY).is_err());
}
#[test]
fn captured_cargo_observation_replays_through_real_engine_and_envelope_verification() {
    use testguard::{
        adapters::{ExecutorProfile, RawArtifactSet, cargo::parse},
        report::{
            AttemptRecord,
            envelope::verify_bundle,
            normalize::{ArtifactRef, canonical_digest},
            transport::prepare,
        },
    };
    let profile = ExecutorProfile {
        tool: "cargo".into(),
        version: "1.99.0".into(),
        protocol: "libtest-pretty-v1".into(),
        target: "lib".into(),
        features: vec![],
        parameters: String::new(),
        environment: "linux".into(),
    };
    let id = canonical_digest(&(
        "cases::pass",
        &profile.target,
        &profile.features,
        &profile.parameters,
        &profile.environment,
    ))
    .unwrap();
    let mut source =
        testguard::obligation::ObligationSet::parse(&common::obligations().to_string()).unwrap();
    source.obligations.truncate(1);
    source.obligations[0].test_id = id;
    source.obligations[0].environments = vec!["linux".into()];
    let binding = common::engine_plan().binding().clone();
    let plan = testguard::plan::FrozenPlan::freeze(&source, binding).unwrap();
    let output = include_str!("../fixtures/cargo/captured/pass/report.input");
    let artifact =
        ArtifactRef::from_bytes("run1", "native-capture-replay", output.as_bytes()).unwrap();
    let raw = RawArtifactSet {
        attempt_id: "run1".into(),
        inventory: include_str!("../fixtures/cargo/captured/pass/inventory.raw").into(),
        output: output.into(),
        artifact: artifact.clone(),
        exit_code: Some(0),
        interrupted: false,
    };
    let attempt = AttemptRecord {
        schema_version: "testguard.local/v1".into(),
        attempt_id: "run1".into(),
        plan_digest: canonical_digest(&plan).unwrap(),
        exit_code: Some(0),
        finished: true,
        observations: parse(&raw, &profile).unwrap(),
        artifacts: vec![artifact],
    };
    let bundle = prepare(&plan, common::invocation(&plan))
        .unwrap()
        .complete(&attempt, &[], &[])
        .unwrap();
    let report = verify_bundle(&plan, &bundle).unwrap();
    assert_eq!(report.decision, guardengine::Decision::Allow);
    assert_eq!(bundle.envelope.coverage.observed_scopes.len(), 1);
}
#[test]
fn engine_projection_refuses_ge_expansion_budget_before_issuing_a_report() {
    let plan = common::plan();
    let attempt = common::attempt(&plan);
    // Individual input is below GE's artifact limit; three rule evaluations exceed its expansion budget.
    let advice = vec!["x".repeat(6 * 1024 * 1024)];
    assert!(project(&plan, &attempt, &[], &advice, CAPABILITY).is_err());
}
