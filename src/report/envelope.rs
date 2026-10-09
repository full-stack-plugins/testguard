use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureBundle {
    pub capability: String,
    pub envelope: guardengine::integration::GuardRunEnvelope,
    pub artifacts: BTreeMap<String, Vec<u8>>,
}
impl FixtureBundle {
    pub fn exit_code(&self) -> i32 {
        if self.envelope.run_status != guardengine::integration::RunStatus::Completed {
            return 4;
        }
        match self.envelope.decision {
            Some(guardengine::Decision::Allow) => 0,
            Some(guardengine::Decision::Block) => 2,
            Some(guardengine::Decision::RequireApproval) => 3,
            None => 4,
        }
    }
}
pub(crate) fn store(
    artifacts: &mut BTreeMap<String, Vec<u8>>,
    run_id: &str,
    name: &str,
    bytes: Vec<u8>,
    media_type: &str,
) -> guardengine::integration::ArtifactRef {
    let uri = format!(
        "artifact://testguard/{}/{name}",
        super::normalize::bytes_digest(run_id.as_bytes())
    );
    let reference = guardengine::integration::ArtifactRef {
        uri: uri.clone(),
        digest: format!("sha256:{}", super::normalize::bytes_digest(&bytes)),
        media_type: media_type.into(),
    };
    artifacts.insert(uri, bytes);
    reference
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectionInput {
    pub attempt: crate::report::AttemptRecord,
    pub changes: Vec<crate::policy::Weakening>,
    pub advice: Vec<String>,
}
pub fn verify_bundle(
    plan: &crate::plan::FrozenPlan,
    bundle: &FixtureBundle,
) -> Result<guardengine::GuardReport, String> {
    use super::engine_adapter::{CAPABILITY, project};
    use crate::obligation::require;
    use guardengine::integration::{EvidenceProfile, verify_engine_artifacts};
    require(
        bundle.capability == CAPABILITY,
        "unsupported bundle capability",
    )?;
    bundle
        .envelope
        .validate(EvidenceProfile::EngineBacked)
        .map_err(|e| e.to_string())?;
    require(
        bundle.envelope.producer == super::transport::producer(),
        "unsupported producer mapping",
    )?;
    require(
        bundle.envelope.approval_refs.is_empty(),
        "local fixture cannot assert approvals",
    )?;
    let invocation = super::transport::FixtureInvocation {
        capability: bundle.capability.clone(),
        run_id: bundle.envelope.run_id.clone(),
        binding: bundle.envelope.binding.clone(),
        started_at: bundle.envelope.started_at.clone(),
        finished_at: bundle.envelope.finished_at.clone(),
    };
    super::transport::prepare(plan, invocation).map_err(|e| e.message)?;
    let all_refs: Vec<_> = bundle
        .envelope
        .artifacts
        .contract
        .iter()
        .chain(bundle.envelope.artifacts.facts.iter())
        .chain(bundle.envelope.artifacts.report.iter())
        .chain(bundle.envelope.artifacts.domain.iter())
        .collect();
    let mut uris = std::collections::BTreeSet::new();
    for reference in &all_refs {
        require(uris.insert(&reference.uri), "duplicate artifact reference")?;
        let bytes = bundle
            .artifacts
            .get(&reference.uri)
            .ok_or("missing referenced bytes")?;
        require(
            bytes.len() <= guardengine::integration::MAX_ARTIFACT_BYTES
                && reference.digest == format!("sha256:{}", super::normalize::bytes_digest(bytes)),
            "artifact byte digest/size mismatch",
        )?;
    }
    require(
        uris.len() == bundle.artifacts.len(),
        "unreferenced bundle artifacts",
    )?;
    let bytes =
        |reference: &Option<guardengine::integration::ArtifactRef>| -> Result<&[u8], String> {
            let reference = reference.as_ref().ok_or("missing engine reference")?;
            bundle
                .artifacts
                .get(&reference.uri)
                .map(Vec::as_slice)
                .ok_or_else(|| "missing engine bytes".into())
        };
    let cb = bytes(&bundle.envelope.artifacts.contract)?;
    let fb = bytes(&bundle.envelope.artifacts.facts)?;
    let rb = bytes(&bundle.envelope.artifacts.report)?;
    let report =
        verify_engine_artifacts(&bundle.envelope, cb, fb, rb).map_err(|e| e.to_string())?;
    let domain_bytes = |name: &str| -> Result<&[u8], String> {
        let uri = format!(
            "artifact://testguard/{}/{name}",
            super::normalize::bytes_digest(bundle.envelope.run_id.as_bytes())
        );
        require(
            bundle
                .envelope
                .artifacts
                .domain
                .iter()
                .any(|r| r.uri == uri),
            "missing domain reference",
        )?;
        bundle
            .artifacts
            .get(&uri)
            .map(Vec::as_slice)
            .ok_or_else(|| "missing domain bytes".into())
    };
    let input: ProjectionInput =
        serde_json::from_slice(domain_bytes("inputs.json")?).map_err(|e| e.to_string())?;
    require(
        input.attempt.attempt_id == bundle.envelope.run_id,
        "attempt identity mismatch",
    )?;
    require(
        domain_bytes("plan.json")? == serde_json::to_vec(plan).map_err(|e| e.to_string())?,
        "frozen plan bytes mismatch",
    )?;
    let projected = project(
        plan,
        &input.attempt,
        &input.changes,
        &input.advice,
        CAPABILITY,
    )?;
    require(
        cb == projected.contract_bytes
            && fb == projected.facts_bytes
            && rb == projected.report_bytes
            && domain_bytes("domain.json")? == projected.domain_bytes
            && bundle.envelope.coverage == projected.coverage,
        "bundle differs from domain recomputation",
    )?;
    Ok(report)
}
