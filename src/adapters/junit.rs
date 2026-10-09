use super::{ExecutorProfile, RawArtifactSet};
use crate::{
    obligation::{require, unique},
    report::{CaseObservation, CaseStatus, normalize::canonical_digest},
};
/// Experimental Surefire 3.5.2 / Gradle 8.14.3 JUnit 4 XML reader. Other dialects are unsupported.
pub fn parse(
    raw: &RawArtifactSet,
    profile: &ExecutorProfile,
) -> Result<Vec<CaseObservation>, String> {
    let mut budget = super::limits::ObservationBudget::new(raw, profile)?;
    require(
        matches!(profile.tool.as_str(), "maven-surefire" | "gradle")
            && crate::supports_profile(&profile.tool, &profile.version, &profile.protocol),
        "unsupported JUnit profile",
    )?;
    require(
        !profile.environment.is_empty() && !profile.target.is_empty() && unique(&profile.features),
        "invalid executor scope",
    )?;
    raw.artifact.validate(&raw.attempt_id)?;
    raw.artifact.verify(raw.output.as_bytes())?;
    super::limits::xml_preflight(&raw.output)?;
    require(
        !raw.interrupted,
        "interrupted XML report cannot prove completion",
    )?;
    let doc = roxmltree::Document::parse_with_options(
        &raw.output,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: super::limits::MAX_XML_NODES,
        },
    )
    .map_err(|e| e.to_string())?;
    let root = doc.root_element();
    require(
        root.tag_name().name() == "testsuite",
        "unsupported XML root",
    )?;
    require(
        doc.descendants().all(|n| n.ancestors().count() <= 64),
        "XML nesting limit",
    )?;
    let suite = root
        .attribute("name")
        .filter(|s| !s.is_empty())
        .ok_or("missing suite identity")?;
    let count = |key| -> Result<usize, String> {
        root.attribute(key)
            .ok_or_else(|| format!("missing {key}"))?
            .parse()
            .map_err(|_| format!("invalid {key}"))
    };
    let mut observations = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    let mut failures = 0;
    let mut errors = 0;
    let mut skipped = 0;
    let mut features = profile.features.clone();
    features.sort();
    for node in root
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "testcase")
    {
        let class = node
            .attribute("classname")
            .filter(|s| !s.is_empty())
            .ok_or("missing class")?;
        let name = node
            .attribute("name")
            .filter(|s| !s.is_empty())
            .ok_or("missing name")?;
        require(
            node.children().filter(|n| n.is_element()).all(|n| {
                [
                    "failure",
                    "error",
                    "skipped",
                    "system-out",
                    "system-err",
                    "properties",
                ]
                .contains(&n.tag_name().name())
            }),
            "unknown native case status element",
        )?;
        let states: Vec<_> = node
            .children()
            .filter(|n| n.is_element())
            .map(|n| n.tag_name().name())
            .filter(|n| ["failure", "error", "skipped"].contains(n))
            .collect();
        require(states.len() <= 1, "conflicting duplicate case statuses")?;
        let status = match states.first() {
            Some(&"failure") => {
                failures += 1;
                CaseStatus::Fail
            }
            Some(&"error") => {
                errors += 1;
                CaseStatus::Fail
            }
            Some(&"skipped") => {
                skipped += 1;
                CaseStatus::Skip
            }
            _ => CaseStatus::Pass,
        };
        budget.reserve(
            suite
                .len()
                .saturating_add(class.len())
                .saturating_add(name.len()),
        )?;
        let native_id = serde_json::to_string(&(suite, class, name)).map_err(|e| e.to_string())?;
        let test_id = canonical_digest(&(
            &native_id,
            &profile.target,
            &features,
            &profile.parameters,
            &profile.environment,
        ))?;
        require(
            ids.insert(test_id.clone()),
            "duplicate parameterized native identity",
        )?;
        observations.push(CaseObservation {
            test_id,
            native_id,
            parameters: profile.parameters.clone(),
            target: profile.target.clone(),
            features: features.clone(),
            environment: profile.environment.clone(),
            started: status != CaseStatus::Skip,
            status,
            discovered: true,
            finished: true,
            artifact_uri: raw.artifact.uri.clone(),
        });
    }
    require(
        count("tests")? == observations.len()
            && count("failures")? == failures
            && count("errors")? == errors
            && count("skipped")? == skipped,
        "native XML counters disagree",
    )?;
    require(
        if failures + errors > 0 {
            raw.exit_code == Some(1)
        } else {
            raw.exit_code == Some(0)
        },
        "exit/XML contradiction",
    )?;
    Ok(observations)
}
