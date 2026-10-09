# Bounded local CLI contract

This CLI evaluates explicitly labelled local fixtures. It does not authenticate a producer, validate real repository object IDs, grant execution/merge permission or grant production eligibility. The original commands do not emit GuardRunEnvelope; the opt-in engine commands below emit explicitly local fixture bundles. Protected production consumers must reject this fixture capability.

| Command | Input | Output and exit |
|---|---|---|
| `doctor` | absolute entries of PATH | JSON executable-presence diagnostics, 0; never invokes tools or writes files |
| `plan OBLIGATIONS BINDING` | strict local JSON | validated frozen local plan, 0 |
| `check PLAN ATTEMPT [CHANGES]` | frozen plan, attempt and optional weakening flags | advisory domain JSON; ALLOW=0, BLOCK=2, REQUIRE_APPROVAL=3 |
| `coverage PLAN ATTEMPT` | same validated evidence | coverage JSON, 0 even when coverage is partial |
| `verify PLAN ATTEMPT REPORT [CHANGES]` | archived report plus original inputs | recomputed domain JSON and decision exit; stderr clarifies consistency is not authenticity |
| `check --engine PLAN INVOCATION ATTEMPT [CHANGES [ADVICE]]` | frozen fixture context plus attempt | actual GE-backed bundle, 0/2/3/4; same as `check-engine` |
| `verify --engine PLAN BUNDLE` | preserved fixture bundle | actual GE/domain recomputation, decision exit; same as `verify-engine` |
| `run [arguments...]` | no qualified general sandbox profile | structured stderr refusal and 4; no candidate code launched |

For the original commands above, invalid arguments/input/verification failures produce static JSON stderr diagnostics, empty stdout and exit 4. No pre-binding error fabricates an envelope. Library statement/branch metrics remain separate from required execution and obligation counts. A zero denominator yields `Metric::fraction() == None`; serialized counts remain explicit. Missing required instances and unfinished attempts block. Caller-supplied weakening flags request review; they are not automated semantic weakening detection or approval records.

`cargo run --example generate_schemas` regenerates Draft-7 shape schemas. `python3 examples/validate_schemas.py` uses jsonschema 4.26.0 in the development environment. Rust parsers additionally enforce version, cardinality, references, digests and frozen matrix integrity; JSON Schema alone cannot establish cross-document authority.

## Coverage counting units

`execution.denominator` is the number of **distinct required `(test_id, environment)` pairs**; its numerator counts those pairs with a completed passing observation in a successfully finished attempt. A test shared by multiple obligations counts once per environment, and extra unrequired passes do not count. `obligations.denominator` counts required obligations; an obligation contributes to its numerator only when all of its required edges are satisfied.

`missing` and `unsatisfied` retain **obligation/test/environment edges**, so one missing shared execution can produce multiple traceable obligation gaps. For example, O1 and O3 both require case1 on Linux and Windows, while O2 requires case2 on both environments. The plan has six edges and four unique executions. Missing case2/Windows yields execution3/4 and obligations2/3. Missing shared case1/Windows yields execution3/4 and obligations1/3, with two missing edges for O1/O3. Both results BLOCK.

## Opt-in engine fixture commands

