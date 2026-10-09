#![allow(dead_code)]
use serde_json::{Value, json};
pub fn obligations() -> Value {
    json!({"schema_version":"testguard.local/v1","capability":"local-fixture","approval_ref":"fixture:approval/1","revision":"rev1","baseline_digest":"a".repeat(64),"requirements":["REQ-1","REQ-2"],"sources":[{"id":"AC-1","requirement_id":"REQ-1","kind":"acceptance"},{"id":"INV-1","requirement_id":"REQ-2","kind":"invariant"}],"environments":["linux","windows"],"obligations":[{"id":"O1","source_ids":["AC-1"],"test_id":"case1","environments":["linux","windows"]},{"id":"O2","source_ids":["INV-1"],"test_id":"case2","environments":["linux","windows"]},{"id":"O3","source_ids":["AC-1"],"test_id":"case3","environments":["linux","windows"]}]})
}
pub fn binding() -> Value {
    json!({"repository":"fixture:repo","candidate":"fixture:candidate-v1","base":"fixture:base-v1","source_digest":"b".repeat(64),"policy_digest":"c".repeat(64)})
}
pub fn plan() -> testguard::plan::FrozenPlan {
    testguard::plan::FrozenPlan::freeze(
        &testguard::obligation::ObligationSet::parse(&obligations().to_string()).unwrap(),
        serde_json::from_value(binding()).unwrap(),
    )
    .unwrap()
}
pub fn attempt(plan: &testguard::plan::FrozenPlan) -> testguard::report::AttemptRecord {
    use testguard::report::*;
    let artifact = normalize::ArtifactRef::from_bytes(
        "run1",
        "output",
        b"local fixture observations; not runtime evidence",
    )
    .unwrap();
    AttemptRecord {
        schema_version: "testguard.local/v1".into(),
        attempt_id: "run1".into(),
        plan_digest: normalize::canonical_digest(plan).unwrap(),
        exit_code: Some(0),
        finished: true,
        observations: plan
            .instances()
            .iter()
            .map(|i| CaseObservation {
                test_id: i.test_id.clone(),
                native_id: i.test_id.clone(),
                parameters: String::new(),
                target: "local-fixture".into(),
                features: vec![],
                environment: i.environment.clone(),
                status: CaseStatus::Pass,
                discovered: true,
                started: true,
                finished: true,
                artifact_uri: artifact.uri.clone(),
            })
            .collect(),
        artifacts: vec![artifact],
    }
}
