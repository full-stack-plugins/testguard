# Playwright local JSON profile

`adapters::playwright::parse` reads Playwright 1.62.1 JSON plus a separate native
`--list` JSON discovery report. The qualified fixture runs Node 24.19.0 and the
existing Debian **system Chromium 151.0.7922.173**, not Playwright's bundled
browser revision. One `system-chromium` project, one worker, retries 0 and
repeatEach 1 are supported. Other versions, projects and browser environment
labels fail closed. This does not enable the disabled general `run` command,
provide OS sandboxing, authenticate a tool, or provide production authority.
The profile's browser/environment label is controller-declared; native JSON
alone does not attest the executable. The capture harness separately records
binary hashes and asserts the real browser version during executed page tests.

IDs commit native spec ID, source file/title/location and executor scope.
Discovery supplies the denominator; filtered-out executions remain Unknown.
An empty interrupted report preserves every discovered case as unfinished.
Native timedOut is a failed case, skips never pass, and global errors, duplicate
IDs, unknown outcomes, retries, malformed input, mismatched counters or exit
codes are rejected. Raw artifact digest/attempt binding is checked first.
The consumer must protect discovery and the frozen plan independently; an
untrusted report pair cannot authenticate its own required scope or freshness.

Both inputs are limited to 4 MiB before parsing, with lexical depth 64 and
131072 structural/string tokens. At most 4096 cases and the existing 8 MiB
observation expansion budget apply before per-case identity hashing/cloning.
JSON reporter configuration/result metadata not used by this profile is ignored;
top-level schema fields are closed. No attachments are followed or executed.

`fixtures/playwright/capture.py` runs only the committed local fixture, with
fresh work directories, pipe limits and deadlines. It captures native discovery,
JSON and stdout/stderr separately with hashes and exact commands. The ten cases
include pass, fail, skip, zero, filtered partial, native timeout, missing report,
collector-truncated malformed input, externally killed timeout and duplicate
parameter titles. Malformed input is **a labeled collector fault**: the intact
native report is also retained, and no manually authored XML/JSON is described
as a native output. The missing-report case makes the actual native reporter
fail to write. The external timeout waits until the real browser test body
starts. A child-only subreaper kills and reaps adopted detached browser processes;
this is fixture lifecycle cleanup, not isolation for arbitrary candidate code.

Regenerate using `python3 fixtures/playwright/capture.py <output-directory>`.
Run `cargo test --locked --offline --test extended_adapters -- --include-ignored`
to regenerate independently and parse the fresh matrix. Installed binaries must
match the declared versions. `captured/PROVENANCE.json` records source/tool hashes;
per-case `capture.json` records command, exit, interruption and artifact hashes.

Protocol references: [official JSON reporter source](https://raw.githubusercontent.com/microsoft/playwright/v1.62.1/packages/playwright/src/reporters/json.ts),
[reporters](https://playwright.dev/docs/test-reporters),
[browser installation/version guidance](https://playwright.dev/docs/browsers).

This is one partial slice of task 5.1. Vitest and Jest native qualification
remain outstanding; no complete task acceptance is claimed.
