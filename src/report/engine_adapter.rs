use crate::{
    coverage::assess,
    obligation::require,
    plan::FrozenPlan,
    policy::Weakening,
    report::{AttemptRecord, CaseStatus},
};
use guardengine::{
    integration::{Coverage, CoverageStatus, evaluate_bounded},
    *,
};
pub const CAPABILITY: &str = "testguard.engine-fixture/v1";
pub const MAPPING_VERSION: &str = "testguard.engine-map/v1";
pub const ANALYZER_ID: &str = "testguard-local-evidence";
pub struct EngineProjection {
    pub contract_bytes: Vec<u8>,
    pub facts_bytes: Vec<u8>,
    pub report_bytes: Vec<u8>,
    pub domain_bytes: Vec<u8>,
    pub report: GuardReport,
    pub coverage: Coverage,
}
fn json<T: serde::Serialize>(v: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(v).map_err(|e| e.to_string())
}
/// Frozen scope identities describe distinct required tests in their environments, not discovery output.
pub fn required_scopes(plan: &FrozenPlan) -> Result<Vec<String>, String> {
    plan.validate()?;
    let scopes: std::collections::BTreeSet<_> = plan
        .instances()
        .iter()
        .map(|i| {
            crate::report::normalize::canonical_digest(&(&i.test_id, &i.environment))
                .map(|d| format!("testguard:test-environment:{d}"))
        })
        .collect::<Result<_, _>>()?;
    require(!scopes.is_empty(), "required execution coverage is empty")?;
    Ok(scopes.into_iter().collect())
}
pub fn observed_coverage(plan: &FrozenPlan, attempt: &AttemptRecord) -> Result<Coverage, String> {
    plan.validate()?;
    attempt.validate()?;
    require(
        attempt.plan_digest == crate::report::normalize::canonical_digest(plan)?,
        "attempt plan drift",
    )?;
    let required = required_scopes(plan)?;
    let observed: std::collections::BTreeSet<_> = attempt
        .observations
        .iter()
        .filter(|o| {
            o.discovered
                && o.started
                && o.finished
                && matches!(o.status, CaseStatus::Pass | CaseStatus::Fail)
        })
        .map(|o| {
            crate::report::normalize::canonical_digest(&(&o.test_id, &o.environment))
                .map(|d| format!("testguard:test-environment:{d}"))
        })
        .collect::<Result<_, _>>()?;
    let observed_scopes: Vec<_> = required
        .iter()
        .filter(|s| observed.contains(*s))
        .cloned()
        .collect();
    let missing_scopes: Vec<_> = required
        .iter()
        .filter(|s| !observed.contains(*s))
        .cloned()
        .collect();
    Ok(Coverage {
        status: if missing_scopes.is_empty() {
            CoverageStatus::Complete
        } else {
            CoverageStatus::Partial
        },
        required_scopes: required,
        observed_scopes,
        missing_scopes,
    })
}
pub fn project(
    plan: &FrozenPlan,
    attempt: &AttemptRecord,
    changes: &[Weakening],
    advice: &[String],
    capability: &str,
) -> Result<EngineProjection, String> {
    require(
        capability == CAPABILITY,
        "unsupported engine adapter capability",
    )?;
    require(
        attempt.finished && attempt.exit_code.is_some(),
        "execution has no completed outcome",
    )?;
    require(
        attempt.exit_code == Some(0)
            || attempt
                .observations
                .iter()
                .any(|o| o.status == CaseStatus::Fail),
        "nonzero exit without test failure is an execution error",
    )?;
    let assessment = assess(plan, attempt, changes)?;
    let coverage = observed_coverage(plan, attempt)?;
    let rule = |id: &str, enforcement: Enforcement, object: &str| GuardRule {
        id: id.into(),
        description: format!("{MAPPING_VERSION}: {object}"),
        enforcement,
        assertion: GuardAssertion::ForbidRelation {
            subject: "testguard:assessment".into(),
            predicate: "has".into(),
            object: object.into(),
        },
    };
    let contract = GuardContract {
        api_version: API_VERSION.into(),
        kind: "GuardContract".into(),
        metadata: ContractMetadata {
            id: MAPPING_VERSION.into(),
            revision: plan.binding().policy_digest.clone(),
        },
        spec: ContractSpec {
            rules: vec![
                rule("mandatory", Enforcement::Enforce, "unsatisfied"),
                rule("weakening", Enforcement::Review, "review"),
                rule("advisory", Enforcement::Advise, "advice"),
            ],
        },
    };
    let fact = |object: &str, source: String| GuardFact {
        subject: "testguard:assessment".into(),
        predicate: "has".into(),
        object: object.into(),
        source,
    };
    let mut relations = Vec::new();
    for instance in &assessment.coverage.unsatisfied {
        relations.push(fact(
            "unsatisfied",
            String::from_utf8(json(instance)?).map_err(|e| e.to_string())?,
        ));
    }
    for change in changes {
        relations.push(fact("review", format!("{change:?}")));
    }
    for message in advice {
        require(!message.trim().is_empty(), "empty advisory source")?;
        relations.push(fact("advice", message.clone()));
    }
    let partial = coverage.status == CoverageStatus::Partial;
    let facts = GuardFacts {
        api_version: API_VERSION.into(),
        kind: "GuardFacts".into(),
        analyzer: AnalyzerIdentity {
            id: ANALYZER_ID.into(),
            version: MAPPING_VERSION.into(),
        },
        subject: GuardSubject {
            id: plan.binding().repository.clone(),
            snapshot_digest: format!("sha256:{}", plan.binding().source_digest),
        },
        completeness: if partial {
            Completeness::Partial
        } else {
            Completeness::Complete
        },
        facts: relations,
        diagnostics: if partial {
            vec![format!(
                "{} required test/environment scopes lack a finished execution",
                coverage.missing_scopes.len()
            )]
        } else {
            vec![]
        },
    };
    let report = evaluate_bounded(&contract, &facts).map_err(|e| e.to_string())?;
    let expected = match assessment.decision {
        crate::policy::Decision::Allow => Decision::Allow,
        crate::policy::Decision::Block => Decision::Block,
        crate::policy::Decision::RequireApproval => Decision::RequireApproval,
    };
    require(
        report.decision == expected,
        "engine mapping disagrees with domain assessment",
    )?;
    Ok(EngineProjection {
        contract_bytes: json(&contract)?,
        facts_bytes: json(&facts)?,
        report_bytes: json(&report)?,
        domain_bytes: json(&assessment)?,
        report,
        coverage,
    })
}
