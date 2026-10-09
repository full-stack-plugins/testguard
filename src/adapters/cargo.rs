use super::{ExecutorProfile, RawArtifactSet};
use crate::{
    obligation::{require, unique},
    report::{CaseObservation, CaseStatus, normalize::canonical_digest},
};
use std::collections::BTreeMap;
/// Stable libtest pretty output for one separately identified test target.
/// Tool output is untrusted evidence; this parser does not authenticate execution.
pub fn parse(
    raw: &RawArtifactSet,
    profile: &ExecutorProfile,
) -> Result<Vec<CaseObservation>, String> {
    let mut budget = super::limits::ObservationBudget::new(raw, profile)?;
    require(
        profile.tool == "cargo"
            && crate::supports_profile(&profile.tool, &profile.version, &profile.protocol),
        "unsupported executor profile",
    )?;
    require(
        !profile.target.trim().is_empty()
            && !profile.environment.trim().is_empty()
            && unique(&profile.features),
        "invalid executor scope",
    )?;
    raw.artifact.validate(&raw.attempt_id)?;
    raw.artifact.verify(raw.output.as_bytes())?;
    let mut cases = BTreeMap::new();
    let mut features = profile.features.clone();
    features.sort();
    for line in raw.inventory.lines().filter(|s| !s.trim().is_empty()) {
        let native = line
            .strip_suffix(": test")
            .ok_or("malformed Cargo inventory")?;
        require(!native.is_empty(), "empty native identity")?;
        budget.reserve(native.len())?;
        let id = canonical_digest(&(
            native,
            &profile.target,
            &features,
            &profile.parameters,
            &profile.environment,
        ))?;
        let observation = CaseObservation {
            test_id: id,
            native_id: native.into(),
            parameters: profile.parameters.clone(),
            target: profile.target.clone(),
            features: features.clone(),
            environment: profile.environment.clone(),
            status: CaseStatus::Unknown,
            discovered: true,
            started: false,
            finished: false,
            artifact_uri: raw.artifact.uri.clone(),
        };
        require(
            cases.insert(native.to_owned(), observation).is_none(),
            "duplicate native inventory identity",
        )?;
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut running = None;
    let mut in_failure = false;
    let mut summary = None;
    for line in raw.output.lines() {
        if let Some(rest) = line.strip_prefix("running ")
            && !in_failure
        {
            let count = rest
                .split_whitespace()
                .next()
                .ok_or("missing running count")?
                .parse::<usize>()
                .map_err(|_| "bad running count")?;
            require(
                running.replace(count).is_none(),
                "multiple targets in artifact",
            )?;
        }
        if line == "failures:" {
            in_failure = true;
        }
        if !in_failure
            && let Some(rest) = line.strip_prefix("test ")
            && !rest.starts_with("result:")
        {
            let (name, status) = rest.rsplit_once(" ... ").ok_or("malformed native case")?;
            require(seen.insert(name.to_owned()), "duplicate native result")?;
            let case = cases
                .get_mut(name)
                .ok_or("case absent from frozen discovery")?;
            if raw.interrupted && status.is_empty() {
                case.started = true;
                continue;
            }
            case.status = match status {
                "ok" => CaseStatus::Pass,
                "FAILED" => CaseStatus::Fail,
                "ignored" => CaseStatus::Skip,
                _ if status.starts_with("ignored, ") => CaseStatus::Skip,
                _ => return Err("unknown native case status".into()),
            };
            case.started = case.status != CaseStatus::Skip;
            case.finished = true;
        }
        if let Some(rest) = line.strip_prefix("test result: ") {
            require(summary.is_none(), "duplicate native summary")?;
            let (verdict, counts) = rest.split_once(". ").ok_or("malformed native summary")?;
            require(
                verdict == "ok" || verdict == "FAILED",
                "unknown summary result",
            )?;
            let fields: Vec<_> = counts.split("; ").collect();
            require(fields.len() == 6, "malformed native summary counts")?;
            let mut nums = Vec::new();
            for (field, suffix) in fields[..5].iter().zip([
                " passed",
                " failed",
                " ignored",
                " measured",
                " filtered out",
            ]) {
                nums.push(
                    field
                        .strip_suffix(suffix)
                        .ok_or("malformed count label")?
                        .parse::<usize>()
                        .map_err(|_| "malformed count")?,
                );
            }
            require(nums[3] == 0, "benchmarks unsupported")?;
            summary = Some((verdict.to_owned(), nums));
        }
    }
    require(
        running.is_some() || raw.interrupted,
        "missing native report",
    )?;
    if let Some(count) = running {
        require(count == cases.len(), "inventory/run scope mismatch")?;
    }
    if !raw.interrupted {
        let (verdict, counts) = summary.ok_or("missing native end summary")?;
        require(
            seen.len() == cases.len(),
            "partial result with successful completion",
        )?;
        for (n, status) in [CaseStatus::Pass, CaseStatus::Fail, CaseStatus::Skip]
            .iter()
            .enumerate()
        {
            require(
                counts[n] == cases.values().filter(|o| &o.status == status).count(),
                "native counts disagree",
            )?;
        }
        let failed = counts[1] > 0;
        require(
            (failed && verdict == "FAILED" && raw.exit_code == Some(101))
                || (!failed && verdict == "ok" && raw.exit_code == Some(0)),
            "exit/report contradiction",
        )?;
    }
    Ok(cases.into_values().collect())
}
