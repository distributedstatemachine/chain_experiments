# TensorVM MVP Core Negative Proofs And Counterexamples

Status: documentation-only proof audit compiled from the current worktree.

Purpose: make the unsound cases explicit. A positive proof plan is not enough if the Rust surface admits
states that contradict the reviewed MVP theorem. This document records concrete witness constructions,
including historical counterexamples already discharged by the current local reference path and remaining
gaps that still cannot satisfy the full v2 useful-verification PoW consensus claim.

The target invariants that would kill these counterexamples are mapped in
[`mvp_core_v2_state_invariants.md`](mvp_core_v2_state_invariants.md).
The lifecycle seed model needed to kill `CEX-006` is specified in
[`mvp_core_receipt_lifecycle_seed_model.md`](mvp_core_receipt_lifecycle_seed_model.md).
The production-authentication boundary behind `CEX-007` is specified in
[`mvp_core_signature_authentication_boundary.md`](mvp_core_signature_authentication_boundary.md).
The root/encoding boundary behind `CEX-004` and block-level `checks_root` failures is specified in
[`mvp_core_canonical_encoding_commitment_model.md`](mvp_core_canonical_encoding_commitment_model.md).
The reward-finality boundary behind `CEX-010` is specified in
[`mvp_core_reward_finality_challenge_model.md`](mvp_core_reward_finality_challenge_model.md).
The difficulty target boundary behind `CEX-011` is specified in
[`mvp_core_difficulty_retarget_model.md`](mvp_core_difficulty_retarget_model.md).

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

The current implementation still cannot prove this theorem end to end. The local block path now discharges
the earlier block-shape, validator-proposer, useful-PoW nonce, vote-admission, and one-shot included-receipt
counterexamples, but parent-state snapshots, challenge openings, difficulty retargeting, exact receipt
lifecycle metadata, and live validator proposer networking remain open.

## Current Theorem That Is Actually Supported

The current local finality theorem is narrower:

```text
If submit_block_vote finalizes block hash h through the local reference path, then enough unique registered
validator stake signed h, h is the hash of a known `Chain` block, and the block passes the local
useful-PoW validity predicate against a reconstructed parent-like state view.
```

That theorem is useful, but it is still not the full production useful-verification PoW consensus theorem.

## Counterexamples

### CEX-001: A Finalized Block Can Have No Useful-PoW Witness

Status: discharged for the local reference block path; retained as historical negative proof.

Former witness construction:

```text
Start from any chain state with enough registered validator stake.
Produce a block through LocalChain::produce_block.
Submit valid BlockVote records from validators representing the finality threshold.
```

Formerly accepted by the core:

- `chain::blocks::produce` built a `TensorBlock` with `job_root`, `receipt_root`,
  `attestation_root`, `state_root`, `reward_root`, and `randomness`.
- `chain::validation::submit_block_vote` checked validator registration, stake, signature, known block hash,
  duplicate votes, and the finality stake threshold.

Missing from the accepted block:

```text
settled_receipt_set_root
checks_root
difficulty_target
nonce
```

Why this disproves the broad theorem:

The old finalized block had no field that could witness canonical settled-receipt selection, recomputable
block-level verification, or PoW target satisfaction. Therefore the historical implication

```text
current_finalized(B) -> useful_verification_pow_valid(B)
```

was false over the old block type.

Current local repair:

`TensorBlock` now carries `settled_receipt_set_root`, `checks_root`, `beacon`, `difficulty_target`, and
`nonce`; local production mines a useful-PoW nonce, and vote admission calls block validation before counting
stake. Remaining work is exact parent-state persistence and challenge-openable verification evidence.

### CEX-002: A Produced Block Does Not Imply Proposer Eligibility

Status: discharged for the local reference block path; retained as historical negative proof.

Witness construction:

```text
Choose an arbitrary address A that is not a registered validator.
Call LocalChain::produce_block(A, timestamp).
Have enough registered validators vote for the resulting block hash.
```

Formerly accepted by the core:

- `LocalChain::produce_block` accepted an `Address` and returned a `TensorBlock`; it did not return a
  `Result` and did not check whether the address was a registered validator.
