//! Fixed contract assertions; not source-line coverage.
use super::advanced::{MetricKind, UnitSpec};
pub const UNITS: [UnitSpec; 2] = [
    UnitSpec {
        kind: MetricKind::Contract,
        id: "contract.nonnegative",
        source: "fixture:balance",
        test: "fixture_balance",
        anchor: "Account::debit",
    },
    UnitSpec {
        kind: MetricKind::Contract,
        id: "contract.conservation",
        source: "fixture:conservation",
        test: "fixture_conservation",
        anchor: "execute/contract.conservation",
    },
];
