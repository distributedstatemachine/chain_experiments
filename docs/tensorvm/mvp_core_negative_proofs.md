# TensorVM MVP Core Negative Proofs And Counterexamples

Status: documentation-only proof audit compiled from the current worktree.

Purpose: make the unsound cases explicit. A positive proof plan is not enough if the current Rust surface
still admits states that contradict the reviewed MVP theorem. This document records concrete witness
constructions that are accepted by the current reference core but cannot satisfy the v2 useful-verification
PoW consensus claim.

The target invariants that would kill these counterexamples are mapped in
[`mvp_core_v2_state_invariants.md`](mvp_core_v2_state_invariants.md).
The lifecycle seed model needed to kill `CEX-006` is specified in
[`mvp_core_receipt_lifecycle_seed_model.md`](mvp_core_receipt_lifecycle_seed_model.md).
The production-authentication boundary behind `CEX-007` is specified in
[`mvp_core_signature_authentication_boundary.md`](mvp_core_signature_authentication_boundary.md).
The root/encoding boundary behind `CEX-004` and block-level `checks_root` failures is specified in
[`mvp_core_canonical_encoding_commitment_model.md`](mvp_core_canonical_encoding_commitment_model.md).

This is not a code change and not a mechanized proof. It is a negative proof ledger for the formal manifest.

## Target Theorem Under Test

The reviewed MVP consensus theorem should eventually look like:

```text
If block B is finalized as a non-fallback v2 block, then:
  B.proposer is a registered validator eligible for the epoch,
  B.settled_receipt_set_root is the deterministic canonical selection from the parent state,
  B.checks_root recomputes from verification transcripts for that selected receipt set,
  H(pow_header(B) || B.nonce) < B.difficulty_target,
  stake-weighted validator finality signatures cover that valid block hash.
```

The current implementation cannot prove this theorem. Some parts are not merely missing; the current public
chain surface admits counterexamples.

## Current Theorem That Is Actually Supported

The current finality theorem is much narrower:

```text
If submit_block_vote finalizes block hash h, then enough unique registered validator stake signed h and h is
the hash of a block already present in LocalChain.blocks.
```

That theorem is useful, but it is not useful-verification PoW consensus.

## Counterexamples

### CEX-001: A Finalized Block Can Have No Useful-PoW Witness

Witness construction:

```text
Start from any chain state with enough registered validator stake.
Produce a block through LocalChain::produce_block.
Submit valid BlockVote records from validators representing the finality threshold.
```

Accepted by current core:

- `chain::blocks::produce` builds a `TensorBlock` with `job_root`, `receipt_root`,
  `attestation_root`, `state_root`, `reward_root`, and `randomness`.
- `chain::validation::submit_block_vote` checks validator registration, stake, signature, known block hash,
  duplicate votes, and the finality stake threshold.

Missing from the accepted block:

```text
settled_receipt_set_root
checks_root
difficulty_target
nonce
```

Why this disproves the broad theorem:

The finalized block has no field that could witness canonical settled-receipt selection, recomputable
block-level verification, or PoW target satisfaction. Therefore:

```text
current_finalized(B) -> useful_verification_pow_valid(B)
```

is false over the current block type.

Repair gate:

Add a v2 block validity predicate and require it before block admission, vote admission, and finality
accounting.

### CEX-002: A Produced Block Does Not Imply Proposer Eligibility

Witness construction:

```text
Choose an arbitrary address A that is not a registered validator.
Call LocalChain::produce_block(A, timestamp).
Have enough registered validators vote for the resulting block hash.
```

Accepted by current core:

- `LocalChain::produce_block` accepts an `Address` and returns a `TensorBlock`; it does not return a
  `Result` and does not check whether the address is a registered validator.
- `submit_block_vote` checks the voters, not the block proposer.

Why this disproves the broad theorem:

The reviewed v2 theorem requires the proposer to be a registered validator that won useful-verification PoW.
The current finality path can finalize a block whose proposer is not proven eligible by the chain transition.

Repair gate:

