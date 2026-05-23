# TensorVM MVP Core Useful-PoW Work Model

Status: documentation-only work and economics model for the blocked v2 consensus proof.

Purpose: define what the phrase "useful-verification PoW" would have to mean before it can support a
formal MVP-core soundness claim. This document separates three things that are easy to blur:

1. A structural hash predicate over a validated block header.
2. A recomputable verification transcript bound into that header.
3. An economic or cost claim that useful verification materially dominates ordinary nonce grinding.

Only the first two can plausibly become ordinary safety theorems over committed chain state. The third is a
parameterized economic assumption backed by measurement and threat-model choices; it is not proved merely
by adding a nonce, a target, or a `checks_root`.

## Current Verdict

The current implementation does not implement useful-verification PoW.

Current blockers:

- Current blocks do not carry `settled_receipt_set_root`, `checks_root`, `difficulty_target`, or `nonce`.
- Current block production advances the chain without a PoW predicate.
- Current finality votes do not import v2 block validation.
- Current proposer selection can use the superseded TensorWork path or caller-supplied proposer input.
- Current receipt roots are roots of current-state maps, not canonical selected settled-receipt blockspace.

Even after those state fields exist, a second issue remains: a valid hash predicate proves that someone found
a nonce for a committed header. It does not by itself prove that nonce search was economically dominated by
useful verification, or that the winning proposer personally performed all verification work rather than
using a cached or shared transcript.

The safe current wording is:

```text
useful-verification PoW is a blocked v2 target.
```

The unsafe wording is:

```text
adding a nonce over a checks root proves useful work.
```

## Model Objects

Let:

- `S` be the parent chain state.
- `B` be a candidate non-fallback v2 block.
- `R(S, B)` be the deterministic selected settled-receipt list for the parent state, beacon, and blockspace
  caps, as specified in
  [`mvp_core_settled_receipt_blockspace_model.md`](mvp_core_settled_receipt_blockspace_model.md).
- `root_R(S, B)` be the canonical root of `R(S, B)`.
- `L(S, B)` be the ordered list of recomputable verification check leaves for `R(S, B)`.
- `root_L(S, B)` be the canonical root of `L(S, B)`.
- `EncPow(B)` be the injective pre-hash encoding of the PoW header fields.
- `H` be the modeled collision-resistant/random-oracle-like hash used for PoW.
- `target_valid(S, B.difficulty_target)` be the target and retarget validity predicate.
- `registered_validator(S, B.proposer)` be the proposer eligibility predicate before PoW success.

The PoW header must bind exactly the fields validated by the v2 block predicate:

```text
pow_header(B) =
  EncPow(
    parent_hash,
    settled_receipt_set_root,
    checks_root,
    beacon,
    proposer,
    difficulty_target,
    params_version
  )
```

The `params_version` field is included in the model so future changes to check formats, work weights, or
difficulty rules cannot silently reuse the same header meaning. If the implementation represents this
through another committed version field, the theorem should name that field instead.

## Structural Validity Predicate

The structural predicate is the part that can become a conventional consensus safety theorem:

```text
valid_useful_pow_structural(B, S) =
  parent_valid(S, B.parent_hash)
  && registered_validator(S, B.proposer)
  && canonical_selected_receipts(S, B) = R(S, B)
  && B.settled_receipt_set_root = root_R(S, B)
  && recomputable_check_leaves(S, B) = L(S, B)
  && B.checks_root = root_L(S, B)
  && target_valid(S, B.difficulty_target)
  && nonfallback_work_floor(S, B)
  && hash_to_uint(H(pow_header(B) || B.nonce)) < B.difficulty_target
```

`nonfallback_work_floor` is required because an empty selected set or trivially cheap transcript turns the
normal path into ordinary nonce PoW. Zero-receipt or no-PoW liveness belongs in the explicit fallback rule,
with reduced rewards and no claim of useful verification work.

This predicate proves only header correctness, selected-receipt binding, check-root binding, target
validity, proposer registration, and nonce success. It does not prove the economic dominance claim below.

## Economic Work Predicate

Define the verification-work and nonce-work accounting functions as model parameters:

