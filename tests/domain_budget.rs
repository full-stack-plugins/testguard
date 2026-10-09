mod common;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
};

struct Meter;
static MEASURE: AtomicBool = AtomicBool::new(false);
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && MEASURE.load(SeqCst) {
            ALLOCATED.fetch_add(layout.size(), SeqCst);
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Meter = Meter;

// A separate integration binary with one test keeps allocation measurements isolated.
// Moving preflight after validation/assessment/fact cloning must fail this regression.
#[test]
fn oversized_domain_is_rejected_before_cloning_or_rebuilding_the_matrix() {
    use testguard::{
        coverage::assess,
        report::{
            AttemptRecord,
            engine_adapter::{CAPABILITY, project},
            normalize::canonical_digest,
        },
    };
    let mut source =
        testguard::obligation::ObligationSet::parse(&common::obligations().to_string()).unwrap();
    source.obligations.truncate(1);
    source.environments = (0..64).map(|i| format!("env{i}")).collect();
    source.obligations[0].environments = source.environments.clone();
    source.obligations[0].test_id = "x".repeat(128 * 1024);
    let plan =
        testguard::plan::FrozenPlan::freeze(&source, common::engine_plan().binding().clone())
            .unwrap();
    let attempt = AttemptRecord {
        schema_version: "testguard.local/v1".into(),
        attempt_id: "run1".into(),
        plan_digest: canonical_digest(&plan).unwrap(),
        exit_code: Some(0),
        finished: true,
        observations: vec![],
        artifacts: vec![],
    };
    ALLOCATED.store(0, SeqCst);
    MEASURE.store(true, SeqCst);
    let result = project(&plan, &attempt, &[], &[], CAPABILITY);
    MEASURE.store(false, SeqCst);
    let allocated = ALLOCATED.load(SeqCst);
    assert!(
        allocated < 256 * 1024,
        "allocated {allocated} bytes before rejection"
    );
    assert!(result.err().unwrap().contains("domain assessment budget"));
    assert!(
        assess(&plan, &attempt, &[])
            .unwrap_err()
            .contains("domain assessment budget")
    );

    // A forged small serialized matrix must not hide the large source expansion.
    let mut wire = serde_json::to_value(&plan).unwrap();
    wire["instances"] = serde_json::json!([]);
    let forged = serde_json::from_value(wire).unwrap();
    ALLOCATED.store(0, SeqCst);
    MEASURE.store(true, SeqCst);
    let result = assess(&forged, &attempt, &[]);
    MEASURE.store(false, SeqCst);
    assert!(ALLOCATED.load(SeqCst) < 256 * 1024);
    assert!(result.unwrap_err().contains("domain assessment budget"));

    // Count-based work limit also rejects a small-byte matrix with quadratic matching cost.
    source.obligations[0].test_id = "case".into();
    source.environments = (0..1024).map(|i| format!("env{i}")).collect();
    source.obligations[0].environments = source.environments.clone();
    let wide =
        testguard::plan::FrozenPlan::freeze(&source, common::engine_plan().binding().clone())
            .unwrap();
    assert!(
        assess(&wide, &attempt, &[])
            .unwrap_err()
            .contains("domain assessment budget")
    );

    // Serialized escaping counts toward admission before any advice string is copied.
    let normal = common::engine_plan();
    let normal_attempt = common::attempt(&normal);
    let advice = vec!["\0".repeat(2 * 1024 * 1024)];
    ALLOCATED.store(0, SeqCst);
    MEASURE.store(true, SeqCst);
    let result = project(&normal, &normal_attempt, &[], &advice, CAPABILITY);
    MEASURE.store(false, SeqCst);
    assert!(ALLOCATED.load(SeqCst) < 256 * 1024);
    assert!(result.err().unwrap().contains("domain assessment budget"));
    let failure = testguard::report::transport::prepare(&normal, common::invocation(&normal))
        .unwrap()
        .complete(&normal_attempt, &[], &advice)
        .unwrap();
    assert_eq!(failure.exit_code(), 4);
    assert_eq!(
        failure.envelope.run_status,
        guardengine::integration::RunStatus::Error
    );
    assert_eq!(failure.envelope.decision, None);
    assert!(failure.envelope.artifacts.report.is_none());
    assert_eq!(failure.envelope.coverage.required_scopes.len(), 6);
    assert!(testguard::report::transport::prepare(&plan, common::invocation(&plan)).is_err());
}
