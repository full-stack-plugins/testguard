# Partme TestGuard

[English](README.md) · [简体中文](README.zh-CN.md)

**TestGuard validates whether the required behavior was actually tested — not merely whether a test command exited with code 0.**

> **Status:** detailed architecture and technical blueprint prepared. No executable TestGuard implementation, release or enforced CI integration is claimed yet.

## What TestGuard does

A frozen test plan maps approved SpecGuard acceptance criteria and ArchGuard domain invariants to required tests and environments. Native executors (JUnit/Maven/Gradle, Cargo, Vitest/Jest and Playwright) produce raw evidence, which TestGuard normalizes into execution status, observed behavior and coverage against the pre-approved plan. The common GuardEngine evaluates versioned contracts and evidence.

~~~text
Approved acceptance / domain invariants / change scope
                         ↓
                  Frozen Test Plan
                         ↓
                  Native Test Runners
                         ↓
            Results + Coverage + Provenance
                         ↓
             TestGuard checks → GuardEngine
                         ↓
             CI / FlowGuard stage decision
~~~

A zero-test run, skipped mandatory case, missing report, stale candidate or unauthorized test reduction must **not** be mapped to PASS. Coverage percentage does not prove functional correctness; flaky retries must retain prior failures. TestGuard never creates merge authorization; only trusted branch protections can enforce merge requirements.

## Responsibility and other Guards

SpecGuard defines what must be accepted; ArchGuard defines domain invariant checks; CodeGuard owns static quality; TestGuard owns test-plan adequacy and execution evidence; GitGuard handles candidate/branch safety; FlowGuard checks lifecycle approvals. GuardEngine provides a common contract, rule and evidence protocol. Each product remains independently installable when implemented.

## Documents

- [Detailed architecture](docs/architecture.md)
- [Detailed technical solution](docs/technical-design.md)
- [Guard Protocol v1alpha1](https://github.com/full-stack-plugins/guardengine/blob/main/docs/protocol.md)

## Planned CLI (not available yet)

~~~sh
testguard doctor --project .
testguard plan --contract approved.yaml --candidate HEAD
testguard run --plan test-plan.json --format json
testguard verify --plan test-plan.json --report evidence.json
~~~

Delivery proceeds by real JUnit/Cargo fixtures, frozen plans, negative cases (zero tests, partial runs, tampering), then coverage, additional frameworks, MCP and trusted CI. Planned capabilities are not counted as implemented behavior.
