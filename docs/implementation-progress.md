# Implementation ledger — add-test-obligation-evidence-pipeline

Base: af115eaee389fd14d6089323417a51418c618722. Worktree: impl/guard-roadmap-20261009.
Baseline: documentation only; no previous runtime or executable tests.

Ruling: implement local-testguard/v1 fixture capability before external SG/GE gates. It carries approval source references but cannot authenticate them or produce trusted integration evidence. Cost if wrong: migrate fixture serialization; production consumers remain unavailable.
Ruling: Cargo 1.99.0 stable pretty libtest text, with separately captured inventory and target/features/parameters identity, is the initial experimental native profile. No unstable libtest JSON assumption. Promotion requires all native conformance cases. JUnit identity includes suite/class/name/parameters/environment; duplicate identities reject.

| Tasks | Concrete edits/tests | Prerequisite/status |
|---|---|---|
|1.1|pinned single crate; bootstrap_profiles tests; bootstrap decision|toolchain present|
|1.2|strict obligation/plan models and JSON schemas; >=12 domain_schema cases|local capability only|
|1.3|attempt/coverage/finding records; domain_records status tests|local models|
|1.4|restricted artifact refs, canonical JSON SHA-256; normalization repeated ten times|no identity authentication claim|
|1.5|CLI doctor/plan/check/coverage/verify + refusal for run until sandbox; exit tests|run remains blocked by group3|
|1.6|read-only PATH discovery with injectable environment; doctor tests|no execution/download|
|2.1|native JUnit adapter and identity/security tests|Maven/Gradle absent; real matrix pending|
|2.2|Cargo parser + actual generated fixtures and captured raw output|Cargo1.99.0 installed|
|2.3|common attempt/artifact/profile validation and collision tests|native adapters|
|2.4|immutable local fixture frozen plan; six independent missing cases|SG/GE production export unavailable|
|2.5|execution/requirement matrix; separate source metrics and weakening review|frozen plan|
|2.6|bidirectional local source/obligation/test/attempt/artifact trace|SG production references unavailable|

Pre-flight interfaces: obligation→plan→coverage: required instances copied from validated source; observations cannot replace them. Adapter→coverage: observations must retain attempt/environment/target/native identity; only finished pass counts. Local binding separate from GE wire. No task checkboxes changed before root review.

## Slice 1 implementation result (pending root review)

- 1.1–1.4: local bootstrap, strict typed domain contracts + five Draft-7 schemas, execution/coverage/finding records and deterministic restricted artifact normalization implemented. Cross-document semantic checks run in Rust, not JSON Schema alone.
- 1.5: local CLI check/verify/plan/coverage/doctor implemented with 0/2/3/4 and output separation; run safely unavailable pending group3 sandbox, so full execution interface remains partial.
- 1.6: Linux PATH executable presence/permission checks for Cargo and Maven implemented; both installed/missing fixture environments are generated and scripts are never invoked. Version execution/ACL/production permission checks are not claimed.
- 2.1: Surefire3.5.2/Maven3.9.9/JUnit4.13.2 real matrix captured. Gradle remains unsupported; missing XML on timeout is rejected rather than recovering unknown case inventory. Partial full-task acceptance.
- 2.2: Cargo1.99.0 stable pretty parser and eight-category native/fault matrix, real feature-scoped target, unknown version and incomplete timeout cases implemented. Missing/malformed are labelled post-capture faults; these are not sandbox/trusted-runner evidence.
- 2.3: artifact digest/attempt reference, native identity + target/features/parameters/environment, duplicate/count/exit contradiction validation implemented locally. Authenticated collection receipts remain group3/4 work.
- 2.4: six-instance frozen matrix and actual SG fixture import implemented. Production SG/GE capability rejected; execution argv/budgets/artifact requirements not yet frozen. Partial.
- 2.5: separate execution/obligation/statement/branch denominators, N/A reasons, missing/extra pass behavior and explicit known weakening comparison implemented. Local source metric inputs and baseline snapshots are not authenticated.
- 2.6: local bidirectional-queryable trace entries preserve both requirement scopes, immutable revision/reference, test/environment/attempt/artifact; original SG source metadata preserved with imported fixture. Production trace authorization and unified finding linkage remain partial.

