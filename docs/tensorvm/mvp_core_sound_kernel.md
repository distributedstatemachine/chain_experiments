# TensorVM MVP Core Sound Kernel v0

Status: documentation-only paper proof boundary compiled from the current worktree.

Purpose: separate the part of the MVP core that has a defensible proof story today from the reviewed v2
consensus claim that remains blocked. This document is intentionally narrow. It should be used as the
allowed claim boundary until the implementation makes the v2 counterexamples impossible.

This is not a mechanized Lean/TorchLean proof. It is a paper-proof kernel that identifies theorem
statements, assumptions, and exclusions for later mechanization.

The theorem import boundary for this kernel is made explicit in
[`mvp_core_theorem_dependency_graph.md`](mvp_core_theorem_dependency_graph.md).
Verifier-local false-accept budgets are recorded in
[`mvp_core_probabilistic_soundness_budget.md`](mvp_core_probabilistic_soundness_budget.md).
The receipt-lifecycle seed model for the hidden-challenge assumption is specified in
[`mvp_core_receipt_lifecycle_seed_model.md`](mvp_core_receipt_lifecycle_seed_model.md).
The signature/authentication boundary is specified in
[`mvp_core_signature_authentication_boundary.md`](mvp_core_signature_authentication_boundary.md).
The canonical encoding and commitment boundary is specified in
[`mvp_core_canonical_encoding_commitment_model.md`](mvp_core_canonical_encoding_commitment_model.md).

## Kernel Claim

The current MVP core supports this limited claim:

```text
Under canonical finite-field tensor semantics, collision-resistant domain-separated hashes, a hidden
receipt-bound validation seed, and honest artifact availability to the verifier:

1. TensorOp verifier acceptance implies the committed matmul output is correct except with Freivalds false
   acceptance probability.
2. LinearTrainingStep verifier acceptance implies the committed finite-field training-step relations hold
   except with the combined Freivalds and random-linear false acceptance probabilities.
3. Chain attestation admission success implies a registered assigned validator signed a matching
   Valid/Invalid/Unavailable statement for an existing receipt, with no duplicate statement by that
   validator for that receipt.
4. Current local settlement can be proved over syntactic quorum statements, redundant-agreement checks, and
   conflict checks.
```

The current MVP core does not support this broader claim:

```text
Finalized current blocks prove useful-verification PoW over canonical settled-receipt blockspace.
```

## Sound Kernel Components

| Component | Included In Kernel | Current Proof Strength | Main Assumptions |
| --- | --- | --- | --- |
| Field arithmetic | Yes | Deterministic local semantics | Rust/formal equivalence, fixed field definition |
| Tensor shape/layout | Yes | Deterministic local semantics | Canonical tensor encoding |
| TensorOp verifier | Yes | Probabilistic algebraic soundness | Hidden uniform-enough challenges, committed outputs |
| LinearTrainingStep verifier | Yes | Probabilistic algebraic soundness | Hidden uniform-enough challenges, field-only training semantics |
| Receipt binding | Partial | Hash-bound metadata and roots | Hash collision resistance, canonical encoding |
| Attestation admission | Yes, syntactic only | Registered assigned signed statement | Reference signature or production signature assumption |
| Attestation quorum | Yes, syntactic only | Unique assigned signed Valid/DataAvailable statements | Stable validator assignment seed remains a caveat |
| Local settlement | Yes, v1 only | Settlement follows syntactic quorum and conflict rules | Semantic verifier execution is not proven by quorum |
| Current block production | No | Reference behavior only | Superseded by v2 |
| Current finality | No for v2 | Stake signatures over known v1 block hashes | Does not imply useful-PoW validity |
| Verification-time artifact retrieval | Partial | Root-matched local/remote fetch can support verifier execution | Does not prove public DA or durable retention |
| Public DA/operator independence | No | Outside kernel | Needs external evidence and assumptions |

## Formal Kernel Model

Let:

```text
F                     finite field for consensus tensor arithmetic
Tensor(F, shape)      canonical tensor over F
H_d(parts)            domain-separated hash modeled as collision resistant
Sig(a, m, s)          signature verification relation
Seed(r)               validation seed fixed before verifier challenge sampling
Commit(T)             canonical tensor commitment root
Receipt               miner statement binding job id, roots, trace root, work units, and signature
Report                verifier output with result and checks_root
Attestation           validator signature over receipt id, job id, primitive, result, checks_root, DA bit
State                 deterministic chain state
```

The kernel treats `Sig` and `H_d` as assumptions. It does not prove SHA-256 collision resistance or
production key ownership.

## Theorems In The Kernel

### K-FIELD-001: Canonical Tensor Determinism