```text
verification_work(S, B) =
  sum(check_cost(leaf) for leaf in L(S, B))

expected_nonce_work(B) =
  expected_hash_trials(B.difficulty_target) * hash_cost(params_version)

transcript_acquisition_work(S, B, A) =
  adversary-specific cost for actor A to obtain L(S, B)
```

The critical term is `transcript_acquisition_work`, not just `verification_work`.

If check leaves or roots are public, cached, cheaply copied, or computed once by another party, then a
proposer may mine over a valid verification commitment without personally performing the full verification.
That may still be acceptable if the protocol claim is "blocks commit to reproducible verification evidence,"
but it is not the stronger claim "the winning proposer spent most of its block-production resource on useful
verification."

A useful-work dominance assumption must therefore be stated separately:

```text
useful_work_dominates(S, B, A, alpha) =
  transcript_acquisition_work(S, B, A) >= alpha * expected_nonce_work(B)
```

or, if the design wants nonce search to be a small tie-breaker after verification:

```text
expected_nonce_work(B) <= beta * verification_work(S, B)
```

for an explicitly chosen `beta < 1`.

Neither inequality follows from `valid_useful_pow_structural`. It requires:

- cost measurements for the target hardware classes,
- difficulty parameters and retarget bounds,
- a position on transcript sharing and caching,
- a policy for empty or low-work selected sets,
- and wording that does not pretend economic evidence is a cryptographic proof.

If these measurements are missing, the safe claim is ordinary PoW over a verification commitment, not
economically useful-verification PoW.

## Theorem Split

Future proof artifacts should not collapse these obligations into one theorem.

| ID | Target Theorem | Kind | Blocker Today |
| --- | --- | --- | --- |
| UPOW-001 | `pow_header` encoding is injective before hashing and includes exactly the validated fields. | Formalizable plus hash assumption. | v2 header object is missing. |
| UPOW-002 | The selected receipt root equals the canonical selected settled-receipt set for parent state. | Implementation-dischargeable. | Canonical selected blockspace is missing. |
| UPOW-003 | The checks root equals the ordered recomputable check leaves for the selected receipts. | Implementation-dischargeable. | Block-level check leaves/root are missing. |
| UPOW-004 | A non-fallback block cannot claim useful-PoW with an empty or below-floor verification transcript. | Implementation plus parameter assumption. | No v2 work-floor rule exists. |
| UPOW-005 | The target is valid for the parent difficulty state and retarget bounds. | Implementation-dischargeable. | Difficulty state and validation are missing. |
| UPOW-006 | The nonce satisfies the target for the exact validated header. | Implementation-dischargeable plus hash model. | Current block has no target/nonce predicate. |
| UPOW-007 | The proposer is a registered validator and TensorWork is excluded from proposer eligibility. | Implementation-dischargeable. | Current proposer path is contradicted by v1/reference behavior. |
| UPOW-008 | Finality vote admission imports `valid_useful_pow_structural`. | Implementation-dischargeable. | Current votes check known current block hashes only. |
| UPOW-009 | Useful verification materially dominates nonce grinding under chosen parameters. | Economic/evidence-bound. | No committed cost model or measurements discharge this. |
| UPOW-010 | Proposer-local verification is guaranteed, if that stronger claim is desired. | Extra protocol assumption or mechanism. | Plain roots and nonce search do not prove who computed the transcript. |

The minimal safety theorem for v2 finality can depend on UPOW-001 through UPOW-008. A public claim that the
chain's block-production resource is mostly useful work needs UPOW-009, and any claim that the winning
proposer itself performed the verification needs UPOW-010 or narrower wording.

## Transferability Caveat

Verification transcripts are usually transferable data. If one validator computes a valid `checks_root` for
the canonical selected set and shares it, another validator can mine over the same header without repeating
the full verification, unless the protocol adds a non-transferability mechanism.

Possible design positions:

| Position | Safe Claim | Required Mechanism Or Wording |
| --- | --- | --- |
| Shared transcripts allowed | Blocks are PoW over reproducible verification commitments. | Do not claim proposer-local useful work. Measure network-level verification, not winner-local verification. |
| Proposer-local work desired | The winning proposer performed the verification work. | Requires a mechanism beyond ordinary hash roots, such as non-transferable challenges, proof-carrying verifier execution, trusted hardware, slashing/challenge games with evidence, or an explicit honesty assumption. |
| Nonce as tie-breaker | Verification is the main candidate-construction cost and nonce search selects among verified candidates. | Keep expected nonce work below a measured fraction of verification work and retarget with floors/ceilings. |
| Ordinary PoW fallback | Liveness continues when no useful selected set exists. | Use explicit fallback validity and reduced rewards; do not count it as useful-PoW. |

