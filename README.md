# TestGuard

[English](README.md) · [简体中文](README.zh-CN.md)

**TestGuard is designed to establish whether required behavior was actually tested, beyond a test command returning zero.**

> **Status: experimental local Rust implementation.** This tree now contains source, Cargo metadata, tests, captured native reports and an executable CLI; 14/30 tasks have independent acceptance. The documentation-only inventory at historical baseline `b4e8ed05b985f6233d671c76f1132aaa4d99a507` does not describe the current tree. The end-to-end isolated execution and trusted production integration below remain design goals. See the [implementation record](docs/implementation-progress.md) and [actual CLI contract](docs/local-cli.md).

## Intended workflow

A protected, approved test plan maps SpecGuard acceptance criteria and ArchGuard domain invariants to mandatory test identities and environments **before execution**. Native framework adapters collect results, coverage and execution provenance. TestGuard evaluates adequacy; GuardEngine supplies general contract validation, neutral rule evaluation and deterministic evidence computation.

~~~text
Approved requirements + invariants + immutable candidate/base
                ↓
Frozen plan + required tests × environments + coverage denominator
                ↓
Isolated native runner → raw reports + all attempts + provenance
                ↓
TestGuard adequacy findings → schema-compatible GuardEngine facts
                ↓
Scoped technical decision → trusted CI / FlowGuard / GitGuard
~~~

Typical uses include proving that a requirement's negative cases ran, validating a regression across required environments, detecting missing mandatory cases in a green partial run, and retaining flaky failure history. A zero-test run, skipped mandatory case, missing report, stale candidate or unauthorized test reduction cannot pass. Line coverage does not prove requirements satisfied; passing on retry does not erase earlier failures. Detecting arbitrary semantic weakening of assertions remains a research and review limitation.

## Inputs, outputs and boundaries

- **Inputs:** immutable approved obligation/plan references, protected rule contracts, candidate/base/merge-group identity, executor/toolchain/environment inventory, discovered test identities and native reports.
- **Outputs (implemented locally; full integration remains a target):** frozen `TestPlan`, per-attempt execution records, raw artifact digest references, requirement/test/environment trace matrix, separate coverage metrics, diagnostic findings and a scoped decision. Missing evidence remains explicit.
- **Owned here:** test-plan adequacy, runner adapters, observed test execution, coverage denominators, regression/flaky evidence and provenance checks.
- **Other products:** SpecGuard owns requirement meaning; ArchGuard owns architecture invariants; CodeGuard owns static quality; GitGuard owns candidate/branch safety; FlowGuard owns lifecycle conditions. The six guards are independent products on GuardEngine. TestGuard never grants merge/release authority or authenticates its own approvals.

Current native profiles cover pinned Cargo1.99.0 and Maven3.9.9/Surefire3.5.2 and Gradle8.14.3 with JUnit4.13.2 fixtures ([native matrix](fixtures/junit/matrix/README.md)); task2.1 awaits independent review. The broader target includes additional native frameworks. Vitest/Jest, Playwright, coverage tools, mutation tools, MCP and CI integrations follow measured adapter validation. No external legacy plugin is present or verified in this repository; any compatibility is an unverified future target.

## Current local CLI

```sh
cargo build --locked --bin testguard
testguard doctor
testguard plan OBLIGATIONS.json BINDING.json
testguard check PLAN.json ATTEMPT.json [CHANGES.json]
testguard coverage PLAN.json ATTEMPT.json
testguard verify PLAN.json ATTEMPT.json REPORT.json [CHANGES.json]
testguard check-engine PLAN.json INVOCATION.json ATTEMPT.json [CHANGES.json [ADVICE.json]]
testguard verify-engine PLAN.json BUNDLE.json
```

These commands take explicit local inputs; they do not resolve HEAD into an authenticated candidate. `check`/`check-engine` map their own decisions to0/2/3 and input/verification errors to4. `coverage` reports denominators and gaps, not gate acceptance. Bound `check-engine` failures have a null decision; `verify-engine` recomputes real GE artifacts and the domain projection. Neither authenticates the producer, approvals or Git objects. Exact arguments and output semantics are in the [CLI contract](docs/local-cli.md).

Main CLI `run` always refuses with exit4. The separate [Linux fixed-fixture lifecycle helper](docs/decisions/testguard-local-process-profile.md) exercises bounded fixed process trees, timeout/cancellation, output limits and reaping. It is not a native test execution entry point, OS sandbox or production permission. The bounded [artifact collector](docs/decisions/testguard-artifact-collection.md) and [local evidence store](docs/decisions/testguard-evidence-store.md) are explicit library boundaries; they do not automatically isolate all CLI inputs.

Run `cargo test --locked` for current checks. The independently accepted lifecycle checkpoint passed85tests; inspect [process lifecycle tests](tests/process_lifecycle.rs), [artifact security tests](tests/artifact_security.rs) and [engine mapping tests](tests/engine_mapping.rs). The [implementation record](docs/implementation-progress.md) tracks current counts and reviewed commits. This is not a claim that every historical external tool was rerun.

## Trust and protocol

Plans and thresholds must come from protected baselines, bound to immutable digests and authenticated approval records. Tests execute untrusted candidate code: isolate worktrees, processes, network, resources and credentials; default to no external side effects. Keep every retry, quarantine decision and artifact reference. Changed candidate/base/merge group, rules, analyzer/coverage scope, baseline or approval validity invalidates affected evidence; check the exact merge-queue candidate.

Existing `guard.partme.ai/v1alpha1` supports GuardContract YAML, GuardFacts JSON and GuardReport JSON with exact `forbid_relation` rules. Domain schemas and the separately versioned [integration envelope](docs/integration-contract.md), used by the opt-in local engine profile, are separate; they are not extra fields accepted by that strict current protocol. Reports are unsigned; recomputation is not authentication.

## Design and delivery

- [Architecture, boundaries, trust and evidence](docs/architecture.md)
- [Technical design, contracts and measurable phase acceptance](docs/technical-design.md)
- [Shared integration contract (draft)](docs/integration-contract.md)
- [External GuardEngine protocol reference](https://github.com/full-stack-plugins/guardengine/blob/main/docs/protocol.md)

Phases T0–T4 progress from versioned schemas and missing-tool diagnostics to real JUnit/Cargo fixtures, frozen coverage matrices, isolated execution/flaky recovery, then additional frameworks and trusted integrations. Each phase requires reproducible positive and negative evidence; documentation is not an implementation milestone. Remaining production decisions include additional adapter versions, sandbox isolation and deployment retention policy.


## OpenSpec implementation backlog

The incremental [proposal](openspec/changes/add-test-obligation-evidence-pipeline/proposal.md), [design](openspec/changes/add-test-obligation-evidence-pipeline/design.md), [requirements](openspec/changes/add-test-obligation-evidence-pipeline/specs/) and [tasks](openspec/changes/add-test-obligation-evidence-pipeline/tasks.md) translate the architecture into pending implementation work. See the [cross-repository dependency roadmap](openspec/guard-roadmap.md) and [structural validation record](openspec/validation-2026-10-09.md). The task list records14/30 independently accepted items; new implementation is checked off only after independent review. Historical inventories and structural validation records describe their original baselines. Current executable behavior, test evidence and remaining limits are recorded in the implementation/profile documents; structural validity is not runtime acceptance.
