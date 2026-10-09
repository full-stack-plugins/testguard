mod common;
use testguard::{
    obligation::ObligationSet,
    plan::{Binding, FrozenPlan},
};
#[test]
fn freezes_six_instances_and_does_not_follow_candidate_edits() {
    let mut source = ObligationSet::parse(&common::obligations().to_string()).unwrap();
    let binding: Binding = serde_json::from_value(common::binding()).unwrap();
    let plan = FrozenPlan::freeze(&source, binding).unwrap();
    source.obligations.clear();
    assert_eq!(plan.instances().len(), 6);
    assert_eq!(plan.obligations().obligations.len(), 3);
    let saved = serde_json::to_string(&plan).unwrap();
    assert!(FrozenPlan::parse(&saved).is_ok());
    let mut tampered: serde_json::Value = serde_json::from_str(&saved).unwrap();
    tampered["instances"].as_array_mut().unwrap().pop();
    assert!(FrozenPlan::parse(&tampered.to_string()).is_err());
}
#[test]
fn rejects_missing_binding_and_unknown_plan_fields() {
    let source = ObligationSet::parse(&common::obligations().to_string()).unwrap();
    let mut binding: Binding = serde_json::from_value(common::binding()).unwrap();
    binding.candidate.clear();
    assert!(FrozenPlan::freeze(&source, binding).is_err());
    assert!(FrozenPlan::parse("{}").is_err());
}
#[test]
fn assessment_revalidates_deserialized_plan_instead_of_trusting_its_matrix() {
    let mut value = serde_json::to_value(common::plan()).unwrap();
    value["instances"].as_array_mut().unwrap().pop();
    let forged: FrozenPlan = serde_json::from_value(value).unwrap();
    let a = common::attempt(&forged);
    assert!(testguard::coverage::assess(&forged, &a, &[]).is_err());
}