This caveat is not a reason to abandon the model. It is the line between a defensible theorem and marketing
language.

## Difficulty And Retarget Requirements

A future difficulty state must be validated as consensus state, not accepted from the candidate block.

Minimum fields or equivalent state:

```text
difficulty_target
retarget_epoch
target_block_time
min_target
max_target
max_retarget_ratio
work_floor
params_version
```

Required checks:

1. `B.difficulty_target` equals the value derived from the parent difficulty state.
2. Retargeting is bounded by `max_retarget_ratio`.
3. `difficulty_target` cannot collapse to trivial nonce search.
4. `work_floor` prevents normal blocks from claiming useful work over empty or tiny selected sets.
5. Fallback blocks use a separate validity predicate and reward rule.
6. Any telemetry-derived parameter update is either not consensus-critical or is included in a governed,
   deterministic state transition.

Without these checks, a block can satisfy a local hash predicate while using an invalid, trivial, or
unreviewed target.

## Current Code Evidence Boundary

The current repository can support these narrow statements:

- Verifier-local TensorOp and LinearTrainingStep checks have proof-ready algebraic obligations under
  explicit probabilistic assumptions.
- Current roots can commit to their current encoded objects under hash assumptions.
- Current chain admission can enforce syntactic validator assignment, signatures, duplicate prevention, and
  current quorum rules.

The current repository cannot support these useful-PoW statements:

- A current block has valid useful-verification PoW.
- A current finalized block implies `valid_useful_pow_structural`.
- The current proposer was selected by registered-validator useful-PoW.
- TensorWork is excluded from block proposer eligibility in implementation.
- Verification work dominates nonce grinding under measured parameters.
- The winning proposer personally performed the selected receipt verification.

## Bad Assumptions Rejected

The following assumptions must be rejected in proof reviews:

1. Adding `nonce` and `difficulty_target` to the current block shape is enough.
2. A `checks_root` proves useful work even if it is not recomputable from canonical selected receipts.
3. A valid nonce proves the winning proposer performed verification work.
4. TensorWork can remain a proposer-selection input in the v2 normal path.
5. Synthetic tensor jobs prove externally demanded useful work.
6. A verification-to-execution ratio proves useful-PoW economics without nonce-work and transcript-sharing
   analysis.
7. Validator attestations reduce proposer PoW cost or improve PoW safety unless the block predicate imports
   their recomputable evidence.
8. Fallback or empty blocks can be counted as useful-verification PoW.
9. Difficulty target values can be trusted from the block without parent-state validation.

## Discharge Gate

Do not classify useful-verification PoW as locally proof-ready until all of these are true:

1. The v2 block object commits parent hash, selected receipt root, checks root, beacon, proposer,
   difficulty target, nonce, and parameter version or equivalent.
2. The selected receipt root is recomputed from deterministic parent-state blockspace rules.
3. The checks root is recomputed from ordered check leaves for exactly the selected receipts.
4. The target and retarget rules are validated from parent difficulty state.
5. The nonce predicate hashes exactly the validated header.
6. The proposer is a registered validator and TensorWork is excluded from normal proposer eligibility.
7. Finality vote admission rejects blocks that fail the full v2 validity predicate.
8. Empty or below-floor selected sets use explicit fallback, not useful-PoW rewards.
9. Cost parameters document the relationship between transcript acquisition, verification work, and nonce
   search.
10. Public wording says whether the claim is structural PoW over verification commitments, network-level
    useful verification, or proposer-local useful work.

## Current Judgment

The useful-PoW proof is blocked in two layers.

First, the implementation does not expose the state needed for the structural predicate. Second, the
economic "useful" claim requires a cost model and explicit stance on transcript sharing. The proof corpus
should therefore treat useful-verification PoW as a target theorem plus an economic assumption, not as a
property of the current chain.