Observed RED→GREEN logs live under `/workspace/guard-implementation-ledger/tg-*.log`; detailed per-task evidence is in `testguard-slice1-report.md`. Main complete suite: 54 tests, no failures; schemas: 5 checked, 1 valid/8 invalid shape vectors; strict OpenSpec: valid/no issues. Full-task checkboxes stay unchecked for independent root review.

Ruling: Maven compiler fixture uses source/target17 rather than --release17 because the installed constrained Java runtime rejects release profiles; the real capture records Java21.0.12.1. Cost if wrong: repeat native corpus on a full JDK; no production portability claim.
Ruling: missing/malformed corpus cases mutate or remove artifacts after genuine native runs, preserving raw output and recording the transform. Native frameworks do not deliberately emit malformed reports; fault provenance must not be disguised. Cost if wrong: rerun additional fault mechanisms during runner validation.
Ruling: SpecGuard producer's actual fixture export can be consumed now under local-fixture only; production profile rejects until controller authority/GE contracts are integrated. Cost if wrong: update importer and pinned compatibility corpus, not protected gates.

Verification found and fixed: deserialized FrozenPlan bypass could reduce the instance matrix; assessment now revalidates it (RED→GREEN test). Unknown JUnit child status could silently look like pass; unsupported child elements now reject (RED→GREEN test). Cargo timeout's partial status line now remains unknown (real timeout RED→GREEN). Clippy found a collapsible conditional; refactored without suppressions.

## Independent review fix pass

Review: `/workspace/guard-implementation-ledger/testguard-review.md`; base `fe85e3d`.

- P2 adapter profile fidelity: added a regression using genuine captured Cargo bytes with the otherwise supported Surefire profile. RED: Cargo parser incorrectly accepted it. GREEN: Cargo entry point now requires `tool=cargo` and the pinned supported triple. Global discovery still lists both adapters correctly.
- P2 execution denominator: added shared-test mappings with six obligation edges but four unique test/environment executions. RED: complete metric6/6 instead of4/4; a missing shared execution4/6 instead of3/4. GREEN: execution numerator/denominator use distinct required pairs, while obligation satisfaction and missing/unsatisfied trace edges retain every obligation. Missing case2/Windows yields execution3/4 and obligations2/3; missing shared case1/Windows yields execution3/4 and obligations1/3 plus both O1/O3 gaps. Extra passes still cannot fill gaps.
- Updated CoverageEvidence field descriptions, generated Draft7 schema and CLI counting documentation together. No wire field/version change; this corrects the existing execution metric semantics.
- Final validation: `cargo test`57 passed/0 failed (including fresh native Cargo execution and native Maven capture consumers); Clippy warnings denied, formatting, schema shape checks and diff whitespace checks passed. Original independent reviewer repro now prints `Cargo parser accepts Surefire profile: false` and execution3/4, obligations2/3.
- Root review previously accepted local tasks1.1/1.2/1.3/1.6; root retains ownership of acceptance registration/checkboxes. This fix pass does not expand production trust, sandbox or other partial-task claims.

## Root acceptance registration

Root confirmed independent fix verification:57 tests, both P2 findings closed. Registered exactly7/30 accepted tasks:1.1,1.2,1.3,1.6,2.2,2.3,2.5. Evidence: `/workspace/guard-implementation-ledger/testguard-review.md`, `/workspace/guard-implementation-ledger/testguard-review-fixes.md`, and commit11ef725. All other checkboxes remain unchecked; partial scopes retain their limitations.

Next slice: actual GuardEngine projection/envelope and shared scope lifecycle integration (local task4.1–4.3 and relevant CLI1.5 portions, not an invented sandbox implementation). Read actual GE799fb1e interfaces; trust API remains under review and lifecycle actual coverage fix is pending. Local-fixture consumption only; no verified provider means no production gate. `run` remains unavailable until sandbox and execution authority are implemented.

### Engine adapter slice executable plan

