# Experimental local CLI

This CLI evaluates explicitly labelled local fixtures. It does not authenticate a producer, validate real repository object IDs, grant execution/merge permission or grant production eligibility. The original commands do not emit GuardRunEnvelope; the opt-in engine commands below emit explicitly local fixture bundles. Protected production consumers must reject this fixture capability.

| Command | Input | Output and exit |
|---|---|---|
| `doctor` | absolute entries of PATH | JSON executable-presence diagnostics, 0; never invokes tools or writes files |
| `plan OBLIGATIONS BINDING` | strict local JSON | validated frozen local plan, 0 |
| `check PLAN ATTEMPT [CHANGES]` | frozen plan, attempt and optional weakening flags | advisory domain JSON; ALLOW=0, BLOCK=2, REQUIRE_APPROVAL=3 |
| `coverage PLAN ATTEMPT` | same validated evidence | coverage JSON, 0 even when coverage is partial |
| `verify PLAN ATTEMPT REPORT [CHANGES]` | archived report plus original inputs | recomputed domain JSON and decision exit; stderr clarifies consistency is not authenticity |
| `run` | no supported execution profile yet | stderr refusal and 4; no candidate code launched |

For the original commands above, invalid arguments/input/verification failures produce stderr diagnostics, empty stdout and exit 4. No pre-binding error fabricates an envelope. Library statement/branch metrics remain separate from required execution and obligation counts. A zero denominator yields `Metric::fraction() == None`; serialized counts remain explicit. Missing required instances and unfinished attempts block. Caller-supplied weakening flags request review; they are not automated semantic weakening detection or approval records.

`cargo run --example generate_schemas` regenerates Draft-7 shape schemas. `python3 examples/validate_schemas.py` uses jsonschema 4.26.0 in the development environment. Rust parsers additionally enforce version, cardinality, references, digests and frozen matrix integrity; JSON Schema alone cannot establish cross-document authority.

## Coverage counting units

`execution.denominator` is the number of **distinct required `(test_id, environment)` pairs**; its numerator counts those pairs with a completed passing observation in a successfully finished attempt. A test shared by multiple obligations counts once per environment, and extra unrequired passes do not count. `obligations.denominator` counts required obligations; an obligation contributes to its numerator only when all of its required edges are satisfied.

`missing` and `unsatisfied` retain **obligation/test/environment edges**, so one missing shared execution can produce multiple traceable obligation gaps. For example, O1 and O3 both require case1 on Linux and Windows, while O2 requires case2 on both environments. The plan has six edges and four unique executions. Missing case2/Windows yields execution3/4 and obligations2/3. Missing shared case1/Windows yields execution3/4 and obligations1/3, with two missing edges for O1/O3. Both results BLOCK.

## Opt-in engine fixture commands

- `check-engine PLAN INVOCATION ATTEMPT [CHANGES [ADVICE]]` recomputes the domain result and evaluates actual GuardEngine contracts/facts. `INVOCATION` is a strict `FixtureInvocation` JSON object with capability `testguard.engine-fixture/v1`, run_id, full GuardEngine binding, started_at and finished_at. Candidate/base/source/baseline/repository/requirements must match PLAN; full OID syntax is checked by GE. This does not prove Git object existence or authenticate the supplied context. CHANGES and ADVICE are optional JSON arrays, resolved before binding.
- Success prints a `FixtureBundle` JSON object containing the actual `guard.integration/v1alpha1` envelope and an inline map of artifact URI to exact byte arrays. Completed ALLOW/BLOCK/REQUIRE_APPROVAL exits0/2/3. `contract.json`, `facts.json`, `report.json`, `domain.json`, `inputs.json` and `plan.json` are referenced and preserved. The fixed TestGuard mapping uses only the existing strict `guard.partme.ai/v1alpha1` fields and exact `forbid_relation` assertions.
- Invalid arguments/plan/invocation/capability/scope before binding produce stderr only, exit4. Unreadable or malformed ATTEMPT after binding produces an error envelope with null decision, retained available input and exit4. The library also supports explicit cancelled fixture outcomes with null decision/4. There is no CLI cancellation/execution promise and `run` still refuses.
- `verify-engine PLAN BUNDLE` checks referenced bytes, calls real GE report recomputation, checks binding and actual scope, and recomputes the domain projection from the archived inputs. It prints the validated envelope and exits with its technical decision. Stderr explicitly states this does not authenticate producer, approval or Git objects. Error/cancelled bundles cannot be verified as completed reports.

These are consistency and fixture interoperability interfaces. They do not resolve approval providers, create production eligibility, fetch artifact URIs, run tests, perform Git writes or publish checks. Completed bundles obey GE artifact/recomputation budgets; failed-input preservation is not a bounded production evidence store. Full parser/CLI resource isolation remains task3.3.