- `submit_block_vote` checked the voters, not the block proposer.

Why this disproves the broad theorem:

The reviewed v2 theorem requires the proposer to be a registered validator that won useful-verification PoW,
so the old path could finalize a block whose proposer was not proven eligible by the chain transition.

Current local repair:

Block production is fallible, rejects unknown validators, and vote admission validates the block proposer.
The remaining production proof must still tie proposer networking and admission to the full useful-PoW
predicate across nodes.

### CEX-003: TensorWork Miner Proposer Selection Contradicts v2

Status: discharged for local proposer selection.

Witness construction:

```text
State has at least one miner with settled_tensor_work > 0.
That miner is not also a registered validator.
Call proposer_for_next_epoch(beacon).
```

Formerly accepted by the core:

- `chain::proposer::for_next_epoch` summed miner `settled_tensor_work`.
- When total work was nonzero, it selected from miners weighted by settled TensorWork.
- Validator stake fallback was used only when total miner work was zero.

Why this disproves the broad theorem:

The reviewed MVP says TensorWork affects miner rewards, blockspace accounting, and telemetry only. It does
not grant block-production eligibility. A miner selected because of TensorWork is not the same object as a
validator that won useful-verification PoW.

Repair gate:

Current local repair:

`chain::proposer::for_next_epoch` selects registered validators by stake and ignores miner TensorWork.

### CEX-004: A Receipt Map Root Is Not Canonical Blockspace

Status: partially discharged locally; full v2 lifecycle metadata remains open.

Witness construction:

```text
Parent state has settled receipts r1 and r2.
The v2 blockspace caps allow only one receipt.
Old TensorBlock committed `receipt_root` over the global receipt map, not the selected receipt set.
```

Formerly accepted by the core:

- `chain::blocks::produce` committed `receipt_root(&chain.state.receipts)`.
- `ChainState` had `settled_receipts: BTreeSet<Hash>`, but `TensorBlock` did not commit a
  `settled_receipt_set_root`.
- The block type did not identify which settled receipts were selected, spent, carried over, or excluded by
  caps.

Why this disproves the broad theorem:

The v2 theorem needs deterministic inclusion and omission rules. The local path now computes deterministic
settled-receipt selection, commits `settled_receipt_set_root`, records block-selected receipts as local
evidence, and marks selected receipts included so they are not selected again.

Remaining repair gate:

Persist exact parent-state snapshots or replayable transitions, promote selected-receipt metadata into the
canonical block/opening model, and add expiry, challenge-window, DA, and carry-over semantics.

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
- `Chain::validation_seed(receipt_id)` also derives from the current `finalized_randomness`.
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

### CEX-009: Aggregated Checks Roots Can Aggregate False Claims

Witness construction for an insufficient future repair:

```text
For a selected receipt r, assigned validators sign:
  result = Valid
  data_availability_passed = true
  checks_root = h_fake
Aggregate h_fake into a receipt-level or block-level checks root.
Do not require recomputation of the verifier transcript and do not provide a challenge opening path.
```

Accepted by the insufficient repair:

- The attestation signatures can be valid.
- The aggregate root can be deterministic and collision-resistant for the signed `checks_root` values.
- The block can bind the aggregate root exactly.

Why this disproves the semantic verifier-execution theorem:

The root proves commitment to signed check claims. It does not prove those claims were produced by
`verify_tensor_op`, `verify_linear_training_step`, or any approved verifier transcript. A false but signed
claim remains false after aggregation unless every leaf is recomputable, directly verified, or challengeable
within consensus state.

Repair gate:

Define a `CheckLeaf` and `VerifierTranscript` schema, bind attestation signatures to those leaves, and add
direct recomputation or challenge openings as specified in
[`mvp_core_verifier_evidence_model.md`](mvp_core_verifier_evidence_model.md).

### CEX-010: Finalized Blocks Can Pay Rewards Before Challenge Finality

Witness construction for an insufficient future repair:

```text
Build a v2-shaped block B with selected receipt r and checks_root h.
Finalize B with enough validator votes.
Immediately credit spendable proposer, miner, or validator rewards from h.
Later, within the intended verification challenge window, a challenger opens h and proves the check leaf for
r was false or unavailable.
```

Accepted by the insufficient repair:

- B can be finalized as an ordering object.
- `reward_root` can include balances derived from B.
- A later challenge can identify a bad `checks_root` leaf.

Why this disproves the reward-soundness theorem:

If verifier-dependent rewards are already spendable, the later challenge cannot deterministically claw back
the exact affected claims without adding more state and assumptions. Block finality proves ordering only; it
does not prove reward finality for evidence that is still challengeable.

Repair gate:

Represent verifier-dependent rewards as pending claims until direct recomputation or challenge-window
settlement. Add challenge admission, deterministic resolution, claim invalidation, clawback/nonpayment, and
single-use settlement as specified in
[`mvp_core_reward_finality_challenge_model.md`](mvp_core_reward_finality_challenge_model.md).

### CEX-011: A Block Can Use An Easy Self-Supplied Target

Witness construction for an insufficient future repair:

```text
Build a v2-shaped block B with valid selected receipt and checks roots.
Set B.difficulty_target to an easy numeric target chosen by the proposer.
Find nonce n such that H(pow_header(B) || n) < B.difficulty_target.
Validate only the hash inequality against B.difficulty_target.
```

Accepted by the insufficient repair:

- The PoW hash can be below the target field in B.
- The header can bind selected receipts, checks, beacon, proposer, and nonce.
- The block can look structurally useful-PoW-shaped.

Why this disproves target validity:

The nonce proves only success against the target that was checked. If the target is not derived from parent
consensus state, the proposer can choose an easy or locally configured target and convert useful-PoW into
ordinary cheap nonce search.

Repair gate:

Add parent-state `DifficultyState`, bounded retarget rules, target bounds, canonical hash-to-target
semantics, and work-floor policy as specified in
[`mvp_core_difficulty_retarget_model.md`](mvp_core_difficulty_retarget_model.md).

## Manifest Corrections Required

The formal manifest should interpret current claims this way:

| Area | Correct Current Claim | Incorrect Claim To Reject |
| --- | --- | --- |
| Finality | Local votes are counted only for known blocks that pass useful-PoW validation against a reconstructed parent-like state view. | Local finality proves the full production v2 consensus theorem. |
| Block production | Local block production is fallible and requires a registered validator proposer. | Local proposer checks prove live production proposer networking. |
| Proposer selection | Local proposer selection is validator-stake weighted and ignores miner TensorWork. | TensorWork-selected miner is a v2 proposer. |
| Receipt roots | Blocks commit deterministic selected settled receipts and mark selected receipts included once. | The local metadata-only selected-receipt map is a complete v2 opening/lifecycle model. |
| Attestation quorum | Quorum counts assigned signed Valid/DataAvailable statements. | Quorum proves validators actually ran the verifier. |
| Validation seed | Assignment is deterministic from current finalized randomness. | Assignment is stable for the receipt lifecycle. |
| Signatures | Reference signing tests message-flow shape. | Reference signing proves production authentication. |
| Useful-PoW work | A valid nonce can prove hash-target success over a validated header. | A valid nonce proves useful-work dominance or proposer-local verification. |
| Verifier evidence | An aggregate checks root can commit signed check claims. | An aggregate checks root proves those claims came from real verifier execution. |
| Reward finality | Finalized blocks may create pending verifier-dependent reward claims. | Block finality makes verifier-dependent rewards irreversible. |
| Difficulty target | A valid nonce proves success against a parent-state-derived target. | A target field validates itself. |

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
9. Discharge the verifier evidence model before claiming semantic verifier execution.
10. Discharge the reward-finality model before making verifier-dependent rewards spendable.
11. Discharge the difficulty-retarget model before claiming production useful-PoW target validity.

## Current Judgment

The current verifier algorithms are still the strongest part of the core. The current consensus object is
not only incomplete; it admits finalized states that are outside the reviewed MVP theorem. Until the
counterexamples above are impossible by construction, the core must remain classified as not sound for the
full v2 MVP.
