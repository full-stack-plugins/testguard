# Local controller scheduler

The opt-in in-memory Scheduler admits work for the existing BoundFixtureAttempt
producer. It never starts an OS process, grants credentials or enables main run.
The controller executes worker code only for Admission::Start's private Lease.
Exact duplicate request IDs return Existing status, including pending/completed/
abandoned, without issuing another lease. A changed plan, invocation, mapping
changes/advice or producer under that request ID conflicts. New retries require
new request IDs and run IDs; there is no cross-run result cache.

Admission freezes testguard.scheduler-work/v1alpha1 over the entire plan,
invocation, changes, advice, producer and scheduler limits BEFORE worker execution.
The scope key includes repository/task/worktree/requirements and excludes candidate
so newer work replaces eligibility in that scope. Other requirement/task scopes
remain independent. Every successful new admission sets current to pending under a
mutex before returning the lease; old success is not current while work is pending.
Current queries additionally require exact full native binding, not just scope.

A lease consumes its own frozen producer. Completion has private scheduler identity,
request ID and original full-work identity; no caller can submit raw JSON or a new
hash to relabel it. The original FixtureBundle bytes are preserved. publish compares
against the current request: current results publish, late ones only archive, exact
repeat publication returns Duplicate, and foreign scheduler results reject. Cloned
Scheduler handles refer to the same controller instance; separately constructed
instances are isolated.

Default/hard caps:256 admitted request/run IDs,16 outstanding leases/completions,
64MiB encoded history and4MiB per encoded bundle. Controllers may lower caps with
with_limits. IDs cap256bytes; plan/invocation/mapping metadata and each worker input
cap256KiB using borrowed counting serialization before hashing/cloning/validation.
Bindings cap64KiB and256 requirements. Changes/advice cap64 (advice strings4096bytes),
plan instances/obligations/sources1024, environments/requirements256 and per-obligation
source/environment references256. Worker observations cap4096, artifact references256,
features256 per observation. Native domain/GE budgets apply as well. These are work/
byte limits, not a wall-clock deadline or process sandbox.

Outstanding completion retains its inflight slot until publication or drop. Dropped
leases/completions and failed output-budget paths mark their row abandoned, free the
slot and never restore prior success. A history-cap rejection leaves current pending
and history unchanged; the private completion may be retried or dropped. No retention
pruning, durable crash recovery or scheduler-global discovery exists. Request caps
bound metadata even for abandoned entries. Poisoned locks fail closed.

Fixtures use actual threads/barriers and native producer evaluation. They verify
single start under duplicate races, pending invalidation, late ALLOW versus newer
BLOCK, independent task/requirement scopes, exact query binding, cancellation/foreign
completion, byte-identical archives, immutable mapping changes, capacity/drop behavior
and bound failure for mismatched worker input. This is local producer admission, not
proof of native command isolation, authenticated CI events or production authority.
