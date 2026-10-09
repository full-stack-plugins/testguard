//! Selected actual guard removals; detection never proves arbitrary semantic completeness.
use super::advanced::{MetricKind, UnitSpec};
pub const UNITS: [UnitSpec; 2] = [
    UnitSpec {
        kind: MetricKind::Mutation,
        id: "mutation.balance_guard",
        source: "fixture:mutation-balance",
        test: "fixture_mutation_balance",
        anchor: "Account::debit/remove_balance_guard",
    },
    UnitSpec {
        kind: MetricKind::Mutation,
        id: "mutation.cancel_guard",
        source: "fixture:mutation-cancel",
        test: "fixture_mutation_cancel",
        anchor: "Account::debit/remove_cancel_guard",
    },
];
