use schemars::{JsonSchema, schema_for};
fn write<T: JsonSchema>(name: &str) {
    let mut schema = serde_json::to_value(schema_for!(T)).unwrap();
    fn constrain(value: &mut serde_json::Value) {
        if let Some(properties) = value
            .get_mut("properties")
            .and_then(serde_json::Value::as_object_mut)
        {
            if let Some(version) = properties.get_mut("schema_version") {
                version["const"] = "testguard.local/v1".into();
            }
            if let Some(capability) = properties.get_mut("capability") {
                capability["const"] = "local-fixture".into();
            }
        }
        if let Some(object) = value.as_object_mut() {
            for child in object.values_mut() {
                constrain(child);
            }
        }
        if let Some(array) = value.as_array_mut() {
            for child in array {
                constrain(child);
            }
        }
    }
    constrain(&mut schema);
    std::fs::write(
        format!("schemas/{name}.json"),
        format!("{}\n", serde_json::to_string_pretty(&schema).unwrap()),
    )
    .unwrap();
}
fn main() {
    write::<testguard::obligation::ObligationSet>("test-obligation");
    write::<testguard::plan::FrozenPlan>("test-plan");
    write::<testguard::report::AttemptRecord>("test-execution");
    write::<testguard::coverage::CoverageEvidence>("test-coverage");
    write::<testguard::report::DomainFinding>("domain-finding");
}
