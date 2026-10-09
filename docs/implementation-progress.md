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