- `check-engine PLAN INVOCATION ATTEMPT [CHANGES [ADVICE]]` recomputes the domain result and evaluates actual GuardEngine contracts/facts. `INVOCATION` is a strict `FixtureInvocation` JSON object with capability `testguard.engine-fixture/v1`, run_id, full GuardEngine binding, started_at and finished_at. Candidate/base/source/baseline/repository/requirements must match PLAN; full OID syntax is checked by GE. This does not prove Git object existence or authenticate the supplied context. CHANGES and ADVICE are optional JSON arrays, resolved before binding.
- Success prints a `FixtureBundle` JSON object containing the actual `guard.integration/v1alpha1` envelope and an inline map of artifact URI to exact byte arrays. Completed ALLOW/BLOCK/REQUIRE_APPROVAL exits0/2/3. `contract.json`, `facts.json`, `report.json`, `domain.json`, `inputs.json` and `plan.json` are referenced and preserved. The fixed TestGuard mapping uses only the existing strict `guard.partme.ai/v1alpha1` fields and exact `forbid_relation` assertions.
- Invalid arguments/plan/invocation/capability/scope before binding produce stderr only, exit4. Unreadable or malformed ATTEMPT after binding produces an error envelope with null decision, retained available input and exit4. The library also supports explicit cancelled fixture outcomes with null decision/4. There is no CLI cancellation/execution promise and `run` still refuses.
- `verify-engine PLAN BUNDLE` checks referenced bytes, calls real GE report recomputation, checks binding and actual scope, and recomputes the domain projection from the archived inputs. It prints the validated envelope and exits with its technical decision. A JSON `ConsistencyNotice` on stderr explicitly states this does not authenticate producer, approval or Git objects. Error/cancelled bundles cannot be verified as completed reports.

These are consistency and fixture interoperability interfaces. They do not resolve approval providers, create production eligibility, fetch artifact URIs, run tests, perform Git writes or publish checks. Completed bundles obey GE artifact/recomputation budgets and the CLI limits below. Task3.3 provides bounded native parsers and an explicitly invoked Linux artifact collector; task3.4 provides a separate local evidence store. CLI-selected read paths are not automatically authenticated or routed through those stores, and no filesystem or execution sandbox is claimed.

## Frozen grammar, resource limits and streams

The command name and optional `--engine` mode are resolved before files are opened.
Only the positional signatures above and the two original engine aliases are
accepted. At most 8 UTF-8 argv entries, each at most4096bytes and all together
at most16384bytes, are collected. NonUTF8 argv rejects with exit4, not a panic.
Doctor accepts at most16384bytes and128 PATH entries; it only checks presence
of its declared initial cargo/mvn tools and does not execute or version-attest them.

The bounded file-reader profile is Linux: open with NONBLOCK/NOFOLLOW, check a
regular file, then read at most the permitted size plus one growth-detection byte.
Final-component symlinks, directories, FIFOs and devices reject; this is not an
ancestry jail or a filesystem authorization provider. Unsupported file-reader
platforms reject explicitly. Inputs cap at1MiB each; `verify --engine`/`verify-engine`
bundles cap at32MiB because artifacts are embedded as byte arrays. JSON nesting
caps at64 and structural/string tokens at4194304 before parsing allocations.

A serialized output body is capped at32MiB and fully buffered before stdout,
then followed by one newline. A serialization/size failure emits no partial JSON
and exits4. Output I/O failure cannot undo bytes already accepted by the OS;
the CLI reports a static error and does not claim atomic pipe delivery.

Errors use `testguard.cli-diagnostic/v1`, fixed code/message and no echoed path,
unknown JSON field, raw snippet, binding or candidate. `PreBindingDiagnostic`
is used for invalid invocation/input or failed verification; `CliDiagnostic`
is used for output failure or a bound processing failure with no publishable
result. Successful verify writes one JSON `ConsistencyNotice` to stderr;
its recomputed JSON result is the only stdout value. All other successes leave
stderr empty. This notice is not authenticated producer/approval evidence.

After valid engine binding, an unavailable or oversized attempt becomes an
actual error/null-decision bundle with exit4. Such a bundle retains a small,
explicit refusal descriptor, not a claim that the unavailable/oversized original
bytes were preserved. In-budget malformed attempt bytes are retained exactly.
Input limits for PLAN/INVOCATION/CHANGES/ADVICE apply before binding, so those
failures have no envelope. A result too large to serialize never emits a completed
success or partial stdout; it produces only a static output diagnostic/4.

`coverage` retains its original query exit0 on valid partial evidence, with
unsatisfied counts preserved; this is not gate success. Both domain `check` and
actual engine `check --engine` return BLOCK/2 for ordinary partial evidence.
No command turns a local fixture capability into production authority or enables
arbitrary execution. `run` remains disabled until a reviewed sandbox profile and
explicit execution permission can be enforced.
