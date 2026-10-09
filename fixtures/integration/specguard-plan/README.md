# Actual SpecGuard export, local authority fixture

PROVENANCE.json pins SpecGuard e0c75597, GitGuard95eb3d9 and GuardEngine6527e2a, original byte hashes, exact source candidate and expected export/baseline/source/scope. The committed source contains three acceptances over two requirements. `source.bundle` preserves the real Git commit; `source.md` is its specs/plan.md.

To reproduce, unpack those three exact Git source archives as siblings, copy generator.rs to the SpecGuard examples directory as generate_testguard_plan.rs, restore source.bundle into a fresh repo, and run the command recorded in PROVENANCE.json with that repository and a fresh output directory. The generator asserts git status is clean, reads/freeze/parses actual files, invokes the explicit fixture ApprovalValidationPort at time50 and exports obligations. Initial generator used a nonexistent SourceSnapshot.clean field and failed compilation; the recorded final source correctly checks actual git status and successfully generated these bytes. Neither that compile error nor fixture authority is runtime candidate authorization.

This is actual producer/consumer interoperability over local fixture authority. It does not prove production approval, trusted host execution, native tests on both environments, or merge eligibility.
