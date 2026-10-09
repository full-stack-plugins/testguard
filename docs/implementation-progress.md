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
