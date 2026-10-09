use std::collections::BTreeSet;
use testguard::{
    coverage::advanced::*,
    obligation::{Obligation, ObligationSet, Source, SourceKind, VERSION},
    plan::{Binding, FrozenPlan},
    report::normalize::canonical_digest,
};
fn set() -> ObligationSet {
    ObligationSet {
        schema_version: VERSION.into(),
        capability: "local-fixture".into(),
        approval_ref: "fixture:advanced".into(),
        revision: "1".into(),
        baseline_digest: "a".repeat(64),
        requirements: vec!["fixture-requirement".into()],
        sources: units()
            .iter()
            .map(|u| Source {
                id: u.source.into(),
                requirement_id: "fixture-requirement".into(),
                kind: SourceKind::Invariant,
            })
            .collect(),
        environments: vec![ENVIRONMENT.into()],
        obligations: units()
            .iter()
            .map(|u| Obligation {
                id: format!("O:{}", u.id),
                source_ids: vec![u.source.into()],
                test_id: u.test.into(),
                environments: vec![ENVIRONMENT.into()],
            })
            .collect(),
    }
}
fn freeze(set: &ObligationSet) -> FrozenPlan {
    FrozenPlan::freeze(
        set,
        Binding {
            repository: "fixture-repo".into(),
            candidate: "fixture-current".into(),
            base: "fixture-base".into(),
            source_digest: source_digest(),
            policy_digest: "b".repeat(64),
        },
    )
    .unwrap()
}
fn scopes() -> Vec<MetricScope> {
    [
        MetricKind::Contract,
        MetricKind::State,
        MetricKind::Concurrency,
        MetricKind::Mutation,
    ]
    .into_iter()
    .map(|kind| MetricScope {
        kind,
        scope: "fixed-account-instrumentation".into(),
        filter: "explicit-versioned-unit-inventory".into(),
        units: units()
            .iter()
            .filter(|u| u.kind == kind)
            .map(|u| UnitSelection {
                id: u.id.into(),
                excluded_reason: None,
            })
            .collect(),
    })
    .collect()
}
fn all_tests() -> BTreeSet<String> {
    units().iter().map(|u| u.test.into()).collect()
}
#[test]
fn actual_instrumentation_keeps_independent_denominators_and_detects_selected_weakenings() {
    let plan = freeze(&set());
    let advanced = AdvancedPlan::freeze(&plan, PROFILE, scopes()).unwrap();
    let receipt = run_fixture(&advanced, "actual-1", &all_tests()).unwrap();
    let result = assess(&advanced, &receipt).unwrap();
    assert_eq!(result.metrics.len(), 4);
    for metric in &result.metrics {
        assert_eq!((metric.counts.numerator, metric.counts.denominator), (2, 2));
        assert_eq!(metric.counts.fraction(), Some(1.0));
        assert!(metric.units.iter().all(|u| u.covered));
        assert!(
            metric
                .traces
                .iter()
                .all(|t| t.observed && t.artifact_uri.is_some() && t.artifact_digest.is_some())
        );
    }
    assert_eq!(
        receipt.attempt().plan_digest,
        canonical_digest(&plan).unwrap()
    );
    receipt.attempt().artifacts[0]
        .verify(receipt.artifact())
        .unwrap();
    let raw: serde_json::Value = serde_json::from_slice(receipt.artifact()).unwrap();
    let mutation = raw["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| !e["mutant"].is_null())
        .collect::<Vec<_>>();
    assert_eq!(mutation.len(), 2);
    for event in mutation {
        assert_eq!(event["baseline"]["passed"], true);
        assert_eq!(event["mutant"]["passed"], false);
        assert_ne!(event["baseline"]["actual"], event["mutant"]["actual"]);
    }
    for event in raw["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["unit"].as_str().unwrap().starts_with("concurrency."))
    {
        let schedule = event["schedule"].as_array().unwrap();
        assert_eq!(schedule.len(), 2);
        assert!(schedule[0].as_str().unwrap().contains("worker-0"));
        assert!(schedule[1].as_str().unwrap().contains("worker-1"));
    }
    if let Some(path) = std::env::var_os("TESTGUARD_ADVANCED_EVIDENCE_DIR") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("raw-events.json"), receipt.artifact()).unwrap();
        std::fs::write(
            path.join("attempt.json"),
            serde_json::to_vec_pretty(receipt.attempt()).unwrap(),
        )
        .unwrap();
        std::fs::write(
            path.join("coverage.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn missing_required_environment_and_extra_passes_never_fill_advanced_units() {
    let mut input = set();
    input.environments.push("unavailable-environment".into());
    input.obligations[0]
        .environments
        .push("unavailable-environment".into());
    let plan = freeze(&input);
    let advanced = AdvancedPlan::freeze(&plan, PROFILE, scopes()).unwrap();
    let receipt = run_fixture(&advanced, "missing-env", &all_tests()).unwrap();
    let result = assess(&advanced, &receipt).unwrap();
    let contract = &result.metrics[0];
    assert_eq!(
        (contract.counts.numerator, contract.counts.denominator),
        (1, 2)
    );
    assert!(
        contract.units[0].unmapped_reason.is_some() || contract.units[1].unmapped_reason.is_some()
    );
    assert_eq!(
        result.execution.decision,
        testguard::policy::Decision::Block
    );
    assert!(
        contract
            .traces
            .iter()
            .any(|t| !t.observed && t.artifact_uri.is_none())
    );
    let plan = freeze(&set());
    let advanced = AdvancedPlan::freeze(&plan, PROFILE, scopes()).unwrap();
    let mut tests = all_tests();
    tests.remove("fixture_balance");
    let result = assess(
        &advanced,
        &run_fixture(&advanced, "missing-case", &tests).unwrap(),
    )
    .unwrap();
    assert_eq!(result.metrics[0].counts.numerator, 1);
    assert_eq!(
        result.execution.decision,
        testguard::policy::Decision::Block
    );
}
#[test]
fn exclusions_na_unmapped_and_original_required_plan_stay_separate() {
    let mut input = set();
    input.obligations.retain(|o| o.test_id != "fixture_cancel");
    let plan = freeze(&input);
    let mut config = scopes();
    for u in &mut config[0].units {
        u.excluded_reason =
            Some("controller excludes contract metric for this qualification".into());
    }
    config[3].units.push(UnitSelection {
        id: "mutation.unknown-operator".into(),
        excluded_reason: None,
    });
    let advanced = AdvancedPlan::freeze(&plan, PROFILE, config).unwrap();
    let result = assess(
        &advanced,
        &run_fixture(&advanced, "scope", &all_tests()).unwrap(),
    )
    .unwrap();
    assert_eq!(result.metrics[0].counts.denominator, 0);
    assert_eq!(result.metrics[0].counts.fraction(), None);
    assert!(result.metrics[0].not_applicable_reason.is_some());
    assert_eq!(result.metrics[0].extra.len(), 2);
    assert_eq!(
        (
            result.metrics[1].counts.numerator,
            result.metrics[1].counts.denominator
        ),
        (1, 2)
    );
    assert!(
        result.metrics[1]
            .units
            .iter()
            .any(|u| u.unmapped_reason.is_some())
    );
    assert_eq!(
        (
            result.metrics[3].counts.numerator,
            result.metrics[3].counts.denominator
        ),
        (2, 3)
    );
    let receipt = run_fixture(&advanced, "empty", &BTreeSet::new()).unwrap();
    let empty = assess(&advanced, &receipt).unwrap();
    assert_eq!(empty.execution.decision, testguard::policy::Decision::Block);
    assert_eq!(empty.metrics[0].counts.denominator, 0);
}
#[test]
fn unknown_profile_source_attempt_and_frozen_mapping_changes_fail_closed() {
    let plan = freeze(&set());
    assert!(AdvancedPlan::freeze(&plan, "future", scopes()).is_err());
    let mut config = scopes();
    config[0].units.clear();
    assert!(AdvancedPlan::freeze(&plan, PROFILE, config).is_err());
    let mut config = scopes();
    config[0].units[0].excluded_reason = Some(" ".into());
    assert!(AdvancedPlan::freeze(&plan, PROFILE, config).is_err());
    let mut raw = serde_json::to_value(&plan).unwrap();
    raw["binding"]["source_digest"] = "c".repeat(64).into();
    let changed: FrozenPlan = serde_json::from_value(raw).unwrap();
    assert!(AdvancedPlan::freeze(&changed, PROFILE, scopes()).is_err());
    let advanced = AdvancedPlan::freeze(&plan, PROFILE, scopes()).unwrap();
    assert!(run_fixture(&advanced, "../escaped", &all_tests()).is_err());
    assert!(
        run_fixture(
            &advanced,
            "unknown",
            &BTreeSet::from(["arbitrary-script".into()])
        )
        .is_err()
    );
    let receipt = run_fixture(&advanced, "bound", &all_tests()).unwrap();
    let mut config = scopes();
    config[0].filter = "different-controller-filter".into();
    let changed = AdvancedPlan::freeze(&plan, PROFILE, config).unwrap();
    assert!(assess(&changed, &receipt).is_err());
    let mut config = scopes();
    config[0].filter = "x".repeat(17 * 1024 * 1024);
    assert!(AdvancedPlan::freeze(&plan, PROFILE, config).is_err());
}

thread_local! {static TRACK_ALLOC: std::cell::Cell<bool> = const {std::cell::Cell::new(false)};static MAX_ALLOC: std::cell::Cell<usize> = const {std::cell::Cell::new(0)};}
struct ObservedAllocator;
unsafe impl std::alloc::GlobalAlloc for ObservedAllocator {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        let _ = TRACK_ALLOC.try_with(|v| {
            if v.get() {
                let _ = MAX_ALLOC.try_with(|m| m.set(m.get().max(l.size())));
            }
        });
        unsafe { std::alloc::System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, n: usize) -> *mut u8 {
        let _ = TRACK_ALLOC.try_with(|v| {
            if v.get() {
                let _ = MAX_ALLOC.try_with(|m| m.set(m.get().max(n)));
            }
        });
        unsafe { std::alloc::System.realloc(p, l, n) }
    }
}
#[global_allocator]
static ALLOCATOR: ObservedAllocator = ObservedAllocator;
#[test]
fn large_scope_is_rejected_before_owned_copy_or_hash() {
    let plan = freeze(&set());
    let mut config = scopes();
    config[0].scope = "x".repeat(17 * 1024 * 1024);
    MAX_ALLOC.set(0);
    TRACK_ALLOC.set(true);
    let result = AdvancedPlan::freeze(&plan, PROFILE, config);
    TRACK_ALLOC.set(false);
    let max = MAX_ALLOC.get();
    assert!(result.is_err());
    assert!(max < 4096, "additional single allocation {max}");
    println!("17MiB scope rejected before large clone/hash; max allocation {max}");
}
