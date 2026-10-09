# Same-run local fixture freshness

The protected controller supplies FreshnessContext independently of the output:
full invocation, weakening/advice, GE EligibilityPolicy, declared configuration
digest, expected coverage and purpose-to-approval references. Completion's
crate-private verify_work compares plan/invocation/weakening/advice against its
original scheduler full-work digest, including producer and scheduler limits.
This cannot be built from uploaded JSON or by recomputing a claimed report hash.

FrozenFreshness validates the original bundle with the existing domain/GE
verifier, compares actual binding/producer/analyzer/contract/coverage against
protected expectations, and retains its exact serialized bytes. Its immutable
versioned key includes the whole plan, invocation including run ID, policy
content (not merely a caller-supplied policy hash), producer/analyzer versions,
configuration, coverage, approval purposes/references and original byte digest.
The plan also carries native baseline and policy digests. Any current key change
requires a rerun. Cross-run result reuse is always disabled in this profile;
there is no cache-enable switch.

Configuration here is controller-declared metadata. The fixture transport does
not establish native command configuration, so this module makes no such claim.
Neither protected context nor freshness receipts are derived automatically from
Completion. Controller protection remains an explicit deployment assumption.

Every check and receipt refresh calls AuthorityProvider.verify_approval for every
purpose required by the frozen protected policy, then GE validate_approval_record
for binding/action/contract/principal/purpose and active lifetime. Reference lists
must exactly cover policy purposes; changing either list invalidates the old key.
No approval record or validation result is cached. A same-reference revocation,
expiry or unavailable provider rejects. Receipt refresh rejects clock rollback.
The returned private, non-deserializable receipt only records these freshness
conditions. It does not authenticate the producer, evaluate a new technical gate,
change coverage, waive failure or grant a Git operation.

The caller must still call EvidenceStore.consume with current access/time for
required artifact availability and retention, and use the external protected
eligibility boundary for trusted consumption. Historical bytes retained here are
an audit snapshot, never a bypass for deleted/expired storage artifacts. A new
FrozenFreshness alone cannot reactivate an abandoned scheduler request or change
which request is current. Scheduler current checks remain independently required.

Borrowed counting serialization caps plan/context at256KiB before hashing or
cloning. Approval purposes cap32, each principal set64; coverage/scope arrays4096;
changes/advice64. Native plan preflight remains active. Original completion bundle
caps4MiB before validation/copy. These are local resource budgets, not OS process
isolation, global cache management or production authority. Freeze accepts only a
completed verifiable fixture bundle; failed/cancelled histories remain with the
scheduler/store and are never promoted through this freshness path.
