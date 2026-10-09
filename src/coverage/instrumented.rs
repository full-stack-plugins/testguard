//! Trusted, version-pinned executable fixture only. Never loads candidate programs.
use serde::Serialize;
use std::sync::{
    Arc, Barrier, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
#[derive(Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct Assertion {
    pub expected: String,
    pub actual: String,
    pub passed: bool,
}
impl Assertion {
    fn equal(expected: impl ToString, actual: impl ToString) -> Self {
        let expected = expected.to_string();
        let actual = actual.to_string();
        let passed = expected == actual;
        Self {
            expected,
            actual,
            passed,
        }
    }
}
#[derive(Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub(super) struct Event {
    pub unit: String,
    pub test: String,
    pub environment: String,
    pub baseline: Assertion,
    pub mutant: Option<Assertion>,
    pub mutation_required: bool,
    pub schedule: Vec<String>,
}
impl Event {
    pub fn covered(&self) -> bool {
        self.baseline.passed
            && if self.mutation_required {
                self.mutant.as_ref().is_some_and(|m| !m.passed)
            } else {
                self.mutant.is_none()
            }
    }
}
#[derive(Clone, Copy)]
enum Variant {
    Original,
    NoBalanceGuard,
    NoCancelGuard,
}
struct Account {
    balance: i32,
    cancelled: bool,
}
impl Account {
    fn debit(&mut self, amount: i32, variant: Variant) -> Result<(), ()> {
        if !matches!(variant, Variant::NoCancelGuard) && self.cancelled {
            return Err(());
        }
        if !matches!(variant, Variant::NoBalanceGuard) && amount > self.balance {
            return Err(());
        }
        self.balance -= amount;
        Ok(())
    }
    fn cancel(&mut self) {
        self.cancelled = true
    }
}
fn balance_assertion(variant: Variant) -> Assertion {
    let mut a = Account {
        balance: 5,
        cancelled: false,
    };
    let rejected = a.debit(6, variant).is_err();
    Assertion::equal("true:5", format!("{rejected}:{}", a.balance))
}
fn cancelled_assertion(variant: Variant) -> Assertion {
    let mut a = Account {
        balance: 5,
        cancelled: false,
    };
    a.cancel();
    let rejected = a.debit(1, variant).is_err();
    Assertion::equal("true:5", format!("{rejected}:{}", a.balance))
}
fn parallel_debits() -> (Assertion, Vec<String>) {
    let barrier = Arc::new(Barrier::new(2));
    let account = Arc::new(Mutex::new(Account {
        balance: 10,
        cancelled: false,
    }));
    let trace = Arc::new(Mutex::new(Vec::new()));
    std::thread::scope(|s| {
        let mut handles = vec![];
        for id in 0..2 {
            let barrier = barrier.clone();
            let account = account.clone();
            let trace = trace.clone();
            handles.push(s.spawn(move || {
                barrier.wait();
                let passed = account.lock().unwrap().debit(1, Variant::Original).is_ok();
                trace
                    .lock()
                    .unwrap()
                    .push(format!("worker-{id}:debit:{passed}"));
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    });
    let balance = account.lock().unwrap().balance;
    let mut trace = trace.lock().unwrap().clone();
    trace.sort();
    (
        Assertion::equal("8:2", format!("{balance}:{}", trace.len())),
        trace,
    )
}
fn parallel_cancel() -> (Assertion, Vec<String>) {
    let barrier = Arc::new(Barrier::new(2));
    let cancelled = Arc::new(AtomicBool::new(false));
    let successes = Arc::new(AtomicUsize::new(0));
    let trace = Arc::new(Mutex::new(Vec::new()));
    std::thread::scope(|s| {
        let mut handles = vec![];
        for id in 0..2 {
            let barrier = barrier.clone();
            let cancelled = cancelled.clone();
            let successes = successes.clone();
            let trace = trace.clone();
            handles.push(s.spawn(move || {
                barrier.wait();
                let won = cancelled
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok();
                if won {
                    successes.fetch_add(1, Ordering::SeqCst);
                }
                trace
                    .lock()
                    .unwrap()
                    .push(format!("worker-{id}:cancel:{won}"));
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    });
    let mut trace = trace.lock().unwrap().clone();
    trace.sort();
    (
        Assertion::equal(
            "1:true:2",
            format!(
                "{}:{}:{}",
                successes.load(Ordering::SeqCst),
                cancelled.load(Ordering::SeqCst),
                trace.len()
            ),
        ),
        trace,
    )
}
pub(super) fn execute(unit: &super::advanced::UnitSpec) -> Event {
    let mut mutant = None;
    let mut schedule = Vec::new();
    let baseline = match unit.id {
        "contract.nonnegative" => balance_assertion(Variant::Original),
        "contract.conservation" => {
            let mut a = Account {
                balance: 10,
                cancelled: false,
            };
            let mut b = Account {
                balance: 4,
                cancelled: false,
            };
            if a.debit(3, Variant::Original).is_ok() {
                b.balance += 3
            }
            Assertion::equal(14, a.balance + b.balance)
        }
        "state.cancel" => {
            let mut a = Account {
                balance: 5,
                cancelled: false,
            };
            a.cancel();
            Assertion::equal(true, a.cancelled)
        }
        "state.cancelled_debit" => cancelled_assertion(Variant::Original),
        "concurrency.debits" => {
            let (result, events) = parallel_debits();
            schedule = events;
            result
        }
        "concurrency.cancel_once" => {
            let (result, events) = parallel_cancel();
            schedule = events;
            result
        }
        "mutation.balance_guard" => {
            let original = balance_assertion(Variant::Original);
            mutant = Some(balance_assertion(Variant::NoBalanceGuard));
            original
        }
        "mutation.cancel_guard" => {
            let original = cancelled_assertion(Variant::Original);
            mutant = Some(cancelled_assertion(Variant::NoCancelGuard));
            original
        }
        _ => unreachable!("private fixed registry"),
    };
    Event {
        unit: unit.id.into(),
        test: unit.test.into(),
        environment: super::advanced::ENVIRONMENT.into(),
        baseline,
        mutant,
        mutation_required: unit.kind == super::advanced::MetricKind::Mutation,
        schedule,
    }
}
