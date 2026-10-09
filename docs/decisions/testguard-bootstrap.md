# Bootstrap profile (experimental local contracts)

Single Rust 2024 crate, minimum Rust 1.99. Cargo 1.99.0 / rustc 1.99.0 were measured in this environment. Exact dependency versions and Cargo.lock define reproducibility; no engine wire fields are extended.

The only initial experimental parser profile is cargo 1.99.0, libtest-pretty-v1. Stable libtest text is parsed conservatively, never unstable JSON. Target, feature set, parameters, native suite/name and environment are part of case identity. Unknown tool/protocol/version rejects. A parser profile is not a certified production runner or a claim of sandboxing. JUnit versions stay unsupported until real Maven/Gradle evidence is collected.

Domain schema is testguard.local/v1 with local-fixture capability, distinct from package, protected policy revision and guard.integration versions. Source approval references are opaque references, never authorization. Production SG imports and engine-backed decisions remain unavailable pending integration. Domain ALLOW is local advisory only.

## Measured native fixtures

Maven 3.9.9 was downloaded from the Apache Maven artifact on Maven Central. Surefire 3.5.2 with JUnit 4.13.2 ran on OpenJDK 21.0.12.1. The experimental `maven-surefire / 3.5.2 / junit-xml-v1` parser is now explicit. Gradle remains unsupported. The constrained Java installation does not support `--release` compiler profiles; the fixture uses source/target 17 and the available compiler module. That is a fixture environment choice, not a portable production support guarantee.

`fixtures/{cargo,junit}/captured/` contains real inputs, exact commands/tool versions, stdout/stderr, exit statuses, and SHA-256 digests. Missing and malformed reports are explicitly recorded post-capture fault injections. Partial captures are filtered real executions and cannot satisfy an unfiltered frozen plan. Cargo timeout captures preserve unfinished case identity as unknown. Maven timeout emits no XML here and the XML parser fails closed; it cannot recover an inventory from a missing report. These limitations keep full adapter/runtime acceptance provisional.

Repeat captures with `python3 fixtures/cargo/capture.py DESTINATION` and `TESTGUARD_MAVEN=/path/to/mvn TESTGUARD_MAVEN_REPOSITORY=/path/to/prewarmed/repository python3 fixtures/junit/capture.py DESTINATION`. Maven capture uses offline dependencies after explicit development setup. Capture helpers are test-only trusted fixture tools, not production sandboxes. They may compile/run only the supplied fixtures.

## SpecGuard fixture interoperability

`fixtures/integration/specguard-obligations.json` is copied from SpecGuard's actual `fixtures/handoffs/obligations.json` export of `specguard.domain/v1alpha1`. `obligation::specguard::import_fixture` preserves that source record, requires explicit test/environment mappings, checks candidate/source digests and produces only a `local-fixture` plan. `fixture-only:specguard-export` identifies provenance, not an approval issuer. Production authentication profiles are deliberately unsupported until SG/GE trust integration is reviewed.