Make block production/admission a fallible transition that validates proposer eligibility against the parent
state. For v2, proposer eligibility must be tied to the useful-verification PoW predicate, not merely address
presence.

### CEX-003: TensorWork Miner Proposer Selection Contradicts v2

Witness construction:

```text
State has at least one miner with settled_tensor_work > 0.
That miner is not also a registered validator.
Call proposer_for_next_epoch(beacon).
```

Accepted by current core:

- `chain::proposer::for_next_epoch` sums miner `settled_tensor_work`.
- When total work is nonzero, it selects from miners weighted by settled TensorWork.
- Validator stake fallback is used only when total miner work is zero.

Why this disproves the broad theorem:

The reviewed MVP says TensorWork affects miner rewards, blockspace accounting, and telemetry only. It does
not grant block-production eligibility. A miner selected because of TensorWork is not the same object as a
validator that won useful-verification PoW.

Repair gate:

Retire TensorWork proposer selection from the normal v2 path. Keep it only as explicitly labeled v1
reference behavior if needed for migration tests.

### CEX-004: A Receipt Map Root Is Not Canonical Blockspace

Witness construction:

```text
Parent state has settled receipts r1 and r2.
The v2 blockspace caps allow only one receipt.
Current TensorBlock commits receipt_root over the global receipt map, not the selected receipt set.
```

Accepted by current core:

- `chain::blocks::produce` commits `receipt_root(&chain.state.receipts)`.
- `ChainState` has `settled_receipts: BTreeSet<Hash>`, but `TensorBlock` does not commit a
  `settled_receipt_set_root`.
- The block type does not identify which settled receipts were selected, spent, carried over, or excluded by
  caps.

Why this disproves the broad theorem:

The v2 theorem needs deterministic inclusion and omission rules. A global receipt map root can be identical
while the intended v2 selected set is undefined at the block level. Validators cannot prove that the block
used the canonical receipt set because the block does not name that set.

Repair gate:

Add settled-receipt pool metadata, deterministic selector rules, blockspace caps, spent/carry-over state,
and a block-level `settled_receipt_set_root`.

### CEX-005: Quorum Is Syntactic Unless Verification Evidence Is Bound

Witness construction:

```text
Take an assigned validator for receipt r.
Have the validator sign an attestation with:
  result = Valid
  data_availability_passed = true
  checks_root = arbitrary hash
Submit enough such assigned-validator attestations to reach quorum.
```

Accepted by current core:

- `submit_attestation` checks validator registration, stake, signature, deterministic assignment, receipt
  existence, receipt metadata, and duplicate submissions.
- `has_attestation_quorum` counts unique assigned validators whose signed statement says
  `VerificationResult::Valid` and `data_availability_passed = true`.
- The chain admission path does not call `verify_tensor_op` or `verify_linear_training_step`, and it does
  not recompute the attestation `checks_root` from tensor artifacts.

Why this disproves the broad theorem:

The current theorem can honestly say:

```text
quorum -> enough assigned validators signed Valid/DataAvailable statements for the stored receipt
```

It cannot honestly say:

```text
quorum -> enough validators actually executed the verifier correctly
```

without additional assumptions or a block-level recomputation/challenge surface.

Repair gate:

Phrase current quorum proofs as syntactic. For v2, bind validator reports to recomputable check leaves,
require block-level `checks_root`, and define challenge openings or direct verification evidence.

### CEX-006: Validation Assignment Uses The Current Beacon, Not A Receipt-Lifecycle Seed

Witness construction:

```text
Receipt r is admitted while finalized_randomness = R0.
Blocks advance and finalized_randomness becomes R1 before all attestations arrive.
Submit an attestation for r after the randomness change.
```

Accepted by current core:

- `assigned_validators` calls `JobScheduler::assign_validators` with
  `chain.state.finalized_randomness`.
- `LocalChain::validation_seed(receipt_id)` also derives from the current `finalized_randomness`.
- Receipt state does not store the validation seed fixed at admission.

Why this disproves the lifecycle theorem:

The validator set for the same receipt can be evaluated against a later beacon. A validator assigned under
R0 can be rejected under R1, and a validator not assigned under R0 can be accepted under R1. That is
deterministic for the current state, but it is not a stable receipt-lifecycle assignment.

