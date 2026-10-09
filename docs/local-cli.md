# Experimental local CLI

This CLI evaluates explicitly labelled local fixtures. It does not authenticate a producer, validate real repository object IDs, grant execution/merge permission or emit GuardRunEnvelope. Protected production consumers must reject this profile.

| Command | Input | Output and exit |
|---|---|---|
| `doctor` | absolute entries of PATH | JSON executable-presence diagnostics, 0; never invokes tools or writes files |
| `plan OBLIGATIONS BINDING` | strict local JSON | validated frozen local plan, 0 |
| `check PLAN ATTEMPT [CHANGES]` | frozen plan, attempt and optional weakening flags | advisory domain JSON; ALLOW=0, BLOCK=2, REQUIRE_APPROVAL=3 |
| `coverage PLAN ATTEMPT` | same validated evidence | coverage JSON, 0 even when coverage is partial |
| `verify PLAN ATTEMPT REPORT [CHANGES]` | archived report plus original inputs | recomputed domain JSON and decision exit; stderr clarifies consistency is not authenticity |
| `run` | no supported execution profile yet | stderr refusal and 4; no candidate code launched |

All invalid arguments/input/verification failures produce stderr diagnostics, empty stdout and exit 4. No pre-binding error fabricates an envelope. Library statement/branch metrics remain separate from required execution and obligation counts. A zero denominator yields `Metric::fraction() == None`; serialized counts remain explicit. Missing required instances and unfinished attempts block. Caller-supplied weakening flags request review; they are not automated semantic weakening detection or approval records.

`cargo run --example generate_schemas` regenerates Draft-7 shape schemas. `python3 examples/validate_schemas.py` uses jsonschema 4.26.0 in the development environment. Rust parsers additionally enforce version, cardinality, references, digests and frozen matrix integrity; JSON Schema alone cannot establish cross-document authority.