-4.1: `report/engine_adapter.rs` recomputes a domain assessment from validated plan/attempt, projects fixed enforce/review/advise relations into real GE contracts/facts, and evaluates with GE. `tests/engine_mapping.rs` covers pass/fail/missing/review/advice and unsupported capability, preserving the current strict wire.
-4.2: `report/transport.rs` consumes GE InvocationDraft/BoundAttempt lifecycle. Invalid/unresolved preparation returns GE TransportDiagnostic without envelope; after binding, parse/runtime/cancel errors use null decision/4 and retain known domain refs. No real Git resolution/provider authentication is claimed.
-4.3: `report/envelope.rs` binds exact local plan context to externally supplied GE invocation context, supplies final actual scope coverage through GE11a5ba7 AttemptOutput.coverage, references exact projected bytes and validates via GE verify_engine_artifacts. `tests/envelope_parity.rs` checks partial BLOCK, report parity, unknown input, digest tampering and recomputation-only verification.
-CLI1.5: opt-in `check-engine`/`verify-engine` local fixture commands; preserve existing local CLI behavior and `run` refusal. `tests/engine_cli.rs` checks stream/exit boundaries and actual wire artifacts.
-Trust4.4+: inspect reviewed GE ports but no provider is selected or fabricated. No productionGate is produced. All new tasks await independent acceptance.

Dependency: sibling pinned-source `guardengine` crate, integration version guard.integration/v1alpha1/current engine wire guard.partme.ai/v1alpha1. Root provided lifecycle11a5ba7. Cargo.lock must resolve actual dependencies once and subsequent verification must use --locked; a path lock does not pin sibling source, so source commit provenance is separately documented.

### Engine fixture slice implementation result (awaiting root review)

-4.1 local mapping implemented using actual GE0.1.0 `evaluate`/strict native types; mandatory fail/gap, partial, review, advisory and capability vectors pass. A genuine captured Cargo result replays through native parsing, plan mapping, GE evaluation and envelope verification; replay is not a new trusted execution.
-4.2 local pre-binding/bound lifecycle implemented with GE prepare_attempt/BoundAttempt. Invalid/missing context is diagnostic-only; bound parser/runtime/cancel outcomes have null decision/4, retain original input and valid same-attempt observations. Repository object existence and authenticated controller resolution remain outside this fixture profile.
-4.3 local completed bundle/parity implemented with exact inline bytes and actual final GE coverage. Verification calls GE and recomputes domain scope, rejects structurally-valid scope reduction, missing refs, drift/tampering and unknown versions/fields. Oversized completion was observed issuing unverifiable ALLOW during development; a RED→GREEN regression now requires error/null and pre-issuance GE byte/recomputation checks.
-CLI1.5 gained opt-in check-engine/verify-engine; existing commands and run refusal preserved. No production provider, productionGate, sandbox, publication or protected eligibility integration was added; trust4.4 and production criteria remain incomplete.
-Dependency provenance: final GE source0b3735fef44ace0624c0d0a79d883eb08982a8c5 (lifecycle11a5ba7). During concurrent GE development its tempfile dependency moved to runtime and an actual `cargo clippy --locked` failed. `cargo generate-lockfile --offline` refreshed the consumer lock; subsequent verification uses --locked. A path dependency's code revision is recorded separately from Cargo.lock.

No new checkbox is changed by this engine slice. Detailed review evidence: `/workspace/guard-implementation-ledger/testguard-engine-slice-report.md`.

Engine slice final verification against clean GE0b3735f: `cargo test --locked`69 passed/0 failed; `cargo clippy --locked --all-targets -- -D warnings`, formatting and whitespace checks passed. Strict OpenSpec valid. Actual CLI fixture REQUIRE_APPROVAL envelope passed GE's published JSON Schema with format validation; unknown version/field rejected. Example bundle and plan plus logs preserved outside repo in the implementation ledger. Existing accepted task count stays7/30.

### Bounded producer evaluation follow-up

GE11e1d86 added the shared `integration::evaluate_bounded` producer API. TestGuard now calls it before report generation and retains GE artifact verification before completed issuance. A regression with individually bounded input but over-budget three-rule expansion failed under the former native evaluate call, then passed using the real GE API. Generic budget logic remains owned by GE. Final source provenance:11e1d86eb353f006844ebb6eefd93f24b98ee481. Checkbox acceptance remains7/30.

Final bounded-API validation: `cargo test --locked`70 passed/0 failed (including genuine Cargo capture replay and fresh native Cargo runner tests), `cargo clippy --locked --all-targets -- -D warnings`, `cargo fmt --check`, and `git diff --check` passed. Cargo.lock required no change for GE11e1d86. RED/GREEN evidence: `tg-bounded-projection-red.log` and `tg-bounded-projection-green.log` in the implementation ledger.
