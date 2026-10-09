//! Native Vitest 4.0.18 JSON/list reader for the fixed local Node fixture profile.
//! JSON does not attest the tool version, configuration, or execution authority.
use super::{ExecutorProfile, RawArtifactSet};
use crate::{
    obligation::require,
    report::{CaseObservation, CaseStatus, normalize::canonical_digest},
};
use serde::{Deserialize, de::DeserializeOwned};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Discovery {
    name: String,
    file: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Report {
    num_total_test_suites: usize,
    num_passed_test_suites: usize,
    num_failed_test_suites: usize,
    num_pending_test_suites: usize,
    num_total_tests: usize,
    num_passed_tests: usize,
    num_failed_tests: usize,
    num_pending_tests: usize,
    num_todo_tests: usize,
    snapshot: Snapshot,
    start_time: f64,
    success: bool,
    test_results: Vec<FileResult>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Snapshot {
    added: usize,
    failure: bool,
    files_added: usize,
    files_removed: usize,
    files_removed_list: Vec<String>,
    files_unmatched: usize,
    files_updated: usize,
    matched: usize,
    total: usize,
    unchecked: usize,
    unchecked_keys_by_file: Vec<serde_json::Value>,
    unmatched: usize,
    updated: usize,
    did_update: bool,
}
impl Snapshot {
    fn empty(&self) -> bool {
        !self.failure
            && !self.did_update
            && [
                self.added,
                self.files_added,
                self.files_removed,
                self.files_unmatched,
                self.files_updated,
                self.matched,
                self.total,
                self.unchecked,
                self.unmatched,
                self.updated,
            ]
            .into_iter()
            .all(|n| n == 0)
            && self.files_removed_list.is_empty()
            && self.unchecked_keys_by_file.is_empty()
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FileResult {
    assertion_results: Vec<Assertion>,
    start_time: f64,
    end_time: f64,
    status: String,
    message: String,
    name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Assertion {
    ancestor_titles: Vec<String>,
    full_name: String,
    status: String,
    title: String,
    duration: Option<f64>,
    failure_messages: Vec<String>,
    meta: serde_json::Value,
}
fn decode<T: DeserializeOwned>(input: &str) -> Result<T, String> {
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
    serde_json::from_str(input).map_err(|e| format!("invalid Vitest JSON: {e}"))
}
fn valid_file(file: &str) -> bool {
    file.starts_with('/')
        && file.ends_with("/cases.test.mjs")
        && !file.contains('\\')
        && !file[1..]
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
}
pub fn parse(raw: &RawArtifactSet, p: &ExecutorProfile) -> Result<Vec<CaseObservation>, String> {
    let mut budget = super::limits::ObservationBudget::new(raw, p)?;
    require(
        p.tool == "vitest"
            && crate::supports_profile(&p.tool, &p.version, &p.protocol)
            && p.target == "cases.test.mjs"
            && p.environment == "linux-node-24.19.0"
            && p.parameters == "pool=forks;retry=0;maxWorkers=1"
            && p.features.is_empty(),
        "unsupported executor profile",
    )?;
    raw.artifact.validate(&raw.attempt_id)?;
    raw.artifact.verify(raw.output.as_bytes())?;
    let inventory: Vec<Discovery> = decode(&raw.inventory)?;
    require(inventory.len() <= 4096, "discovery case budget")?;
    let mut cases = BTreeMap::new();
    let mut source = None;
    for d in &inventory {
        require(
            !d.name.is_empty() && valid_file(&d.file),
            "invalid discovery identity",
        )?;
        if let Some(s) = source {
            require(s == d.file, "multiple native source files unsupported")?;
        } else {
            source = Some(d.file.as_str());
        }
        budget.reserve(d.name.len().saturating_add(d.file.len()))?;
        let case = CaseObservation {
            test_id: canonical_digest(&(&d.name, &p.target, &p.parameters, &p.environment))?,
            native_id: d.name.clone(),
            parameters: p.parameters.clone(),
            target: p.target.clone(),
            features: vec![],
            environment: p.environment.clone(),
            status: CaseStatus::Unknown,
            discovered: true,
            started: false,
            finished: false,
            artifact_uri: raw.artifact.uri.clone(),
        };
        require(
            cases.insert(d.name.as_str(), case).is_none(),
            "duplicate native discovery identity",
        )?;
    }
    if raw.output.is_empty() {
        require(raw.interrupted, "missing native report")?;
        return Ok(cases.into_values().collect());
    }
    let report: Report = decode(&raw.output)?;
    require(
        report.snapshot.empty() && report.start_time >= 0.0 && report.num_todo_tests == 0,
        "unsupported snapshot/todo report",
    )?;
    require(
        report.test_results.len() == 1
            && report.num_total_test_suites == 1
            && report.num_pending_test_suites == 0,
        "unsupported file/suite scope",
    )?;
    let file = &report.test_results[0];
    require(
        valid_file(&file.name)
            && source.is_none_or(|s| s == file.name)
            && file.message.is_empty()
            && file.start_time >= 0.0
            && file.end_time >= file.start_time,
        "native source/time/error drift",
    )?;
    require(file.assertion_results.len() <= 4096, "result case budget")?;
    let mut seen = BTreeSet::new();
    let mut counts = [0usize; 3];
    for a in &file.assertion_results {
        require(
            seen.insert(a.full_name.as_str())
                && a.ancestor_titles.is_empty()
                && a.full_name == a.title,
            "ambiguous native test identity",
        )?;
        require(
            a.meta.as_object().is_some_and(|m| m.is_empty()) && a.duration.is_none_or(|n| n >= 0.0),
            "unsupported native metadata",
        )?;
        let case = cases
            .get_mut(a.full_name.as_str())
            .ok_or("case absent from frozen discovery")?;
        let (status, index) = match (a.status.as_str(), a.failure_messages.is_empty()) {
            ("passed", true) => (CaseStatus::Pass, 0),
            ("failed", false) => (CaseStatus::Fail, 1),
            ("skipped", true) => (CaseStatus::Skip, 2),
            _ => return Err("unknown or contradictory native outcome".into()),
        };
        require(
            index == 2 || a.duration.is_some(),
            "missing executed duration",
        )?;
        counts[index] += 1;
        case.started = status != CaseStatus::Skip;
        case.finished = true;
        case.status = status;
    }
    require(
        [
            report.num_passed_tests,
            report.num_failed_tests,
            report.num_pending_tests,
        ] == counts
            && report.num_total_tests == seen.len(),
        "native test counts disagree",
    )?;
    let failed = counts[1] > 0;
    require(
        file.status == if failed { "failed" } else { "passed" }
            && report.success != failed
            && report.num_failed_test_suites == usize::from(failed)
            && report.num_passed_test_suites == usize::from(!failed),
        "native suite verdict contradiction",
    )?;
    if !raw.interrupted {
        require(
            raw.exit_code == Some(i32::from(failed)),
            "exit/report contradiction",
        )?;
    }
    Ok(cases.into_values().collect())
}
