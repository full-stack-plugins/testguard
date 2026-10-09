//! Bounded Playwright JSON reader for the qualified local system-browser profile.
//! Bytes and declared profiles are evidence, not authenticated execution.
use super::{ExecutorProfile, RawArtifactSet};
use crate::{
    obligation::require,
    report::{CaseObservation, CaseStatus, normalize::canonical_digest},
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Report {
    config: Config,
    suites: Vec<Suite>,
    errors: Vec<serde_json::Value>,
    stats: Stats,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    version: String,
    root_dir: String,
    config_file: String,
    argv: Vec<String>,
    projects: Vec<Project>,
    workers: usize,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Project {
    id: String,
    name: String,
    retries: usize,
    repeat_each: usize,
}
#[derive(Deserialize)]
struct Suite {
    #[serde(default)]
    suites: Vec<Suite>,
    #[serde(default)]
    specs: Vec<Spec>,
}
#[derive(Deserialize)]
struct Spec {
    id: String,
    title: String,
    file: String,
    line: usize,
    column: usize,
    ok: bool,
    tests: Vec<Test>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Test {
    expected_status: String,
    project_id: String,
    project_name: String,
    results: Vec<Outcome>,
    status: String,
}
#[derive(Deserialize)]
struct Outcome {
    status: String,
    retry: usize,
    errors: Vec<serde_json::Value>,
}
#[derive(Deserialize)]
struct Stats {
    expected: usize,
    unexpected: usize,
    skipped: usize,
    flaky: usize,
}

// Bound parser allocations before serde; strings cannot manufacture structural depth.
fn decode(input: &str) -> Result<Report, String> {
    let (mut quoted, mut escaped, mut depth, mut tokens) = (false, false, 0usize, 0usize);
    for b in input.bytes() {
        if quoted {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                quoted = false;
            }
            continue;
        }
        match b {
            b'"' => {
                quoted = true;
                tokens += 1;
            }
            b'{' | b'[' => {
                depth += 1;
                tokens += 1;
            }
            b'}' | b']' => {
                depth = depth.checked_sub(1).ok_or("unbalanced JSON")?;
            }
            b',' => tokens += 1,
            _ => {}
        }
        require(
            depth <= 64 && tokens <= 131072,
            "JSON structural budget exceeded",
        )?;
    }
    serde_json::from_str(input).map_err(|e| format!("invalid Playwright JSON: {e}"))
}
fn specs(report: &Report, discovery: bool) -> Result<Vec<&Spec>, String> {
    let c = &report.config;
    require(
        c.version == "1.62.1" && c.workers == 1 && c.projects.len() == 1,
        "unsupported native configuration",
    )?;
    let p = &c.projects[0];
    require(
        p.id == "system-chromium" && p.name == p.id && p.retries == 0 && p.repeat_each == 1,
        "unsupported native project",
    )?;
    require(
        c.argv.iter().any(|a| a == "--list") == discovery
            && !c.root_dir.is_empty()
            && !c.config_file.is_empty(),
        "invalid discovery/run configuration",
    )?;
    require(
        report.errors.is_empty() && report.stats.flaky == 0,
        "native errors or retries unsupported",
    )?;
    let mut stack: Vec<_> = report.suites.iter().collect();
    let mut all = Vec::new();
    let mut ids = BTreeSet::new();
    while let Some(s) = stack.pop() {
        stack.extend(s.suites.iter());
        for spec in &s.specs {
            require(
                all.len() < 4096 && spec.tests.len() == 1,
                "native case budget or project multiplicity",
            )?;
            require(
                !spec.id.is_empty()
                    && ids.insert(spec.id.as_str())
                    && !spec.title.is_empty()
                    && spec.line > 0
                    && spec.column > 0,
                "invalid or duplicate case identity",
            )?;
            require(
                !spec.file.is_empty()
                    && !spec.file.starts_with('/')
                    && !spec.file.contains('\\')
                    && !spec.file.split('/').any(|p| p == ".." || p.is_empty()),
                "unsafe native source path",
            )?;
            let t = &spec.tests[0];
            require(
                t.project_id == p.id
                    && t.project_name == p.name
                    && matches!(t.expected_status.as_str(), "passed" | "skipped"),
                "unsupported native test scope",
            )?;
            if discovery {
                require(
                    t.results.is_empty() && t.status == "skipped" && spec.ok,
                    "execution in discovery",
                )?;
            }
            all.push(spec);
        }
    }
    if discovery {
        require(
            report.stats.expected == 0
                && report.stats.unexpected == 0
                && report.stats.skipped == all.len(),
            "discovery counts disagree",
        )?;
    }
    Ok(all)
}

pub fn parse(
    raw: &RawArtifactSet,
    profile: &ExecutorProfile,
) -> Result<Vec<CaseObservation>, String> {
    let mut budget = super::limits::ObservationBudget::new(raw, profile)?;
    require(
        profile.tool == "playwright"
            && crate::supports_profile(&profile.tool, &profile.version, &profile.protocol)
            && profile.target == "system-chromium"
            && profile.environment == "linux-system-chromium-151.0.7922.173"
            && profile.parameters == "retries=0;repeatEach=1"
            && profile.features.is_empty(),
        "unsupported executor profile",
    )?;
    raw.artifact.validate(&raw.attempt_id)?;
    raw.artifact.verify(raw.output.as_bytes())?;
    let inventory = decode(&raw.inventory)?;
    let discovered = specs(&inventory, true)?;
    let mut cases = BTreeMap::new();
    for s in discovered {
        budget.reserve(
            s.id.len()
                .saturating_add(s.title.len())
                .saturating_add(s.file.len()),
        )?;
        let test_id = canonical_digest(&(
            &s.id,
            &s.title,
            &s.file,
            s.line,
            s.column,
            &profile.target,
            &profile.features,
            &profile.parameters,
            &profile.environment,
        ))?;
        cases.insert(
            s.id.as_str(),
            (
                s,
                CaseObservation {
                    test_id,
                    native_id: s.id.clone(),
                    parameters: profile.parameters.clone(),
                    target: profile.target.clone(),
                    features: profile.features.clone(),
                    environment: profile.environment.clone(),
                    status: CaseStatus::Unknown,
                    discovered: true,
                    started: false,
                    finished: false,
                    artifact_uri: raw.artifact.uri.clone(),
                },
            ),
        );
    }
    if raw.output.is_empty() {
        require(raw.interrupted, "missing native report")?;
        return Ok(cases.into_values().map(|(_, c)| c).collect());
    }
    let report = decode(&raw.output)?;
    let results = specs(&report, false)?;
    require(
        report.config.root_dir == inventory.config.root_dir
            && report.config.config_file == inventory.config.config_file,
        "stale report context",
    )?;
    let mut counts = [0usize; 3];
    for s in results {
        let (known, case) = cases
            .get_mut(s.id.as_str())
            .ok_or("result absent from discovery")?;
        let t = &s.tests[0];
        require(
            s.title == known.title
                && s.file == known.file
                && s.line == known.line
                && s.column == known.column
                && t.expected_status == known.tests[0].expected_status,
            "native identity drift",
        )?;
        require(t.results.len() == 1, "missing or repeated execution")?;
        let r = &t.results[0];
        require(r.retry == 0, "native retries unsupported")?;
        let (status, index) = match (
            r.status.as_str(),
            t.status.as_str(),
            t.expected_status.as_str(),
            s.ok,
            r.errors.is_empty(),
        ) {
            ("passed", "expected", "passed", true, true) => (CaseStatus::Pass, 0),
            ("failed" | "timedOut", "unexpected", "passed", false, false) => (CaseStatus::Fail, 1),
            ("skipped", "skipped", "skipped", true, true) => (CaseStatus::Skip, 2),
            _ => return Err("native outcome contradiction".into()),
        };
        counts[index] += 1;
        case.started = status != CaseStatus::Skip;
        case.finished = true;
        case.status = status;
    }
    require(
        [
            report.stats.expected,
            report.stats.unexpected,
            report.stats.skipped,
        ] == counts,
        "native counts disagree",
    )?;
    if !raw.interrupted {
        require(
            raw.exit_code == Some(if counts[1] > 0 { 1 } else { 0 }),
            "exit/report contradiction",
        )?;
    }
    Ok(cases.into_values().map(|(_, c)| c).collect())
}
