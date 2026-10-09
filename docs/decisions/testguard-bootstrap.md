# Bootstrap profile (experimental local contracts)

Single Rust 2024 crate, minimum Rust 1.99. Cargo 1.99.0 / rustc 1.99.0 were measured in this environment. Exact dependency versions and Cargo.lock define reproducibility; no engine wire fields are extended.

The only initial experimental parser profile is cargo 1.99.0, libtest-pretty-v1. Stable libtest text is parsed conservatively, never unstable JSON. Target, feature set, parameters, native suite/name and environment are part of case identity. Unknown tool/protocol/version rejects. A parser profile is not a certified production runner or a claim of sandboxing. JUnit versions stay unsupported until real Maven/Gradle evidence is collected.

Domain schema is testguard.local/v1 with local-fixture capability, distinct from package, protected policy revision and guard.integration versions. Source approval references are opaque references, never authorization. Production SG imports and engine-backed decisions remain unavailable pending integration. Domain ALLOW is local advisory only.
