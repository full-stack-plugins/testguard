//! Two explicitly instrumented state checks; no whole-state-space claim.
use super::advanced::{MetricKind, UnitSpec};
pub const UNITS: [UnitSpec; 2] = [
    UnitSpec {
        kind: MetricKind::State,
        id: "state.cancel",
        source: "fixture:cancel",
        test: "fixture_cancel",
        anchor: "Account::cancel",
    },
    UnitSpec {
        kind: MetricKind::State,
        id: "state.cancelled_debit",
        source: "fixture:cancelled-debit",
        test: "fixture_cancelled_debit",
        anchor: "Account::debit",
    },
];
