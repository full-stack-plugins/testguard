# Jest native JSON and protected case manifest

The qualified artifact is the official npm **Jest distribution 30.2.0** with its
committed transitive package-lock. Its actual `--version` prints **30.1.3**:
`@jest/core/package.json` says 30.2.0, while the published core build embeds
30.1.3. We preserve both facts and hash the installed core build; we neither
patch the distribution nor claim that the CLI prints 30.2.0. The executor tuple
is `jest / 30.2.0 / jest-json-distribution-v30.2.0`; its parameters explicitly
include `cli=30.1.3`. Reports have no self-authenticating tool-version field.

`PreparedCases::freeze(controller_manifest, native_file, profile)` accepts
protected controller input **before execution**. Its private state binds schema,
distribution version, CLI display version, core/source/config SHA-256 declarations,
required case names, fixed target and executor scope. `required_test_ids()` gives
the immutable required IDs for freezing the plan independently of any report.
These declarations are not an authentication provider, an attestation that those
bytes executed, or a replacement for run/attempt/source binding by the consumer.
The caller must supply the protected expected file/profile/manifest; copying
untrusted output into those expectations does not create trust.

`adapters::jest::parse(raw, profile, &prepared)` reads the actual native
`--listTests --json` **file** discovery. Jest does not provide case discovery
through that command. Accordingly the fixture controller writes a separately
labeled required-case manifest before both native commands. The parser never
uses results to reduce requirements, and never labels controller names as native
case discovery. Only actual native assertion records become CaseObservations.
If a killed run has no final JSON, there are no case observations; all protected
required IDs remain unsatisfied. Native empty-file runtime error likewise keeps
the independent required placeholder unsatisfied, with real exit1 retained.

The supported local profile uses Node24.19.0, one cases.test.cjs source, no
nested suites, runInBand, no retries, no snapshots, and no coverage provider.
This is not arbitrary-code sandboxing or production authority, and does not
open the disabled general run command. Normal Cargo/JUnit/Vitest/Playwright
readers retain their existing entry points.

Real committed scenarios cover pass, assertion fail, static skip, zero tests,
filtered partial, native500ms timeout, native reporter write failure, bounded
collector truncation, external kill after a real test-body marker, and duplicate
parameter titles. Duplicate names reject even though Jest's real collision run
exits0 and reports two passes. Skip is native status `pending`; whole skipped
suite is `skipped`, while a filtered suite with a pass is `focused`. The parser
checks these different suite/count rules against the observed protocol.
Malformed input is expressly a128-byte bounded collector read of genuine bytes;
the intact native report is retained and is not claimed to be malformed output.

Required identities commit the entire controller manifest and profile; absolute
fresh work directory is checked against protected discovery/report paths but
excluded from stable case IDs. Unknown fields/statuses, duplicate identity,
source drift, unsupported retries, missing evidence, counter/verdict/exit or
start/end/duration contradictions reject. Executed duration uses checked addition.
Skipped cases cannot claim an executed duration or passing assertions.

Raw report and file inventory each cap at4MiB; manifest caps at1MiB. Before
serde allocation a lexical scan caps nesting at64 and tokens at131072. Required
names and result cases cap at4096; identity fields/total expansion are bounded
before cloning or hashing. Existing ObservationBudget also bounds output case
expansion. Artifact digest and attempt URI are checked; attachments are not read.

`python3 fixtures/jest/capture.py <directory>` regenerates the fixed native
matrix. `cargo test --locked --offline --test jest_adapter -- --include-ignored`
regenerates it, parses actual output, and verifies stable IDs across fresh
work directories. Provenance records exact commands, raw hashes, package/core
versions, installed tool hashes and original source/config/lock bytes.

Official implementation: [Jest30.2.0 result formatter](https://raw.githubusercontent.com/jestjs/jest/v30.2.0/packages/jest-test-result/src/formatTestResults.ts),
[CLI reference](https://jestjs.io/docs/cli).

Task5.1 requires independent acceptance of all three native framework slices.
This implementation does not itself mark the complete task accepted.