Statement:

```text
For fixed dtype, shape, layout, and element vector, TensorVM tensor operations used by consensus have a
single deterministic result.
```

Proof status: local-proof-ready.

Proof sketch:

The consensus tensor paths operate over explicit finite-field element vectors and explicit shapes. There is
no ambient floating-point rounding or nondeterministic hardware scheduling in the canonical CPU semantics.

What this does not prove:

GPU kernel equivalence, real-valued ML semantics, or external framework equivalence.

### K-TOP-001: TensorOp Completeness

Statement:

```text
If:
  A.shape = [m,k],
  B.shape = [k,n],
  C = A @ B over F,
  receipt roots bind A, B, and C,
  receipt metadata matches the job,
  receipt signature verifies,
then verify_tensor_op(...).result = Valid.
```

Proof status: local-proof-ready.

Proof sketch:

All metadata, shape, root, trace, and signature checks match by premise. For every sampled vector `q`,
Freivalds checks:

```text
A(Bq) = (AB)q = Cq
```

Row sampling also accepts honest rows because each sampled row of `C` equals the corresponding row of
`AB`.

### K-TOP-002: TensorOp Probabilistic Soundness

Statement:

```text
If C != A @ B and q is sampled uniformly from F^n after C is committed, then one full Freivalds round
accepts with probability at most 1 / |F|.
```

Proof status: assumption-bound.

Proof sketch:

Let `D = AB - C`. Since `D != 0`, at least one row of `D` is a non-zero linear form. The bad event is
`Dq = 0`. A non-zero linear polynomial over a field has at most a `1 / |F|` zero probability under uniform
`q`. Independent rounds multiply the bound.

Required assumptions:

- The challenge vector is sampled uniformly enough from the field.
- The miner cannot adapt `C` after learning the challenge.
- Hash-derived randomness is domain-separated and not materially biased for these parameters.

Bad assumption rejected:

```text
Freivalds deterministically proves every output cell.
```

### K-ROW-001: Row Sampling Is Audit Math

Statement:

```text
For t corrupted rows among m total rows and s sampled rows without replacement:
P_detect = 1 - choose(m - t, s) / choose(m, s)
```

Proof status: local-proof-ready.

Proof sketch:

The verifier misses the corruption only if every sampled row is one of the `m - t` clean rows. The miss
probability is the hypergeometric clean-sample probability; detection is one minus that value.

Kernel boundary:

Row sampling is included as audit math, not as the block-validity security primitive.

### K-LIN-001: LinearTrainingStep Algebraic Completeness

Statement:

```text
If:
  Y = XW,
  dY = Y - T,
  G = X^T dY,
  W_next = W - lr * G,
  loss commitment matches the field MSE calculation,
  receipt roots and metadata bind the above tensors,
  receipt signature verifies,
then verify_linear_training_step(...).result = Valid.
```

Proof status: local-proof-ready.

Proof sketch:

The forward and backward matrix equations reduce to K-TOP-001. Error, optimizer, and loss checks are
deterministic finite-field equalities. Honest tensors satisfy all checked equations.

Bad assumption rejected:

```text
LinearTrainingStep proves meaningful real-valued SGD.
```

Correct boundary:

It proves a deterministic finite-field training-shaped transition. It does not prove convergence,
usefulness, fixed-point approximation quality, or faithful real-valued SGD.

### K-LIN-002: Random-Linear Equality Soundness

Statement:

```text
For tensors L != R with identical shape, a random-linear check <q,L> = <q,R> accepts with probability at
most 1 / |F| under uniform q.
```

Proof status: assumption-bound.

Proof sketch:

Let `D = L - R`, with `D != 0`. The equality check accepts when `<q,D> = 0`. This is a non-zero linear
polynomial in the sampled coordinates of `q`, so the zero probability is at most `1 / |F|`.

### K-ATT-001: Attestation Admission Is Syntactic

Statement:

```text
If SubmitAttestation(A) succeeds, then:
  A.validator is registered,
  A.stake equals registered validator stake,
  A.signature verifies for A's statement,
  A.validator is assigned to A.receipt_id under the current assignment function,
  A.receipt_id exists,
  A.job_id and A.primitive_type match the stored receipt,
  no previous attestation from A.validator exists for A.receipt_id.
```

Proof status: local-proof-ready, with seed-lifecycle caveat.

Proof sketch:

The admission function checks each listed condition before inserting the attestation into state. Duplicate
validator submissions are rejected before insertion.

Kernel boundary:

This theorem does not prove the validator actually ran the verifier. It proves the chain accepted a signed
statement from an assigned validator.

### K-QUO-001: Quorum Is Syntactic

Statement:

