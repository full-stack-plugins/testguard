use crate::plan::{Binding, FrozenPlan};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseMapping {
    pub obligation_id: String,
    pub test_id: String,
    pub environments: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRef {
    pub path: String,
    pub line: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub namespace: String,
    pub id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceStatus {
    pub path: String,
    pub status: String,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceEdge {
    pub from: Identity,
    pub to: Identity,
    pub relation: String,
    pub source: SourceRef,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExportObligation {
    pub id: String,
    pub requirement: Identity,
    pub acceptance: Identity,
    pub source: SourceRef,
    pub text_digest: String,
    pub links: Vec<TraceEdge>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Export {
    pub api_version: String,
    pub kind: String,
    pub baseline_digest: String,
    pub source_digest: String,
    pub candidate_oid: String,
    pub scope: Vec<Identity>,
    pub sources: Vec<SourceStatus>,
    pub complete: bool,
    pub authentication_profile: String,
    pub obligations: Vec<ExportObligation>,
}
pub struct ImportedFixture {
    pub source: Export,
    pub plan: FrozenPlan,
}
pub fn import_fixture(
    input: &str,
    mappings: &[CaseMapping],
    binding: Binding,
) -> Result<ImportedFixture, String> {
    use super::{Obligation, ObligationSet, Source, SourceKind, VERSION, digest, require, unique};
    let source: Export = serde_json::from_str(input).map_err(|e| e.to_string())?;
    require(
        source.api_version == "specguard.domain/v1alpha1"
            && source.kind == "TestObligationSet"
            && source.authentication_profile == "fixture-only",
        "unsupported export version/kind/authentication capability",
    )?;
    require(
        source.complete
            && !source.scope.is_empty()
            && source
                .scope
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == source.scope.len(),
        "incomplete or duplicate scope",
    )?;
    let raw_digest = |s: &str| {
        s.strip_prefix("sha256:")
            .filter(|s| digest(s))
            .map(str::to_owned)
            .ok_or_else(|| "invalid export digest".to_owned())
    };
    let baseline_digest = raw_digest(&source.baseline_digest)?;
    require(
        raw_digest(&source.source_digest)? == binding.source_digest
            && source.candidate_oid == binding.candidate,
        "export binding drift",
    )?;
    require(
        !source.sources.is_empty()
            && source
                .sources
                .iter()
                .all(|s| s.status == "complete" && !s.path.is_empty()),
        "incomplete source status",
    )?;
    let identity = |id: &Identity| -> Result<String, String> {
        require(
            !id.namespace.is_empty() && !id.id.is_empty(),
            "empty namespaced identity",
        )?;
        serde_json::to_string(id).map_err(|e| e.to_string())
    };
    let requirements: Vec<_> = source
        .scope
        .iter()
        .map(identity)
        .collect::<Result<_, _>>()?;
    require(
        unique(
            &mappings
                .iter()
                .map(|m| m.obligation_id.clone())
                .collect::<Vec<_>>(),
        ) && mappings.len() == source.obligations.len(),
        "missing or duplicate external case mapping",
    )?;
    let mut obligations = Vec::new();
    let mut sources = std::collections::BTreeMap::new();
    let mut environments = std::collections::BTreeSet::new();
    for o in &source.obligations {
        require(
            source.scope.contains(&o.requirement)
                && o.source.line > 0
                && source.sources.iter().any(|s| s.path == o.source.path),
            "dangling export requirement/source",
        )?;
        raw_digest(&o.text_digest)?;
        for link in &o.links {
            identity(&link.from)?;
            identity(&link.to)?;
            require(
                ["depends_on", "traces_to_adr", "traces_to_task"].contains(&link.relation.as_str())
                    && link.source.line > 0
                    && !link.source.path.is_empty(),
                "unsupported or invalid trace edge",
            )?;
        }
        let source_id = identity(&o.acceptance)?;
        let requirement_id = identity(&o.requirement)?;
        if let Some(existing) = sources.get(&source_id) {
            let existing: &Source = existing;
            require(
                existing.requirement_id == requirement_id,
                "acceptance identity collision",
            )?;
        }
        sources.insert(
            source_id.clone(),
            Source {
                id: source_id.clone(),
                requirement_id,
                kind: SourceKind::Acceptance,
            },
        );
        let mapping = mappings
            .iter()
            .find(|m| m.obligation_id == o.id)
            .ok_or("missing external case mapping")?;
        environments.extend(mapping.environments.iter().cloned());
        obligations.push(Obligation {
            id: o.id.clone(),
            source_ids: vec![source_id],
            test_id: mapping.test_id.clone(),
            environments: mapping.environments.clone(),
        });
    }
    let local = ObligationSet {
        schema_version: VERSION.into(),
        capability: "local-fixture".into(),
        approval_ref: "fixture-only:specguard-export".into(),
        revision: source.baseline_digest.clone(),
        baseline_digest,
        requirements,
        sources: sources.into_values().collect(),
        environments: environments.into_iter().collect(),
        obligations,
    };
    let plan = FrozenPlan::freeze(&local, binding)?;
    Ok(ImportedFixture { source, plan })
}
