//! Borrowed admission accounting, before plan validation rebuilds its matrix or assessment clones gaps.
use crate::{plan::FrozenPlan, policy::Weakening, report::AttemptRecord};
use serde::Serialize;

const LIMIT: usize = guardengine::integration::MAX_ARTIFACT_BYTES;
const MAX_MATCH_WORK: usize = 1_000_000;
const ERROR: &str = "domain assessment budget exceeded";

struct Budget {
    remaining: usize,
    weight: usize,
}
impl std::io::Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let cost = bytes.len().saturating_mul(self.weight);
        if cost > self.remaining {
            return Err(std::io::Error::other(ERROR));
        }
        self.remaining -= cost;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl Budget {
    fn count(&mut self, value: &impl Serialize, weight: usize) -> Result<(), String> {
        self.weight = weight;
        serde_json::to_writer(self, value).map_err(|_| ERROR.to_owned())
    }
}

pub(crate) fn preflight(
    plan: &FrozenPlan,
    attempt: Option<&AttemptRecord>,
    changes: &[Weakening],
    advice: &[String],
) -> Result<(), String> {
    // No JSON Value, encoded Vec, native string clone, or semantic validation before this guard.
    let mut budget = Budget {
        remaining: LIMIT,
        weight: 1,
    };
    budget.count(plan, 1)?;
    budget.count(&attempt, 1)?;
    budget.count(&changes, 2)?;
    budget.count(&advice, 2)?;

    #[derive(Serialize)]
    struct Edge<'a> {
        obligation_id: &'a str,
        test_id: &'a str,
        environment: &'a str,
    }
    let source = plan.obligations();
    let mut expected_edges = 0usize;
    // Derive the expansion from source obligations, not a potentially forged instances array.
    for obligation in &source.obligations {
        for environment in &obligation.environments {
            expected_edges = expected_edges.saturating_add(1);
            // Six encoded-edge equivalents conservatively cover rebuilt matrix, two gap lists,
            // owned fact source, domain serialization and collection overhead. This is admission
            // accounting, not an exact allocator/RSS bound. Never discard edges to fit the cap.
            budget.count(
                &Edge {
                    obligation_id: &obligation.id,
                    test_id: &obligation.test_id,
                    environment,
                },
                6,
            )?;
        }
    }
    let edges = expected_edges.max(plan.instances().len());
    let observations = attempt.map_or(0, |a| a.observations.len());
    let scan_width = observations
        .saturating_add(source.obligations.len())
        .saturating_add(source.environments.len());
    let work = edges.saturating_mul(scan_width).saturating_add(
        source
            .sources
            .len()
            .saturating_mul(source.requirements.len()),
    );
    if work > MAX_MATCH_WORK {
        return Err(ERROR.into());
    }
    Ok(())
}