```text
If has_attestation_quorum(receipt_id) is true, then enough unique assigned validators signed
Valid/DataAvailable statements for the stored receipt under the current quorum rule.
```

Proof status: local-proof-ready for syntax, assumption-bound for validator honesty.

Proof sketch:

The quorum function iterates stored attestations, ignores unassigned validators, ignores duplicate validator
entries, checks signature validity, checks receipt metadata, requires `Valid`, requires
`data_availability_passed`, and compares counted stake/validator thresholds against assigned validator
stake.

Kernel boundary:

The chain does not recompute verifier transcripts during quorum calculation. The semantic claim
`quorum -> verified tensor work` requires either honest validators as an assumption or a future
recomputable/challengeable evidence surface.

### K-SET-001: Local Settlement Follows Syntactic Quorum

Statement:

```text
If a receipt is newly settled by the current settlement transition, then it had syntactic attestation
quorum, satisfied redundant agreement if required, was not marked data-unavailable, and did not violate the
current linear-transition conflict rule.
```

Proof status: local-proof-ready for current v1 settlement behavior.

Proof sketch:

The settlement transition skips receipts already settled, receipts without quorum, unavailable receipts,
receipts without required redundant agreement, and conflicting linear state transitions before inserting
into `settled_receipts` and crediting rewards.

Kernel boundary:

This does not prove canonical v2 block inclusion, challenge-window reward finality, or useful-verification
PoW.

## Explicitly Outside The Kernel

These are not defensible as current proof claims:

| Claim | Why Excluded |
| --- | --- |
| Finalized blocks prove useful-verification PoW | Current blocks have no `settled_receipt_set_root`, `checks_root`, `difficulty_target`, or `nonce`. |
| Produced blocks imply eligible proposers | Current block production accepts a supplied proposer address. |
| TensorWork-selected miners are v2 proposers | v2 says TensorWork does not select block proposers. |
| Receipt map root is canonical blockspace | The current block commits the global receipt map, not a deterministic selected settled-receipt set. |
| Attestation quorum proves verifier execution | The chain counts signed statements and does not recompute verifier transcripts. |
| Remote tensor fetch proves public DA | Root-matched fetch proves only verification-time retrieval by one runtime. |
| Reference signatures prove production authentication | The current signing helper is a hash relation, not production crypto. |
| Local tensor serving proves public DA | Local fetches do not prove independent public retention. |
| Local Compose operators are independent principals | Containers and deterministic wallet labels are local evidence only. |

## Assumption Ledger For Kernel Claims

| Assumption | Needed By | Failure Mode |
| --- | --- | --- |
| Canonical Rust/formal equivalence | All kernel theorems | Mechanized proof could prove a different semantics than Rust executes. |
| Hash collision resistance | Receipt/root/statement binding | Different preimages could share an accepted commitment. |
| Hidden receipt-bound randomness | Freivalds and random-linear soundness | Miner can adapt outputs to challenges. |
| Uniform-enough challenge sampling | Freivalds and random-linear soundness | False-accept bounds become invalid. |
| Stable receipt lifecycle seed | Assignment/quorum across delayed validation | Same receipt can be judged under the wrong beacon. |
| Signature unforgeability or reference-signature caveat | Receipt, attestation, vote statements | Statements may not prove actor control. |
| Artifact availability during verification | Verifier semantic claims | Validators cannot recompute required checks. |
| Validator honesty or challenge evidence | Quorum-to-semantics bridge | Signed Valid statements may not reflect actual verification. |

## Upgrade Gates To Expand The Kernel

The kernel can only expand toward the full MVP theorem after these are true:

1. Receipt admission stores a receipt-lifecycle validation seed or assignment anchor.
2. Attestation evidence is bound to recomputable check leaves or challenge openings.
3. A v2 settled-receipt pool exists with eligibility, expiry, spent/carry-over, byte cap, TWU cap, and count
   cap semantics.
4. `TensorBlock` or its successor commits `settled_receipt_set_root`, `checks_root`, `difficulty_target`,
   `nonce`, and beacon.
5. Block production/admission rejects ineligible proposers.
6. Useful-verification PoW validation ties the nonce to the selected receipt set and checks root.
7. Finality votes count only blocks that pass v2 block validity.
8. Production signatures replace or explicitly wrap the reference signature helper.
9. Public DA and operator-independence claims are backed by external evidence, not local Compose.

## Current Judgment

The sound kernel is real but small. It is mostly verifier-local algebra plus syntactic chain admission and
settlement rules. The consensus layer remains outside the kernel for the reviewed v2 MVP. Any summary that
collapses this kernel into "the MVP core is formally proven sound" is still overstating the current system.
