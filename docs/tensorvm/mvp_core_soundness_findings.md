# TensorVM MVP Core Soundness Findings

Status: critical findings memo compiled from the current worktree.

Scope: this document does not implement code. It records what the current MVP core can support with formal
proofs, what remains only an assumption, and where the implementation contradicts the reviewed MVP spec.

## Executive Finding

The current MVP core has a credible formal proof path for verifier-local algebraic claims, but it does not
yet have a sound consensus proof for the reviewed v2 MVP.

The strongest current claims are:

- TensorOp verification can be specified as Freivalds over canonical finite-field tensors.
- LinearTrainingStep verification can be reduced to Freivalds plus random-linear equality checks.
- Chain attestation admission now checks registered validator stake, signature, deterministic assignment,
  receipt metadata, and duplicate attestations.

The weakest current claims are:

- Blocks are not useful-verification PoW blocks.
- Blocks do not commit to a canonical settled-receipt set or recomputable block-level `checks_root`.
- Finality votes do not validate useful-verification PoW or canonical blockspace.
- Proposer selection still uses the superseded settled TensorWork model.
- Several security properties depend on hash, signature, randomness, network availability, and operator
  independence assumptions that are not proven by tests.

Bottom line: the verifier core is partially proof-ready; the consensus core is not yet sound for the v2 MVP.

## Evidence Snapshot

### Current v2 Spec Requirement

The reviewed spec says validators should verify the canonical settled-receipt set, commit to `checks_root`,
and search for a PoW nonce over that commitment. It also says TensorWork no longer selects proposers.

Evidence:

- [`mvp_spec.md`](mvp_spec.md) defines useful-verification PoW and says TensorWork no longer selects
  proposers.
- [`mvp_core_formal_proofs.md`](mvp_core_formal_proofs.md) already records this as the central proof
  boundary.

### Current Block Shape Contradicts The v2 Spec

Current block fields are still:

```text
job_root
receipt_root
attestation_root
state_root
reward_root
randomness
```

Current block fields are missing:

```text
settled_receipt_set_root
checks_root
difficulty_target
nonce
```

Evidence:

- [`../../crates/tensor_vm/src/chain/state.rs`](../../crates/tensor_vm/src/chain/state.rs) defines
  `TensorBlock` with `job_root`, `receipt_root`, and `randomness`, but no v2 PoW/check fields.
- [`../../crates/tensor_vm/src/chain/blocks.rs`](../../crates/tensor_vm/src/chain/blocks.rs) produces a
  block by committing global job/receipt/attestation maps and advancing height/randomness with no PoW
  predicate.

Finding:

This is not a minor implementation gap. The consensus object is wrong for the reviewed v2 MVP. Adding a
nonce to the existing block shape would not fix the proof, because validators still could not recompute the
canonical receipt set or the expected block-level verification commitment.

### Current Proposer Selection Contradicts The v2 Spec

Current proposer selection computes total settled miner TensorWork and selects from miners when total work
is nonzero. It falls back to validators only when total work is zero.

Evidence:

- [`../../crates/tensor_vm/src/chain/proposer.rs`](../../crates/tensor_vm/src/chain/proposer.rs) selects
  by `settled_tensor_work`.
- [`coverage_matrix.md`](coverage_matrix.md) now marks the v2 useful-verification PoW criterion as not
  complete.
- [`completion_audit.md`](completion_audit.md) now marks the v2 block-production and zero-receipt fallback
  criteria as not complete.

Finding:

The old proposer path may remain useful as a compatibility or local reference path, but it cannot be counted
as proof evidence for the reviewed MVP.

### Current Finality Does Not Validate v2 Block Soundness

Current block voting checks:

- validator exists
- stake matches
- vote signature verifies
- block hash/height exists
- duplicate vote is rejected
- signed stake reaches threshold

It does not check:

- useful-verification PoW target
- canonical settled-receipt set
- recomputed `checks_root`
- v2 block reward challenge-window state
- invalid canonical-set omission or receipt-subset grinding

Evidence:

- [`../../crates/tensor_vm/src/chain/validation.rs`](../../crates/tensor_vm/src/chain/validation.rs)
  validates block votes and finality as stake-threshold checks over known block hashes.

Finding:

The current finality proof is only a BFT signature-threshold proof for the current reference block type. It
is not a finality proof for useful-verification PoW block validity.

## Claims That Are Actually Proof-Ready

### F1: TensorOp Freivalds Completeness

Claim:

```text
If C = A @ B under canonical field semantics and the receipt commits to A, B, C, then TensorOp verification
accepts.
```

Why it is proof-ready:

- Shape checks, receipt digest checks, signature checks, program hash checks, commitment-root checks, and
  trace-root checks happen before Freivalds.
- Honest matrix multiplication satisfies `A(Bq) = Cq`.

Evidence:

