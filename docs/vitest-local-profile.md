# Vitest local JSON profile

`adapters::vitest::parse` supports Vitest 4.0.18's native JSON reporter and separate
`list --json` discovery in a fixed local Node 24.19.0 profile: one
`cases.test.mjs` source, forks, one worker, retry 0, no nested suites, snapshots,
custom metadata or browser dependencies. This does not enable the general run
command, sandbox arbitrary code, or provide production execution authority.
The native report contains no Vitest version or retry/config attestation: these
are protected controller declarations, with actual installed tool/source hashes
recorded by the fixture collector. A supplied report cannot authenticate them.

The committed real corpus includes pass, assertion failure, runtime skip, zero,
filtered partial, native timeout, reporter write failure, bounded collector
truncation, hard timeout after the test body starts, duplicate parameter titles
and an additional static-skip negative control. Collector truncation retains the
intact native report separately; it is not described as malformed native output.
Native failure/timeout and filtered skip observations preserve their exact
counts and statuses. Duplicate title ambiguity rejects even when Vitest exits 0.

**Vitest's actual default list command excludes static skipped tests.** The main
skip case uses actual `ctx.skip()` during execution after native discovery. The
extra real static-skip case demonstrates the missing discovery identity and is
rejected rather than silently adding a passing obligation or shrinking scope.
Consumers must independently protect the required frozen plan; discovery is
not sufficient authority to remove requirements. The zero-test fixture retains
an independent required placeholder in domain tests, hence cannot pass vacuously.
Missing/interrupted reports return only unfinished Unknown observations for the
original discovery; they do not infer successful cases from stdout progress.

Native case identity consists of file target plus full name (this profile has
no ancestor suites), parameters and environment. Native absolute file paths
must agree exactly between discovery and report. The canonical test ID uses the
fixed relative target, so fresh isolated directories do not change identity.
The frozen plan/current attempt and artifact digest remain separate mandatory
consumer bindings. Compatible-looking old raw bytes do not create authority.

Both JSON inputs are capped at 4 MiB; lexical preflight limits nesting to 64 and
structural/string tokens to 131072 before serde allocations. Discovery and
results each cap at 4096 cases, and existing ObservationBudget reserves bounded
identity/scope expansion before case hashing/cloning. Schema fields are closed;
unknown statuses, result/title/path drift, duplicate identities, unsupported
metadata/snapshots, counts/suite verdicts and exit contradictions fail closed.
No report attachment paths are followed. Nested-suite support is deliberately
absent because native suite counters require a separately qualified profile.

Run `python3 fixtures/vitest/capture.py <directory>` to regenerate actual output.
`cargo test --locked --offline --test vitest_adapter -- --include-ignored`
regenerates and tests it. Toolchain package/package-lock are committed as install
provenance; the fixture uses the isolated `/workspace/guard-toolchain/vitest-4.0.18`
installation. Install used exact npm version with `--ignore-scripts`; official
optional platform binaries already execute successfully, with no install script.

Official protocol implementation: [Vitest 4.0.18 JSON reporter](https://raw.githubusercontent.com/vitest-dev/vitest/v4.0.18/packages/vitest/src/node/reporters/json.ts),
[reporter documentation](https://v4.vitest.dev/guide/reporters),
[CLI discovery](https://vitest.dev/guide/cli).

This is a partial slice of task 5.1. Playwright has a separately qualified native
profile; Jest remains outstanding. No complete task checkbox is changed here.
