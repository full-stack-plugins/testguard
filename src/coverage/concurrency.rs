//! Selected real two-thread barrier scenarios, not all schedules or race freedom.
use super::advanced::{MetricKind, UnitSpec};
pub const UNITS: [UnitSpec; 2] = [
    UnitSpec {
        kind: MetricKind::Concurrency,
        id: "concurrency.debits",
        source: "fixture:parallel-debits",
        test: "fixture_parallel_debits",
        anchor: "parallel_debits",
    },
    UnitSpec {
        kind: MetricKind::Concurrency,
        id: "concurrency.cancel_once",
        source: "fixture:cancel-once",
        test: "fixture_cancel_once",
        anchor: "parallel_cancel",
    },
];