- [`../../crates/tensor_vm/src/verify.rs`](../../crates/tensor_vm/src/verify.rs) implements
  `full_freivalds` and `verify_tensor_op`.

Formal proof obligation:

```text
matmul_semantics(A,B,C) and receipt_binds(A,B,C,R) imply verify_tensor_op(...).result = Valid
```

### F2: TensorOp Freivalds Soundness

Claim:

```text
If C != A @ B, one full Freivalds round accepts with probability at most 1 / |F|.
```

This is a probabilistic theorem, not a deterministic guarantee.

Bad assumption to reject:

```text
"Freivalds proves every output cell is correct."
```

Correct statement:

```text
Freivalds bounds false acceptance under uniform hidden randomness and committed outputs.
```

Formal proof obligation:

```text
C != A @ B -> Pr_q[A(Bq) = Cq] <= 1 / |F|
```

Explicit assumptions:

- Sample vector is uniform enough.
- Miner cannot adapt output after seeing the validation seed.
- Hash-derived sampling behaves like the stated random oracle model.

### F3: Row Sampling Is Audit Evidence Only

Claim:

```text
If t of m rows are corrupted and s rows are sampled without replacement:
P_detect = 1 - choose(m - t, s) / choose(m, s)
```

Bad assumption to reject:

```text
"Sixteen sampled rows is meaningful block validity for large matrices."
```

For one corrupted row in a 1024-row output, 16 sampled rows detects only about 1.56% before considering any
other checks. Row sampling should not be used as the only block-eligibility check.

Evidence:

- `row_sample_detection_probability` exists in the verifier.
- The spec and proof boundary already classify row sampling as audit coverage unless stronger bounds are
  documented.

### F4: LinearTrainingStep Algebraic Completeness

Claim:

```text
If Y = XW, dY = Y - T, grad_W = X^T dY, W_next = W - lr * grad_W, and roots match the receipt, then the
linear training verifier accepts.
```

Why it is proof-ready:

- Forward and backward matrix multiplications reduce to Freivalds.
- Error and optimizer checks reduce to random-linear equality.
- Loss commitment is recomputed exactly under the current field arithmetic.

Evidence:

- [`../../crates/tensor_vm/src/verify.rs`](../../crates/tensor_vm/src/verify.rs) implements
  `verify_linear_training_step`.

Bad assumption to reject:

```text
"This proves real-valued SGD."
```

Correct statement:

```text
This proves the configured finite-field algebraic update, not semantic convergence of real-valued ML
training.
```

### F5: Assigned Validator Attestation Admission

Claim:

```text
If SubmitAttestation succeeds, then the validator is registered, has the stated stake, signs the statement,
is assigned to the receipt, references the stored receipt, and has not already attested to it.
```

Evidence:

- [`../../crates/tensor_vm/src/chain/validation.rs`](../../crates/tensor_vm/src/chain/validation.rs)
  checks registration, stake, signature, assignment, receipt metadata, and duplicates.
- [`../../crates/tensor_vm/src/scheduler.rs`](../../crates/tensor_vm/src/scheduler.rs) binds validator
  assignment to seed, receipt id, and validator address.

Remaining flaw:

Assignment is receipt-bound, but the chain recomputes assignment from current `finalized_randomness`. A
delayed attestation can be evaluated against a different beacon if finality randomness advances before the
receipt lifecycle closes.

Required proof repair:

```text
Receipt admission must store or derive a receipt-lifecycle validation seed, and all assignment/verifier
checks must use that seed until the receipt expires.
```

## Claims That Are Not Proven

### N1: Useful-Verification PoW

Not proven because the implementation lacks:

- canonical settled-receipt blockspace selector
- block-level `settled_receipt_set_root`
- block-level `checks_root`
- difficulty target
- nonce
- block-validity predicate tying all of the above together

Formal theorem that cannot currently be stated over the code:

```text
If a block is finalized, then some registered validator verified the canonical settled-receipt set and found
a nonce satisfying H(parent || receipt_set_root || checks_root || beacon || validator || nonce) < target.
```

### N2: Canonical Receipt Inclusion

Not proven because there is no v2 selector that answers:

- Which settled receipts are eligible?
- Which receipts are already spent?
- Which receipts are expired?
- Which receipts are excluded by byte/TWU/count caps?
- How is carry-over handled?
- How does a validator prove a block omitted a required receipt?

Bad assumption to reject:

```text
"A receipt_root over the current map is equivalent to deterministic blockspace."
```

It is not. A global receipt map root does not prove a canonical per-block receipt set.

### N3: Block-Level `checks_root` Recomputability

Not proven because attestations contain per-receipt `checks_root` values, but blocks do not commit to a
canonical aggregate verification transcript over the selected receipt set.

Bad assumption to reject:

```text
"Validator attestations imply the proposer actually verified the block receipt set."
```

They do not. Without a block-level transcript and challenge path, a proposer can commit whatever the current
block type allows.

### N4: Public Data Availability

