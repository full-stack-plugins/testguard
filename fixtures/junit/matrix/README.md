# Pinned native JUnit evidence matrix

Actual Linux executions use Maven3.9.9/Surefire3.5.2 and Gradle8.14.3, Java21.0.12.1, JUnit4.13.2. The new fixture uses Java release17. `captured/provenance.json` records every fixture source SHA256, capture-script SHA256, tool distribution/JDK modules/JVM/JUnit/provider hashes and actual version output. Gradle's official distribution hash is `bd71102213493060956ec229d946beee57158dbd89d0e62b91bca0fa2c5f3531`, verified against https://downloads.gradle.org/distributions/gradle-8.14.3-bin.zip.sha256.

Each tool has these real executions; counts below are tests/failures/errors/skipped:

| Scenario | Native exit | Native XML | Consumed result |
|---|---:|---|---|
|pass|0|2/0/0/0|2 passes; frozen coverage2/2|
|fail|1|1/1/0/0|failure; BLOCK|
|skip|0|1/0/0/1|skip; BLOCK|
|zero|0|no report|incomplete/error; BLOCK|
|partial|0|1/0/0/0|1 pass; frozen coverage1/2, BLOCK|
|timeout|-9|no report|actual testcase marker then SIGKILL; incomplete/error, BLOCK|
|missing-report|0|2/0/0/0|consumer removes report AFTER successful execution; BLOCK|
|malformed|0|2/0/0/0|consumer truncates report AFTER successful execution; BLOCK|
|parameter-unique|0|2/0/0/0|two distinct native IDs retained|
|parameter-collision|0|2/0/0/0|two actual identical names rejected|

Missing/malformed are **postcapture fault injections**, not native emitter behavior. Original XML remains byte-for-byte in `report.raw.xml`; consumed bytes are separate `report.input.xml`. Native Gradle counts come from its real `TestResult` afterSuite callback printed to stdout. Maven counts come from Surefire's stdout. Tests compare both against original XML and capture metadata. Native parameter IDs are never rewritten.

Run from repository root with the fixed toolchain paths declared at the top of `capture.py`:

```sh
python3 fixtures/junit/matrix/capture.py /workspace/guard-implementation-ledger/junit-recapture-new
source /workspace/guard-toolchain/env.sh
cargo test --locked --offline --test junit_matrix
```

Use a new output directory: existing scenario directories are not overwritten. Every `capture.json` preserves exact argv, temporary cwd/report path, exit/termination, fresh report root, output hashes, cleanup and fault provenance. Builds run offline, with fresh per-run fixture copies and report directories, pinned dependencies and bounded JVM heaps/workers. The fixture-only Python supervisor imposes120seconds per build/startup,2MiB per stdout/stderr and32 observed descendants, enables subreaping only in its own process, and kills/reaps descendants including detached Gradle single-use daemons. Timeout triggers only after `TG_TIMEOUT_READY` from inside the test method, plus0.5seconds. These are resource safeguards for trusted fixtures, not a hostile-process containment guarantee.

The adapter rejects an artifact reference belonging to an old attempt even with unchanged valid native bytes. Fresh capture roots prevent accidental report reuse during these runs. Digest/attempt metadata does not authenticate an attacker who can relabel old bytes with forged provenance; controller identity and production trust remain outside this local profile. The test-only capture helper is not an OS/filesystem/network sandbox, does not support candidate commands and does not unlock main `testguard run`.

Task2.1 stays unchecked until independent review. These captured files are regression inputs; ordinary Cargo tests validate them without claiming a fresh Maven/Gradle execution on every test run.
