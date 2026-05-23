# TensorVM MVP Core Formal Proof Boundary

Status: proof/audit draft for the current Rust reference core.

This document is not a completed mechanized Lean proof. It is a formal proof plan plus paper-proof audit of
the MVP core that exists today. A claim is marked **proved locally** only when the proof obligation maps to
current deterministic Rust code and focused tests. A claim is marked **assumption** when the property depends
on cryptography, network/economic behavior, or a still-missing consensus mechanism.

## Critical Finding

The verifier core is much stronger than the consensus core.

The TensorOp and LinearTrainingStep verifier paths have clear algebraic proof obligations and tests. The
block-production path still uses the superseded v1 model: blocks commit global job/receipt roots and are
produced by a proposer selected from settled miner TensorWork. The reviewed MVP spec now requires
validator useful-verification PoW over a canonical settled-receipt set. Until that blockspace/PoW slice is
implemented, TensorVM should not claim the v2 MVP consensus theorem.

## Formal Model

Let:

```text
F_p                      finite field used by consensus tensor arithmetic
H                        domain-separated hash oracle
Sig                      signature verification relation
State                    deterministic chain state
Job                      TensorOp or LinearTrainingStep workload request
Receipt                  signed miner output commitment for a Job
Attestation              signed validator statement about a Receipt
AssignValidators(S,r,b)  deterministic validator assignment from state S, receipt r, beacon b
Verify(J,R,A,b)          runtime verifier over job J, receipt R, artifacts A, and beacon b
Settle(S,r)              state transition that marks receipt r settled and credits rewards
```

All consensus-relevant maps are ordered maps or sets, so root construction and assignment iteration are
deterministic for a fixed state.

## Proved Locally

### P1: TensorOp Completeness

Statement:

```text
If C = A @ B under canonical field matmul semantics, and receipt R commits to A, B, C, then
VerifyTensorOp(J,R,A,B,C,b).result = Valid.
```

Proof sketch:

- The receipt digest, signature, program hash, input roots, output root, trace root, and shapes are checked.
- Freivalds accepts honest products because `A(Bq) = (AB)q = Cq` for every sampled vector `q`.
- Row sampling is an audit layer and also accepts honest rows.

Evidence:

- `verify_tensor_op`
- `full_freivalds`
- `verify::tests::full_freivalds_accepts_honest_and_rejects_corruption`
- `verify::tests::tensor_op_verifier_rejects_metadata_and_shape_mismatches`

### P2: TensorOp Soundness Bound

Statement:

```text
If C != A @ B and the verifier samples q uniformly from F_p^n after C is committed, then one full
Freivalds round accepts with probability at most 1 / p. Independent rounds multiply the bound.
```

Proof sketch:

Let `D = AB - C`, with `D != 0`. At least one row of `D` is a non-zero linear form. The check accepts only
when `Dq = 0`. A non-zero linear polynomial over `F_p` has at most a `1 / p` zero probability under a
uniform vector. Independent domain-separated rounds multiply false-accept probability.

Assumptions:

- `random_field_vector` is modeled as uniform over `F_p`.
- The validation seed is hidden until the receipt is committed.
- Hash domain separation has no exploitable collisions.

### P3: Row Sampling Is Not A Block-Validity Proof

Statement:

```text
For t corrupted rows among m rows and s sampled rows without replacement:
P_detect = 1 - C(m - t, s) / C(m, s).
```

Implication:

Sparse row corruption is weakly detected when `t` is small. Row sampling must remain audit telemetry unless
the configured probability is explicitly high enough for the job shape.

Evidence:

- `row_sample_detection_probability`
- `verify::tests::row_sampling_probability_exposes_sparse_weakness`
- `study::tests::row_sampling_study_blocks_sparse_row_sampled_only_acceptance`

### P4: LinearTrainingStep Completeness

Statement:

```text
If Y = XW, dY = Y - T, G = X^T dY, W_next = W - lr * G, and the receipt commits to those tensors,
then VerifyLinearTrainingStep(...).result = Valid.
```

Proof sketch:

- Forward and backward matmuls reduce to P1.
- Error and optimizer checks are random-linear equality checks over tensors with matching shapes.
- Loss is recomputed exactly under consensus field arithmetic.

Evidence:

- `verify_linear_training_step`
- `vm::tests::linear_backward_and_sgd_match_equations`
- `jobs::tests::linear_receipt_commits_to_learning_step`
- `verify::tests::linear_training_verifier_rejects_metadata_and_commitment_mismatches`

### P5: LinearTrainingStep Random-Linear Soundness

Statement:

```text
For tensors L != R of equal shape, random-linear equality
<q,L> = <q,R> accepts with probability at most 1 / p.
```

Proof sketch:

`<q,L-R>` is a non-zero linear polynomial in the sampled vector `q`. It evaluates to zero with probability at
most `1 / p` under uniform sampling.

Evidence:

