# TensorVM MVP Core Candidate v2 Block Audit

Status: documentation-only audit of the local dirty v2-block candidate observed on May 23, 2026.

Purpose: evaluate whether the current uncommitted v2-shaped block changes discharge the proof obligations in
the MVP core proof corpus. They do not. The candidate is meaningful progress, but it is not build-clean and
does not yet prove the reviewed v2 consensus theorem.

This document deliberately treats the dirty Rust files as candidate evidence only. It does not stage or
commit any code. Existing proof statuses remain conservative until the code is committed, build-clean,
tested adversarially, and mapped back into the formal proof manifest.

The parent-state transition theorem that this candidate still needs is specified in
[`mvp_core_parent_state_transition_model.md`](mvp_core_parent_state_transition_model.md).

## Evidence Scope

Observed dirty files:

```text
crates/tensor_vm/src/chain.rs
crates/tensor_vm/src/chain/blocks.rs
crates/tensor_vm/src/chain/commands.rs
crates/tensor_vm/src/chain/proposer.rs
crates/tensor_vm/src/chain/roots.rs
crates/tensor_vm/src/chain/state.rs
crates/tensor_vm/src/chain/validation.rs
crates/tensor_vm/src/lib.rs
crates/tensor_vm/src/localnet.rs
crates/tensor_vm/src/storage.rs
crates/tensor_vm/src/testnet.rs
docs/tensorvm/local_chain_production_exec_plan.md
```

Build check:

```text
cargo check -p tensor_vm --all-targets
```

Result: failed.

Failure summary:

```text
error[E0609]: no field `parent_hash` on type `Result<TensorBlock, TvmError>`
error[E0308]: expected `&TensorBlock`, found `&Result<TensorBlock, TvmError>`
```

The failing site is the storage block-log test path where `produce_block` now returns `Result<TensorBlock,
TvmError>` but the test still mutates and encodes the value as if it were a `TensorBlock`. There are also
warnings where callers ignore the new `Result`.

Consequence: the candidate cannot be proof evidence yet. A build-failing implementation does not discharge
any theorem.

## Candidate Progress

The candidate does make real movement toward the reviewed v2 shape:

- `TensorBlock` has v2-shaped fields: `settled_receipt_set_root`, `checks_root`, `beacon`,
  `difficulty_target`, and `nonce`.
- `produce_block` now returns `Result<TensorBlock>` and rejects non-validator proposers.
- `proposer_for_next_epoch` selects among validators and ignores miner TensorWork.
- `submit_block_vote` looks up the block and calls block validation before counting a vote.
- `canonical_blockspace` derives a selected receipt list from settled receipts, beacon, parent hash, and
  static caps.
- `pow_hash` and `pow_valid` bind nonce search to a candidate header.
- Block storage has started moving from old roots to v2-shaped block fields.

These changes attack several live counterexamples. They are still a candidate slice, not a completed proof
surface.

## Non-Discharged Findings

### CV2-001: The Candidate Does Not Build

The first proof gate is a build-clean committed implementation. The candidate fails `cargo check -p
tensor_vm --all-targets`.

Required repair:

- Update all callers and tests for fallible `produce_block`.
- Treat ignored `Result` warnings as proof-relevant: silently ignoring failed block production can hide
  consensus failures.
- Rerun broad validation before upgrading any proof status.

### CV2-002: Block Validation Is Against Current State, Not An Explicit Parent Snapshot

`blocks::validate(chain, block, strict_state_root)` recomputes selected receipts, checks root, attestation
root, and reward root from `chain.state`. For finality and replay the theorem needs validation against the
block's parent state, not whichever mutable state happens to be present when the vote is submitted.

The candidate partially avoids this for immediate local production, but the proof statement must be:

```text
valid_v2_block(parent_state, block)
```

not:

```text
valid_v2_block(current_node_state, block)
```

Required repair:

- Define the parent-state snapshot used for validation.
- Validate every root and transition against that parent state.
- Prove vote admission imports the parent-state validation result, not a later mutable state.

### CV2-003: The Beacon Is Self-Consistent But Not Parent-State Validated

The candidate block stores `beacon`, and validation recomputes canonical blockspace using `block.beacon`.
It does not prove that `block.beacon` equals the parent state's finalized randomness or another valid
beacon transition.

This preserves a selection-grinding hole: a proposer could choose a beacon that gives a favorable selected
receipt set and still be self-consistent under validation.

Required repair:

- Add `beacon_valid(parent_state, block.beacon)`.
- Bind beacon evolution into the block transition theorem.
- Add adversarial tests for altered beacons that preserve all other roots.

### CV2-004: Selected Receipt Root Does Not Bind The Full Selected Receipt Leaf

`selected_receipt_root` currently hashes a set of receipt ids. The v2 proof target needs a canonical selected
receipt leaf that binds at least receipt id, receipt hash, TensorWork units, and any blockspace-accounting
fields used by caps.

Receipt id may indirectly bind some content if its construction is proven, but the selected-root theorem
should not rely on an unstated transitive assumption.

Required repair:

- Define `SelectedReceiptLeaf`.
- Prove leaf encoding injectivity before hashing.
- Include enough fields to support blockspace and reward theorems.

### CV2-005: Canonical Selection Skips Over Cap-Exceeding Receipts

The reviewed spec says the selector appends receipts in deterministic order until adding the next receipt
would exceed a cap. The candidate continues past a cap-exceeding receipt and may include later smaller
receipts.

That may be a valid design choice, but it is a different theorem. It changes omission/carry-over semantics
and can alter censorship, fairness, and congestion proofs.

Required repair:

- Either change the selector to the spec's stop-before-first-exceed rule, or update the spec and proof
  statement to the skip-over-large-receipts rule.
- Add adversarial tests for one large receipt followed by many small receipts.
- State carry-over and expiry behavior for skipped receipts.

### CV2-006: Eligibility Lacks Unspent, Expiry, And Challenge-Window State

The candidate filters settled receipts and data-unavailable receipts. The v2 proof target also needs
unspent/included state, expiry, challenge-window availability, and carry-over semantics.

Without this state, a receipt can remain selectable after inclusion unless some separate transition removes
it. The candidate production path does not show selected receipts being marked spent or carried over.

Required repair:

- Add included/spent metadata or an equivalent selected-receipt lifecycle.
- Add expiry and challenge-window availability predicates.
- Prove nonselected eligible receipts carry over and selected receipts cannot be included twice.

### CV2-007: `checks_root` Aggregates Signed Claims, Not Recomputed Check Leaves

`block_checks_root` aggregates per-attestation `checks_root` values that are signed, valid, and
data-available. It does not recompute verifier transcripts from selected receipt artifacts. It also does not
define `CheckLeaf`, `VerifierTranscript`, or challenge openings.

This is exactly the bad assumption rejected by the verifier evidence model: a root over signed claims is
still a root over claims.

Required repair:

- Define check leaves over receipt id, seed anchor, primitive, verifier parameters, artifact roots, result,
  and transcript roots.
- Either recompute those leaves during block/vote validation or make them challengeable with consensus
  openings.
- Tie reward settlement to direct recomputation or challenge-window finality.

### CV2-008: `block_checks_root` Does Not Itself Prove Assignment Or Stake

The root helper checks attestation signature relation, `Valid`, DA bit, and receipt id. It does not itself
prove the attesting validator was registered, assigned, had the stated stake, or was accepted by
`submit_attestation`.

This can be acceptable only if the theorem imports the chain admission invariant:

```text
all attestations in state were admitted through submit_attestation
```

That invariant is not automatic while `ChainState` fields are public and tests or internal code can mutate
state directly.

Required repair:

- State the admitted-state invariant explicitly.
- Keep root helpers private to admitted state, or validate assignment/stake in the check-leaf construction.
- Add tests for forged direct state entries if public mutation remains in scope.

### CV2-009: Difficulty Is A Static Helper, Not Parent-State Difficulty

`useful_pow_difficulty_target` returns a fixed target. Validation checks equality to that helper. There is
no parent-state difficulty, retarget rule, floor/ceiling theorem, or useful-work cost relationship.

This may be enough for a tiny local static-difficulty slice. It is not enough for the reviewed useful-PoW
economics theorem.

Required repair:

- Define difficulty state and validate `block.difficulty_target` from parent state.
- Add retarget bounds, target floor/ceiling, and work-floor rules.
- State that static difficulty is local evidence only unless the economic model is discharged.

### CV2-010: Empty Or Tiny Blocks Can Still Be Normal PoW Blocks

There is no nonfallback work floor. If the selected set is empty, the block can still satisfy `pow_valid`
and be produced as a normal block.

The v2 model needs explicit fallback semantics for zero-receipt or no-PoW periods. Empty normal blocks must
not be marketed as useful-verification PoW.

Required repair:

- Add `nonfallback_work_floor(parent_state, block)`.
- Add a separate fallback block validity predicate with reduced rewards and no miner TWU rewards.
- Prove fallback blocks do not claim useful work.

### CV2-011: Reward Mutation Can Precede Block Validity Failure

`produce_block_with_rewards` credits the proposer before calling `produce`. If `produce` fails after that
credit, the reward mutation can remain even though no valid block was produced.

This is a state-transition proof failure: failed block admission must not mutate rewards.

Required repair:

- Make block production/admission atomic with respect to rewards.
- Credit proposer rewards only after block validity succeeds.
- Add an adversarial test for invalid proposer or invalid block production with nonzero rewards.

### CV2-012: Vote Validation Does Not Prove Full State Transition Roots

`submit_block_vote` calls `blocks::validate(chain, &block, false)`. With `strict_state_root = false`, vote
admission does not validate `block.state_root`.

The finality theorem needs finalized blocks to imply a valid v2 state transition, not only parent, roots,
proposer, and PoW-like checks.

Required repair:

- Split validation into explicit predicates: header validity, selected blockspace validity, checks-root
  validity, state-transition validity, and reward-transition validity.
- Require the right complete predicate before finality votes count.

### CV2-013: PoW Hash Ordering Needs A Consensus Integer Model

`hash_below_target` compares byte arrays directly. That may be acceptable if the consensus model defines
hashes as big-endian byte strings and target comparison uses lexicographic order. It is not acceptable if the
intended theorem is over a `U256` with different byte order.

Required repair:

- State the hash-to-integer interpretation.
- Add test vectors around boundary targets.
- Keep the proof model and Rust comparison identical.

## Proof Status Matrix

| Obligation | Candidate Movement | Remaining Status |
| --- | --- | --- |
| V2-BLK-001 canonical selection | Partial selector exists. | Still blocked: eligibility, expiry, spent/carry-over, and spec mismatch remain. |
| V2-BLK-002 selected root | Field exists. | Still blocked: root over ids only, no selected leaf theorem. |
| V2-CHK-001 check leaf recomputability | No semantic leaf. | Still blocked. |
| V2-CHK-002 block checks root | Aggregate root exists. | Still blocked: aggregates signed claims, not recomputed/challengeable evidence. |
| V2-POW-001 useful-PoW validity | Nonce/target/hash predicate exists. | Still blocked: static target, no beacon validity, no work floor, no economics. |
| V2-PROP-001 proposer eligibility | Significant progress: non-validator rejection and validator selector. | Not fully discharged until build-clean and finality imports complete validation. |
| V2-STATE-001 valid transition | Roots exist. | Still blocked: no parent-state apply theorem, spent/carry-over, or reward atomicity. |
| V2-FIN-001 vote admission validates block | Partial: vote path calls `validate`. | Still blocked: validation predicate incomplete and current-state-based. |
| V2-FIN-002 finality implies v2 validity | Partial path. | Still blocked until V2-FIN-001 and V2-STATE-001 are complete. |
| V2-FALLBACK-001 PoW-skip fallback | No candidate evidence. | Missing. |

## Minimum Next Gates

Before this candidate can upgrade any proof status:

1. `cargo check -p tensor_vm --all-targets` passes.
2. All block-production callers handle `Result` explicitly.
3. `valid_v2_block(parent_state, block)` is factored as a named predicate.
4. Beacon, difficulty, selected receipt root, checks root, state root, reward root, and proposer eligibility
   are validated from parent state.
5. Selected receipt lifecycle state prevents double inclusion.
6. `CheckLeaf` is recomputable or challengeable, not merely an aggregated signed `checks_root`.
7. Empty normal blocks are separated from fallback blocks.
8. Rewards are atomic with successful block admission.
9. Adversarial tests cover wrong beacon, wrong selected root, wrong checks root, invalid nonce, invalid
   target, non-validator proposer, cap-boundary selection, repeated receipt inclusion, and failed production
   with rewards.
10. The formal proof manifest and traceability matrix are updated only after the build and tests establish
    the implementation surface.

## Current Judgment

The candidate is directionally aligned with v2 but not proof-sound. It closes the most obvious old shape gap
by adding v2-looking block fields and validator proposer checks, but it still leaves the hard theorem gaps:
parent-state validation, semantic verifier evidence, challenge-window reward finality, selected receipt
lifecycle, difficulty economics, fallback, and build-clean evidence.

Do not claim this candidate proves the MVP core sound.