Repair gate:

Store the validation seed or assignment epoch at receipt admission, and use that value for attestation
admission, verifier challenges, and quorum counting until the receipt expires.

### CEX-007: Reference Signatures Do Not Establish Production Authentication

Witness construction:

```text
Anyone who knows an address and message can compute the current reference signature relation.
```

Accepted by current core:

- `types::sign` is a deterministic hash helper.
- `verify_signature` recomputes the same helper relation.

Why this disproves the production-authentication theorem:

The current signature flow can test message binding and plumbing. It does not prove private-key ownership,
unforgeability, replay-domain separation, or key custody.

Repair gate:

Keep the current signature theorem assumption-bound until production cryptography is wired and modeled.

### CEX-008: A Valid Nonce Does Not Prove Useful-Work Dominance

Witness construction for an insufficient future repair:

```text
Add difficulty_target and nonce to a v2-like block.
Let checks_root be valid but already known, cached, shared, empty, or cheap to compute.
Set difficulty so expected nonce search dominates transcript acquisition.
Mine H(pow_header || nonce) < difficulty_target.
```

Accepted by the insufficient repair:

- The hash predicate can be true for the committed header.
- The `checks_root` can be a valid commitment to verification evidence.
- The proposer may still have spent most of its resource on ordinary nonce grinding, or may have obtained
  the transcript from another party.

Why this disproves the useful-work-dominance theorem:

The nonce proves target success over bytes. It does not prove that useful verification was the dominant
block-production cost, and it does not prove the winning proposer personally performed verification. That
stronger claim needs an explicit work model, target bounds, transcript-acquisition assumptions, and
conservative wording.

Repair gate:

Separate structural useful-PoW validity from economic useful-work dominance. Add a nonfallback work floor,
parent-state target validation, retarget bounds, and measured cost assumptions as specified in
[`mvp_core_useful_pow_work_model.md`](mvp_core_useful_pow_work_model.md).

## Manifest Corrections Required

The formal manifest should interpret current claims this way:

| Area | Correct Current Claim | Incorrect Claim To Reject |
| --- | --- | --- |
| Finality | Enough registered validator stake signed a known v1 block hash. | Finality implies useful-verification PoW validity. |
| Block production | A reference block can be appended with a supplied proposer address. | Produced block implies eligible v2 proposer. |
| Proposer selection | v1 reference selector can choose miners by settled TensorWork. | TensorWork-selected miner is a v2 proposer. |
| Receipt roots | Blocks commit the global receipt map root. | Receipt map root is deterministic settled-receipt blockspace. |
| Attestation quorum | Quorum counts assigned signed Valid/DataAvailable statements. | Quorum proves validators actually ran the verifier. |
| Validation seed | Assignment is deterministic from current finalized randomness. | Assignment is stable for the receipt lifecycle. |
| Signatures | Reference signing tests message-flow shape. | Reference signing proves production authentication. |
| Useful-PoW work | A valid nonce can prove hash-target success over a validated header. | A valid nonce proves useful-work dominance or proposer-local verification. |

## Proof Upgrade Order

The minimum proof repair order is:

1. Define v2 block fields and canonical settled-receipt blockspace.
2. Make block production/admission fallible and validate proposer eligibility.
3. Bind useful-verification PoW to `settled_receipt_set_root` and `checks_root`.
4. Require vote/finality admission to depend on v2 block validity.
5. Store a receipt-lifecycle validation seed or equivalent immutable assignment anchor.
6. Downgrade quorum theorem language to syntactic until verifier evidence is recomputable or challengeable.
7. Replace reference signatures with a production signature scheme before making authentication claims.
8. Discharge the useful-PoW work model before claiming useful-work dominance.

## Current Judgment

The current verifier algorithms are still the strongest part of the core. The current consensus object is
not only incomplete; it admits finalized states that are outside the reviewed MVP theorem. Until the
counterexamples above are impossible by construction, the core must remain classified as not sound for the
full v2 MVP.
