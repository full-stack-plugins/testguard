//! Reader for official Jest 30.2.0 distribution (whose CLI displays 30.1.3).
//! Native listTests discovers files only; required cases are separately protected.
use super::{ExecutorProfile, RawArtifactSet};
use crate::{
    obligation::require,
    report::{CaseObservation, CaseStatus, normalize::canonical_digest},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: String,
    distribution_version: String,
    cli_version: String,
    core_digest: String,
    target: String,
    required_names: Vec<String>,
    source_digest: String,
    config_digest: String,
}
/// Controller-owned expectations. Freeze before native execution; this is not an
/// authentication provider and declared source/tool digests are not attestations.
pub struct PreparedCases {
    manifest: Manifest,
    manifest_digest: String,
    native_file: String,
    profile_digest: String,
    required_ids: Vec<String>,
}
fn profile_ok(p: &ExecutorProfile) -> bool {
    p.tool == "jest"
        && crate::supports_profile(&p.tool, &p.version, &p.protocol)
        && p.target == "cases.test.cjs"
        && p.environment == "linux-node-24.19.0"
        && p.parameters == "runInBand;retry=0;cli=30.1.3"
        && p.features.is_empty()
}
fn digest_ok(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl PreparedCases {
    /// Freeze the required plan from these controller IDs, never from actual results.
    pub fn required_test_ids(&self) -> impl Iterator<Item = &str> {
        self.required_ids.iter().map(String::as_str)
    }
    pub fn freeze(
        input: &str,
        native_file: &str,
        profile: &ExecutorProfile,
    ) -> Result<Self, String> {
        require(
            input.len() <= 1024 * 1024
                && native_file.len() <= 4096
                && valid_file(native_file)
                && profile_ok(profile),
            "invalid protected scope or input budget",
        )?;
        let manifest: Manifest = decode(input)?;
        require(
            manifest.schema_version == "testguard.jest-fixture/v1"
                && manifest.distribution_version == "30.2.0"
                && manifest.cli_version == "30.1.3"
                && manifest.target == profile.target
                && [
                    &manifest.source_digest,
                    &manifest.config_digest,
                    &manifest.core_digest,
                ]
                .into_iter()
                .all(|s| digest_ok(s)),
            "unsupported protected manifest",
        )?;
        require(
            !manifest.required_names.is_empty() && manifest.required_names.len() <= 4096,
            "required case count budget",
        )?;
        let mut seen = BTreeSet::new();
        let mut expansion = 0usize;
        for name in &manifest.required_names {
            expansion = expansion.saturating_add(
                name.len()
                    .saturating_add(native_file.len())
                    .saturating_add(1024)
                    .saturating_mul(6),
            );
            require(
                !name.is_empty()
                    && name.len() <= 16 * 1024
                    && seen.insert(name.as_str())
                    && expansion <= 8 * 1024 * 1024,
                "ambiguous or excessive required identities",
            )?;
        }
        let manifest_digest = canonical_digest(&manifest)?;
        let profile_digest = canonical_digest(profile)?;
        let required_ids = manifest
            .required_names
            .iter()
            .map(|name| canonical_digest(&(&manifest_digest, name, &profile_digest)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            required_ids,
            manifest,
            manifest_digest,
            native_file: native_file.into(),
            profile_digest,
        })
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Report {
    num_total_test_suites: usize,
    num_passed_test_suites: usize,
    num_failed_test_suites: usize,
    num_pending_test_suites: usize,
    num_runtime_error_test_suites: usize,
    num_total_tests: usize,
    num_passed_tests: usize,
    num_failed_tests: usize,
    num_pending_tests: usize,
    num_todo_tests: usize,
    open_handles: Vec<serde_json::Value>,
    snapshot: Snapshot,
    start_time: u64,
    success: bool,
    test_results: Vec<FileResult>,
    was_interrupted: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FileResult {
    assertion_results: Vec<Assertion>,
    start_time: u64,
    end_time: u64,
    status: String,
    message: String,
    name: String,
    summary: String,
    coverage: Option<serde_json::Value>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Assertion {
    ancestor_titles: Vec<String>,
    full_name: String,
    status: String,
    title: String,
    duration: Option<u64>,
    failure_messages: Vec<String>,
    failure_details: Vec<serde_json::Value>,
    failing: bool,
    invocations: usize,
    location: Option<serde_json::Value>,
    num_passing_asserts: usize,
    retry_reasons: Vec<serde_json::Value>,
    start_at: u64,
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
    serde_json::from_str(input).map_err(|e| format!("invalid Jest JSON: {e}"))
}
fn valid_file(file: &str) -> bool {
    file.starts_with('/')
        && file.ends_with("/cases.test.cjs")
        && !file.contains('\\')
        && !file[1..]
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
}
pub fn parse(
    raw: &RawArtifactSet,
    p: &ExecutorProfile,
    prepared: &PreparedCases,
) -> Result<Vec<CaseObservation>, String> {
    let mut budget = super::limits::ObservationBudget::new(raw, p)?;
    require(
        profile_ok(p) && canonical_digest(p)? == prepared.profile_digest,
        "protected executor scope mismatch",
    )?;
    raw.artifact.validate(&raw.attempt_id)?;
    raw.artifact.verify(raw.output.as_bytes())?;
    let files: Vec<String> = decode(&raw.inventory)?;
    require(
        files.len() == 1 && files[0] == prepared.native_file,
        "native file discovery disagrees with protected scope",
    )?;
    let mut cases = BTreeMap::new();
    for name in &prepared.manifest.required_names {
        budget.reserve(name.len().saturating_add(prepared.manifest_digest.len()))?;
        cases.insert(
            name.as_str(),
            CaseObservation {
                test_id: canonical_digest(&(
                    &prepared.manifest_digest,
                    name,
                    &prepared.profile_digest,
                ))?,
                native_id: name.clone(),
                parameters: p.parameters.clone(),
                target: p.target.clone(),
                features: vec![],
                environment: p.environment.clone(),
                status: CaseStatus::Unknown,
                discovered: true,
                started: false,
                finished: false,
                artifact_uri: raw.artifact.uri.clone(),
            },
        );
    }
    if raw.output.is_empty() {
        require(raw.interrupted, "missing native report")?;
        return Ok(Vec::new());
    }
    let r: Report = decode(&raw.output)?;
    require(
        r.snapshot.empty()
            && r.open_handles.is_empty()
            && !r.was_interrupted
            && r.num_todo_tests == 0,
        "unsupported snapshot/handle/interrupted report",
    )?;
    require(
        r.test_results.len() == 1
            && r.num_total_test_suites == 1
            && r.num_runtime_error_test_suites <= 1,
        "unsupported suite scope",
    )?;
    let f = &r.test_results[0];
    require(
        f.name == prepared.native_file
            && f.start_time >= r.start_time
            && f.end_time >= f.start_time
            && f.summary.is_empty()
            && f.coverage
                .as_ref()
                .is_none_or(|v| v.as_object().is_some_and(|m| m.is_empty())),
        "native source/time/coverage drift",
    )?;
    require(f.assertion_results.len() <= 4096, "result case budget")?;
    let mut seen = BTreeSet::new();
    let mut counts = [0usize; 3];
    let mut last_start = r.start_time;
    for a in &f.assertion_results {
        require(
            seen.insert(a.full_name.as_str())
                && a.ancestor_titles.is_empty()
                && a.full_name == a.title
                && !a.failing
                && a.invocations == 1
                && a.location.is_none()
                && a.retry_reasons.is_empty(),
            "unsupported or ambiguous native case",
        )?;
        require(
            a.start_at >= last_start && a.start_at <= f.end_time,
            "native case time contradiction",
        )?;
        last_start = a.start_at;
        let case = cases
            .get_mut(a.full_name.as_str())
            .ok_or("case absent from protected required manifest")?;
        let (status, index) = match (
            a.status.as_str(),
            a.failure_messages.is_empty(),
            a.failure_details.is_empty(),
        ) {
            ("passed", true, true) => (CaseStatus::Pass, 0),
            ("failed", false, false) => (CaseStatus::Fail, 1),
            ("pending", true, true) => (CaseStatus::Skip, 2),
            _ => return Err("unknown or contradictory native outcome".into()),
        };
        if index == 2 {
            require(
                a.duration.is_none() && a.num_passing_asserts == 0,
                "skipped test claims execution",
            )?;
        } else {
            let duration = a.duration.ok_or("missing executed duration")?;
            require(
                a.start_at >= f.start_time
                    && a.start_at
                        .checked_add(duration)
                        .is_some_and(|end| end <= f.end_time),
                "executed duration outside suite",
            )?;
        }
        counts[index] += 1;
        case.started = status != CaseStatus::Skip;
        case.finished = true;
        case.status = status;
    }
    require(
        [r.num_passed_tests, r.num_failed_tests, r.num_pending_tests] == counts
            && r.num_total_tests == seen.len(),
        "native test counts disagree",
    )?;
    let runtime_error = r.num_runtime_error_test_suites == 1;
    if runtime_error {
        require(
            seen.is_empty(),
            "runtime error with unsupported case evidence",
        )?;
    }
    let failed = counts[1] > 0 || runtime_error;
    let skipped = !failed && counts[0] == 0 && counts[2] > 0;
    let suite_status = if failed {
        "failed"
    } else if skipped {
        "skipped"
    } else if counts[2] > 0 {
        "focused"
    } else {
        "passed"
    };
    require(
        f.status == suite_status
            && r.success != failed
            && f.message.is_empty() != failed
            && r.num_failed_test_suites == usize::from(failed)
            && r.num_pending_test_suites == usize::from(skipped)
            && r.num_passed_test_suites == usize::from(!failed && !skipped),
        "native suite verdict contradiction",
    )?;
    if !raw.interrupted {
        require(
            raw.exit_code == Some(i32::from(failed)),
            "exit/report contradiction",
        )?;
    }
    Ok(cases.into_values().filter(|case| case.finished).collect())
}