Not proven by local tensor fetches.

Local tests can prove the code path for tensor serving and openings. They cannot prove public network
retention, independent hosting, or 95% availability during active and retention windows.

Bad assumption to reject:

```text
"Serving sampled chunks during validation is durable data availability."
```

It is verification-time availability only.

### N5: Signature Security

Not proven. The reference signature helper is deterministic hashing over address and message.

Evidence:

- [`../../crates/tensor_vm/src/types.rs`](../../crates/tensor_vm/src/types.rs) defines `sign` as a hash,
  and `verify_signature` recomputes that hash.

Bad assumption to reject:

```text
"The reference signature helper proves production key security."
```

It does not. Production security requires a real signature scheme, key ownership, anti-replay domain
separation, and key-management assumptions.

### N6: Operator Independence

Not proven by distinct addresses, local containers, or deterministic wallet labels.

Bad assumption to reject:

```text
"Ten miners and five validators in local Compose are independent operators."
```

They are separate local participants, not independent economic/security principals.

## Highest-Risk Bad Assumptions

1. **"The MVP core is sound because Gate 0 passes."**
   Gate 0 is valuable local evidence, but it does not prove the reviewed v2 consensus theorem.

2. **"TensorWork proposer selection is a minor legacy detail."**
   It changes the consensus resource. The reviewed MVP says validator verification work, not miner TensorWork,
   is the block-production primitive.

3. **"checks_root exists, so useful verification is proved."**
   Per-receipt check roots are not enough. The block needs a canonical aggregate commitment and a validation
   rule.

4. **"Block finality implies block validity."**
   Current finality proves signatures over an existing block hash. It does not prove useful-PoW validity.

5. **"Hash-derived randomness equals unbiasable randomness."**
   It only helps if the seed is fixed after commitment and cannot be ground by whoever controls the block
   hash or receipt timing.

6. **"Field training is real training."**
   The current LinearTrainingStep can prove an algebraic transition. It does not prove useful ML convergence
   or faithful real-valued SGD.

7. **"Local data serving is data availability."**
   Local serving proves a path, not public retention or adversarial availability.

8. **"Reference signatures are security signatures."**
   They are test-domain authentication placeholders.

## Formal Proof Roadmap

### Proof Group A: Verifier Algebra

Status: ready for mechanization.

Theorems:

- Tensor shape preservation for matmul and transpose.
- Freivalds completeness.
- Freivalds one-round soundness.
- Repeated Freivalds bound under independent rounds.
- Random-linear equality completeness and soundness.
- LinearTrainingStep algebraic completeness.

### Proof Group B: Receipt And Attestation Admission

Status: partially ready.

Theorems:

- Receipt digest/root/signature checks bind submitted metadata to receipt contents.
- Successful attestation admission implies registered assigned validator and matching receipt metadata.
- Attestation quorum only counts assigned, signed, valid, data-available attestations.

Missing:

- Receipt-lifecycle validation seed stored or otherwise stable across delayed validation windows.
- Production signature model.

### Proof Group C: Settlement

Status: partially ready.

Theorems:

- Settlement only credits receipts with quorum.
- Settlement skips unavailable receipts and conflicting linear transitions.
- Rewards are deterministic from the settled receipt set.

Missing:

- v2 settled-receipt pool with spent/carry-over/expiry semantics.
- Challenge-window delayed reward finality.

### Proof Group D: Consensus

Status: not ready.

Required before proof:

- v2 block fields.
- canonical settled-receipt selector.
- recomputable block-level `checks_root`.
- useful-verification PoW predicate.
- finality vote validation against v2 block validity.
- PoW-skip fallback for zero-receipt or no-PoW periods.

Target theorem:

```text
If a non-fallback block is finalized, then:
  proposer is a registered validator,
  receipt_set is canonical for parent state and blockspace caps,
  settled_receipt_set_root commits to that receipt_set,
  checks_root recomputes from the selected receipt verification transcripts,
  nonce satisfies the useful-verification PoW target,
  finality signatures cover the valid block hash.
```

The current code cannot prove this theorem.

## Recommended Next Documentation Or Implementation Decision

Do not claim "MVP core soundness" yet.

The next implementation feature, when code work resumes, should be:

```text
useful_verification_block_v0:
  canonical settled-receipt blockspace
  block-level settled_receipt_set_root
  block-level checks_root
  static difficulty target
  nonce predicate
  finality vote rejection for invalid v2 blocks
```

The next documentation feature, if staying docs-only, should be:

```text
formal_proof_manifest_v0:
  approved primitive names
  Rust verifier functions
  theorem names
  explicit assumptions
  tests/evidence
  gaps that block consensus claims
```

## Current Completion Judgment

Not complete.

The current repository has a useful proof boundary and several proof-ready verifier claims. It does not yet
have a sound formal proof for the full reviewed MVP core because the consensus layer still uses the wrong
block-production object.
