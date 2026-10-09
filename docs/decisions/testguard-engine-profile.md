# Local GuardEngine mapping and envelope profile

Integration dependency: sibling `guardengine`0.1.0. Lifecycle interface read at GE11a5ba7; final consuming verification targets actual source commit11e1d86eb353f006844ebb6eefd93f24b98ee481 (includes normal tempfile dependency and bounded producer evaluation). `Cargo.lock` fixes registry dependencies but cannot pin a path dependency's source. Reproducing this slice requires the documented sibling revision. A concurrent GE Cargo.toml dependency move caused a genuine --locked lint failure during development; the lock was refreshed offline and final checks must pass with --locked.

Supported local capability: `testguard.engine-fixture/v1`. Mapping/analyzer version: `testguard.engine-map/v1`, analyzer `testguard-local-evidence`. Bundle serde readers reject unknown fields; GE owns and validates the unextended `guard.integration/v1alpha1` envelope and `guard.partme.ai/v1alpha1` engine wire. Unknown capability/version is not implicitly upgraded. There is no production provider or productionGate field.

## Domain mapping

`project` validates the frozen plan and completed attempt, recomputes domain assessment, and constructs three fixed exact relation rules. Unsatisfied mandatory edges map to enforce, known weakening changes map to review, and explicit advisory observations map to advise. GE integration::evaluate_bounded checks the shared evaluation expansion budget before generating the actual report; the adapter checks its decision against the domain decision. Incomplete required execution coverage makes facts partial with diagnostics and GE evaluates BLOCK/INDETERMINATE. Completed native failures remain technical BLOCK. Unfinished attempts and runtime nonzero exits without known test failures are execution errors, not manufactured technical reports.

Required scopes are the sorted unique strings `testguard:test-environment:<sha256>`, where SHA256 hashes TestGuard's canonical JSON array `[test_id, environment]`. This is a versioned local convention, not a cross-language canonical standard. Scope requirements derive only from FrozenPlan. Actual observed scopes require discovered, started and finished pass/fail observations for the exact plan; skip/unknown/absent cases remain missing. A failed test is observed even though it is unsatisfied. Requirement identities remain separately bound in RunBinding, and obligation trace edges remain in domain evidence.

## Preparation, completion and verification

Preparation validates all supplied fixture context, including timestamps, against the domain plan and actual GE types before calling GE prepare_attempt. It freezes required scopes with no observations. Completion supplies actual final coverage through GE AttemptOutput.coverage; GE rejects changed required scopes. Neither invocation construction nor lexically valid object IDs establish authority or Git existence.

Bound parser/runtime/cancel errors have null decision, no successful report reference and exit4. Available raw input and the frozen plan are preserved. Valid same-attempt partial observations are retained in actual error coverage; foreign/invalid inputs cannot contribute observed scopes. Original domain/raw references remain in the captured attempt, but this bundle does not claim to fetch or authenticate original native artifact content or implement retention.

A completed local FixtureBundle carries exact inline artifact bytes under generated artifact:// URIs. Envelope references have only URI, sha256-prefixed digest and mediaType; domain size and native metadata remain inside domain records. Publication is not performed. Before issuing completed output, GE byte/recomputation checks are run; oversized or unverifiable completion becomes an error outcome. Verification rechecks all referenced bytes, uses actual GE verify_engine_artifacts and recomputes TestGuard domain facts and coverage. Unsigned digest consistency does not authenticate the producer.

## Trust and remaining gates

GE's reviewed eligibility/provider/store ports were inspected. No provider was selected, implemented, loaded or trusted by TestGuard. Fixture bundle approval refs are empty and recomputation rejects injected refs. Later controller integration must authenticate producer/approvals, verify actual repository/source identities, enforce protected policy and freshness, and control storage access. Sandbox, bounded runner, Gradle and production authorization remain incomplete. No new task checkbox is accepted based only on this implementation.