- `verify::tests::linear_training_verifier_rejects_sparse_error_poisoning`
- `verify::tests::linear_training_verifier_rejects_sparse_weight_poisoning`

### P6: Chain Attestation Admission

Statement:

```text
If ChainCommand::SubmitAttestation(A) succeeds, then:
  A.validator is registered,
  A.stake equals the registered validator stake,
  A.signature verifies,
  A.validator is in AssignValidators(State, A.receipt_id, State.finalized_randomness),
  A.receipt_id exists,
  A.job_id and A.primitive_type match the stored receipt,
  no prior attestation by A.validator exists for that receipt.
```

Proof sketch:

`chain::validation::submit_attestation` checks registration, stake, signature, deterministic assignment,
receipt existence, receipt metadata, and duplicate validator submissions before inserting the attestation.

Evidence:

- `chain::validation::submit_attestation`
- `chain::tests::unassigned_validator_attestations_are_rejected`
- `chain::tests::duplicate_receipts_and_validator_attestations_are_rejected`
- `chain::tests::forged_attestation_stake_is_rejected`

Recent hardening:

The shared chain engine now rejects attestations from validators outside the deterministic assigned set.
Before this hardening, role-loop code checked assignment, but a direct `SubmitAttestation` caller could
inject an otherwise valid unassigned attestation.

## Explicit Assumptions

These are not proved by the Rust code and should not be implied by local tests:

- **Hash binding**: SHA-256/domain-separated hash outputs are collision resistant for receipts, roots,
  transcripts, and pseudo-random draws.
- **Signature security**: production signatures are unforgeable and keys are controlled by claimed
  operators. The current reference `sign` helper is not a production signature scheme.
- **Randomness unbiasability**: validation samples and future useful-verification PoW inputs are not known
  to miners before receipt commitment and cannot be biased by the current block proposer.
- **Artifact availability**: validators can retrieve the tensor rows/chunks/openings needed during active
  validation and retention windows.
- **Operator independence**: redundant miner agreement and validator quorum assumptions require independent
  operators, not only distinct local addresses.
- **Rust/formal equivalence**: the Rust verifier must match the eventual Lean/TorchLean semantics and
  approved program manifest for each consensus-eligible primitive.
- **Useful-PoW economics**: useful-verification PoW is only useful if verification materially gates nonce
  search. If nonce search dominates, validators can skip verification and brute-force headers.

## Known Unsound Or Incomplete Areas

### U1: v2 Block Production Is Not Implemented

Current blocks do not contain:

```text
settled_receipt_set_root
checks_root
difficulty_target
nonce
```

They still contain v1 fields:

```text
job_root
receipt_root
randomness
```

Consequence:

The current chain can prove local deterministic block production, but not useful-verification PoW block
production.

### U2: TensorWork Still Selects Proposers

The current `proposer_for_next_epoch` path selects miners by settled TensorWork and falls back to validators
when there is no settled work. The reviewed spec says TensorWork must affect miner rewards and blockspace
accounting only, not proposer eligibility.

Consequence:

Tests that pass for settled TensorWork proposer selection are regression tests for the old model, not proof
evidence for the reviewed MVP.

### U3: No Canonical Settled-Receipt Blockspace Selector

The state has `settled_receipts: BTreeSet<Hash>`, but not a v2 settled-receipt pool with deterministic
selection by parent/beacon, spent/carry-over status, expiration, byte cap, TWU cap, and receipt-count cap.

Consequence:

Validators cannot currently recompute the exact receipt set that a useful-verification PoW block was
supposed to verify.

### U4: Finality Does Not Validate v2 Block Soundness

Block votes currently prove stake-weighted signatures over known block hashes. They do not check PoW target,
canonical blockspace, recomputed `checks_root`, or challenge-window state.

Consequence:

BFT finality exists for the reference block type, but not for the reviewed useful-verification PoW block
type.

### U5: Validator Assignment Seed Is Not Receipt-Lifecycle Stable

Attestation admission currently uses the chain's current `finalized_randomness` to recompute assignment.
That is deterministic for immediate local validation, but it is too weak for delayed attestations if
finalized randomness advances before the receipt's validation window closes.

Required upgrade:

Store or derive a receipt-locked validation seed at receipt admission and use that seed for assignment,
Freivalds/random-linear checks, and attestation verification throughout the receipt lifecycle.

## Next Core Upgrade

The next coherent feature should be `useful_verification_block_v0`:

```text
1. Add settled-receipt blockspace metadata and deterministic canonical selection.
2. Add block fields for settled_receipt_set_root, checks_root, difficulty_target, and nonce.
3. Implement a static-difficulty useful-verification PoW predicate over the v2 block header.
4. Reject block votes unless the block passes parent, canonical receipt set, checks_root, and PoW checks.
5. Keep difficulty retargeting and challenge-window reward clawback as follow-up slices.
```

Do not add nonce search to the existing v1 block header and call it useful PoW. That would preserve the wrong
consensus object.
