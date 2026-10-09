mod common;
use testguard::report::engine_adapter::{CAPABILITY, contract, project};
#[test]
fn contract_is_available_before_observations_and_preserves_legacy_bytes() {
    let plan = common::plan();
    let bytes = serde_json::to_vec(&contract(&plan).unwrap()).unwrap();
    // Captured from the unchanged project() at source 9abc6dc before extraction.
    assert_eq!(
        testguard::report::normalize::bytes_digest(&bytes),
        "e894ac59f516f3d2a83875fd6dad0a0256bf8c1acf0b4c9614aedb822025a634"
    );
    assert_eq!(
        bytes,
        serde_json::to_vec(&contract(&plan).unwrap()).unwrap()
    );
}
#[test]
fn completed_and_partial_projections_keep_the_prepared_contract_bytes() {
    let plan = common::plan();
    let expected = serde_json::to_vec(&contract(&plan).unwrap()).unwrap();
    for state in 0..5 {
        let mut attempt = common::attempt(&plan);
        let mut changes = vec![];
        let mut advice = vec![];
        match state {
            1 => {
                attempt.observations[0].status = testguard::report::CaseStatus::Fail;
                attempt.exit_code = Some(101);
            }
            2 => {
                attempt.observations.pop();
            }
            3 => changes.push(testguard::policy::Weakening::FilterChanged),
            4 => advice.push("fixture advisory".into()),
            _ => {}
        }
        assert_eq!(
            project(&plan, &attempt, &changes, &advice, CAPABILITY)
                .unwrap()
                .contract_bytes,
            expected
        );
    }
}
