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

#[test]
fn store_admission_precedes_forged_plan_matrix_cloning() {
    use testguard::report::{
        normalize::{ArtifactRef, canonical_digest},
        store::*,
    };
    let mut wire = serde_json::to_value(common::plan()).unwrap();
    wire["obligations"]["obligations"][0]["test_id"] = "x".repeat(128 * 1024).into();
    let environments: Vec<_> = (0..64).map(|i| format!("env{i}")).collect();
    let mut all = environments.clone();
    all.extend(["linux".into(), "windows".into()]);
    wire["obligations"]["environments"] = serde_json::json!(all);
    wire["obligations"]["obligations"][0]["environments"] = serde_json::json!(environments);
    wire["instances"] = serde_json::json!([]);
    let plan = serde_json::from_value(wire).unwrap();
    let mut attempt = common::attempt(&common::plan());
    attempt.plan_digest = canonical_digest(&plan).unwrap();
    attempt.observations.clear();
    let reference = ArtifactRef::from_bytes("run1", "native", b"report").unwrap();
    attempt.artifacts = vec![reference.clone()];
    let root = tempfile::tempdir().unwrap();
    let (s, admin) = EvidenceStore::open(
        root.path(),
        StorePolicy {
            id: "policy".into(),
            retention_seconds: 100,
            evidence_seconds: 50,
            max_attempts: 4,
            max_total_bytes: 32 * 1024 * 1024,
        },
        vec![],
    )
    .unwrap();
    let (a, _) = s.grant(&admin, "project").unwrap();
    let provenance = Provenance {
        execution_source: "fixture".into(),
        tool: "cargo".into(),
        tool_version: "1.99.0".into(),
        analyzer: "fixture".into(),
        analyzer_version: "1".into(),
        config_digest: "a".repeat(64),
        parent_attempt: None,
        command: vec!["cargo".into()],
        environment: Default::default(),
        approval_refs: vec![],
    };
    ALLOCATED.store(0, SeqCst);
    MEASURE.store(true, SeqCst);
    let result = s.append(
        &a,
        AppendInput {
            plan: &plan,
            attempt: &attempt,
            provenance: &provenance,
            artifacts: &[InputArtifact {
                reference: &reference,
                bytes: b"report",
                kind: ArtifactKind::Evidence,
            }],
        },
        1000,
    );
    MEASURE.store(false, SeqCst);
    let allocated = ALLOCATED.load(SeqCst);
    assert!(
        allocated < 256 * 1024,
        "pre-admission allocation {allocated}"
    );
    assert!(result.unwrap_err().contains("domain assessment budget"));
}
