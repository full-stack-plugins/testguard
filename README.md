# TestGuard

[English](README.md) · [简体中文](README.zh-CN.md)

**TestGuard is designed to establish whether required behavior was actually tested, beyond a test command returning zero.**

> **Status: documentation-only design.** Inspected baseline: `b4e8ed05b985f6233d671c76f1132aaa4d99a507` (2026-10-09). That tree contains only the two READMEs and two design documents listed below: no application source, package manifest, tests, fixtures, CI, OpenSpec, CLI, MCP server or release. All capabilities and commands below are proposals, not verified executable features. No implementation tests or OpenSpec validation are claimed.

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
- **Outputs (planned):** frozen `TestPlan`, per-attempt execution records, raw artifact digest references, requirement/test/environment trace matrix, separate coverage metrics, diagnostic findings and a scoped decision. Missing evidence remains explicit.
- **Owned here:** test-plan adequacy, runner adapters, observed test execution, coverage denominators, regression/flaky evidence and provenance checks.
- **Other products:** SpecGuard owns requirement meaning; ArchGuard owns architecture invariants; CodeGuard owns static quality; GitGuard owns candidate/branch safety; FlowGuard owns lifecycle conditions. The six guards are independent products on GuardEngine. TestGuard never grants merge/release authority or authenticates its own approvals.

The initial target is JUnit reports with Maven/Gradle provenance and a pinned Cargo adapter. Vitest/Jest, Playwright, coverage tools, mutation tools, MCP and CI integrations follow measured adapter validation. No external legacy plugin is present or verified in this repository; any compatibility is an unverified future target.

## Planned CLI — not runnable

~~~sh
testguard doctor --project .
testguard plan --contract approved.yaml --candidate HEAD
testguard run --plan test-plan.json --format json
testguard check --plan test-plan.json --execution execution.json --format json
testguard verify --plan test-plan.json --report evidence.json
testguard coverage --requirement REQ-017
~~~

`plan` would resolve `HEAD` into immutable object identities before freezing the plan. `check` targets exit codes **0 ALLOW, 2 BLOCK, 3 REQUIRE_APPROVAL, 4 invalid input/runtime/verification error**, with JSON on stdout and diagnostics on stderr. These are design contracts, not current behavior; other subcommands and `--report` file semantics are not implemented. Incomplete analysis blocks and cannot be overridden by approval. `verify` would recompute evidence, not certify runner trust or authorize a merge.

## Trust and protocol

Plans and thresholds must come from protected baselines, bound to immutable digests and authenticated approval records. Tests execute untrusted candidate code: isolate worktrees, processes, network, resources and credentials; default to no external side effects. Keep every retry, quarantine decision and artifact reference. Changed candidate/base/merge group, rules, analyzer/coverage scope, baseline or approval validity invalidates affected evidence; check the exact merge-queue candidate.

Existing `guard.partme.ai/v1alpha1` supports GuardContract YAML, GuardFacts JSON and GuardReport JSON with exact `forbid_relation` rules. Domain schemas and a proposed [integration envelope](docs/integration-contract.md) are separate; they are not extra fields accepted by that strict current protocol. Reports are unsigned; recomputation is not authentication.

## Design and delivery

- [Architecture, boundaries, trust and evidence](docs/architecture.md)
- [Technical design, contracts and measurable phase acceptance](docs/technical-design.md)
- [Shared integration contract (draft)](docs/integration-contract.md)
- [External GuardEngine protocol reference](https://github.com/full-stack-plugins/guardengine/blob/main/docs/protocol.md)

Phases T0–T4 progress from versioned schemas and missing-tool diagnostics to real JUnit/Cargo fixtures, frozen coverage matrices, isolated execution/flaky recovery, then additional frameworks and trusted integrations. Each phase requires reproducible positive and negative evidence; documentation is not an implementation milestone. Remaining decisions include adapter versions, test-ID stability, sandbox platform and retention limits.
