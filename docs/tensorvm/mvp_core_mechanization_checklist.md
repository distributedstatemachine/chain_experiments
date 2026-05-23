# TensorVM MVP Core Mechanization Checklist

Status: documentation-only checklist for future Lean/TorchLean work.

Purpose: turn the sound-kernel paper proofs into a concrete mechanization plan without pretending that the
blocked v2 consensus theorems are ready. This file is intentionally scoped to the defensible kernel in
[`mvp_core_sound_kernel.md`](mvp_core_sound_kernel.md).
The import graph that separates completed kernel proofs from blocked v2 nodes is documented in
[`mvp_core_theorem_dependency_graph.md`](mvp_core_theorem_dependency_graph.md).

This is not a mechanized proof artifact. It is the checklist a mechanized proof package should satisfy
before any claim moves from "paper proof" to "formal proof."

## Scope Rule

Mechanize only these now:

```text
finite-field tensor determinism
matmul shape and algebra
Freivalds completeness
Freivalds one-round and repeated soundness under assumptions
row-sampling detection probability
random-linear equality completeness and soundness under assumptions
LinearTrainingStep field-algebra completeness
canonical encoding pre-hash injectivity where applicable
syntactic attestation admission/quorum invariants
v1 syntactic settlement invariant
root-matched verification-time artifact retrieval
```

Do not mechanize these as completed theorems yet:

```text
canonical settled-receipt blockspace
block-level checks_root recomputability
useful-verification PoW validity
finality implies v2 block validity
zero-receipt PoW-skip fallback liveness
public DA
operator independence
production authentication
real-valued SGD correctness
```

Those can be represented as assumptions, blocked theorem stubs, or future theorem names only.

## Proposed Package Shape

```text
formal/TensorVM/Field.lean
formal/TensorVM/Tensor.lean
formal/TensorVM/Matmul.lean
formal/TensorVM/RandomOracle.lean
formal/TensorVM/Freivalds.lean
formal/TensorVM/RandomLinear.lean
formal/TensorVM/LinearStep.lean
formal/TensorVM/Encoding.lean
formal/TensorVM/Signature.lean
formal/TensorVM/Attestation.lean
formal/TensorVM/Settlement.lean
formal/TensorVM/DataAvailability.lean
formal/TensorVM/SoundKernel.lean
formal/TensorVM/BlockedConsensus.lean
```

`SoundKernel.lean` should import only completed kernel proofs and explicit assumptions. `BlockedConsensus.lean`
should contain theorem statements or comments for v2 consensus goals without proofs that would imply current
implementation support.

## Definitions To Pin Down First

| Definition | Module | Required Before | Notes |
| --- | --- | --- | --- |
| `FieldElement` | `Field.lean` | All algebraic proofs | Must match the consensus field used by Rust. |
| `Tensor shape` | `Tensor.lean` | Matmul, random-linear checks | Shape is part of the theorem, not metadata trivia. |
| `Tensor values` | `Tensor.lean` | All verifier proofs | Fixed row-major semantics unless Rust specifies otherwise. |
| `matmul` | `Matmul.lean` | Freivalds, LinearTrainingStep | Define over finite-field tensors only. |
| `dot_vector` | `Matmul.lean` | Freivalds | Must align with Rust verifier semantics. |
| `transpose` | `Tensor.lean` | LinearTrainingStep | Needed for `X^T dY`. |
| `random_vector` | `RandomOracle.lean` | Freivalds and random-linear soundness | Model as assumption-backed uniform sampling. |
| `canonical_encode` | `Encoding.lean` | Receipt/root binding | Prove pre-hash injectivity for encodings we claim canonical. |
| `hash` | `Encoding.lean` or `RandomOracle.lean` | Binding and sampling | Collision resistance/random-oracle behavior remains assumed. |
| `signature_valid` | `Signature.lean` | Receipt/attestation/vote statements | Reference helper is not production security. |
| `validator_assigned` | `Attestation.lean` | Attestation/quorum proofs | Must include the seed-lifecycle caveat if modeling current Rust. |
| `quorum` | `Attestation.lean` | Settlement | Syntactic signed-statement quorum only. |
| `settle_receipt_v1` | `Settlement.lean` | v1 settlement theorem | Do not conflate with v2 blockspace inclusion. |
| `root_matched_fetch` | `DataAvailability.lean` | Verification-time artifact retrieval | Proves payload/root matching, not public DA. |

## Assumptions To Declare Explicitly

| Assumption | Suggested Name | Used By | Reason It Cannot Be Proved Here |
| --- | --- | --- | --- |
| Hash collision resistance | `assumption hash_collision_resistant` | Receipt/root binding | Cryptographic assumption. |
| Random-oracle-like sampling | `assumption random_vector_uniform` | Freivalds, random-linear | Hash-derived sampling is modeled, not proven uniform. |
| Challenge hidden until commitment | `assumption challenge_after_commitment` | Soundness bounds | Protocol/lifecycle property, not pure algebra. |
| Independent rounds | `assumption rounds_independent` | Repeated Freivalds bound | Depends on domain separation and sampling model. |
| Signature unforgeability | `assumption signature_unforgeable` | Statement authenticity | Current Rust helper is not production crypto. |
| Key ownership | `assumption key_ownership` | Actor claims | Operational/cryptographic assumption. |
| Artifact availability during verification | `assumption verifier_artifacts_available` | Verifier semantic claims | Network/runtime property. |
| Validator honesty or challenge evidence | `assumption validators_report_truthfully_or_can_be_challenged` | Quorum-to-semantics bridge | Current quorum is syntactic. |
| Rust/formal equivalence | `assumption rust_matches_formal_model` | All code-backed theorems | Requires extraction/conformance work. |

Every theorem file should import assumptions deliberately. Do not hide these in comments.

## Theorem Checklist

| ID | Theorem | Module | Dependencies | Status Target | Stop Condition |
| --- | --- | --- | --- | --- | --- |
| MECH-FIELD-001 | `tensor_ops_deterministic` | `Tensor.lean` | `FieldElement`, tensor shape/value definitions | Complete | Fails if tensor semantics depend on hardware/backend behavior. |
| MECH-MM-001 | `matmul_shape` | `Matmul.lean` | Tensor shape, `matmul` | Complete | Fails if invalid shapes are not represented in the theorem. |
| MECH-MM-002 | `matmul_dot_associative_for_freivalds` | `Matmul.lean` | `matmul`, `dot_vector` | Complete | Fails if row/column orientation differs from Rust. |
| MECH-FRV-001 | `freivalds_complete` | `Freivalds.lean` | MECH-MM-002 | Complete | Fails if honest products can reject under any sampled vector. |
| MECH-FRV-002 | `freivalds_sound_one_round` | `Freivalds.lean` | Random vector model, nonzero linear polynomial lemma | Assumption-bound complete | Fails if theorem omits challenge-after-commitment. |
| MECH-FRV-003 | `freivalds_repeated_sound` | `Freivalds.lean` | MECH-FRV-002, independent rounds | Assumption-bound complete | Fails if repeated rounds are treated as independent without assumption. |
| MECH-ROW-001 | `row_sample_detection_probability` | `RandomLinear.lean` or `Freivalds.lean` | finite sampling without replacement | Complete | Fails if sparse corruption weakness is hidden. |
| MECH-RLIN-001 | `random_linear_equal_complete` | `RandomLinear.lean` | dot product over tensors | Complete | Fails if equal tensors can reject. |
| MECH-RLIN-002 | `random_linear_equal_sound` | `RandomLinear.lean` | random vector model, nonzero linear polynomial lemma | Assumption-bound complete | Fails if uniform sampling assumption is absent. |
| MECH-LIN-001 | `linear_training_step_complete` | `LinearStep.lean` | MECH-FRV-001, MECH-RLIN-001, field subtraction/scalar multiplication | Complete | Fails if theorem claims real-valued SGD. |
| MECH-ENC-001 | `canonical_encoding_injective_before_hash` | `Encoding.lean` | canonical encode definitions | Complete where modeled | Fails if hash collision resistance is confused with encoding injectivity. |
| MECH-SIG-001 | `accepted_statement_has_valid_signature_under_Sig` | `Signature.lean` | signature relation | Assumption-bound complete | Fails if current `sign` helper is treated as production security. |
| MECH-ATT-001 | `submit_attestation_success_implies_registered_assigned_validator` | `Attestation.lean` | state model, assignment function, signature relation | Complete for syntax | Fails if theorem says verifier execution occurred. |
| MECH-QUO-001 | `quorum_implies_unique_assigned_valid_statement_weight` | `Attestation.lean` | MECH-ATT-001, quorum definition | Complete for syntax | Fails if duplicate validators or unassigned validators can count. |
| MECH-SET-001 | `settle_epoch_only_settles_quorum_agreed_receipts` | `Settlement.lean` | MECH-QUO-001, conflict/unavailable predicates | Complete for v1 syntax | Fails if theorem implies v2 block inclusion. |
| MECH-DA-001 | `root_matched_fetch_inserts_matching_tensor` | `DataAvailability.lean` | commitment root definition, decode relation | Complete for local retrieval | Fails if theorem claims public availability. |

## Dependency Order

1. `Field.lean`
2. `Tensor.lean`
3. `Matmul.lean`
4. `RandomOracle.lean`
5. `Freivalds.lean`
6. `RandomLinear.lean`
7. `LinearStep.lean`
8. `Encoding.lean`
9. `Signature.lean`
10. `Attestation.lean`
11. `Settlement.lean`
12. `DataAvailability.lean`
13. `SoundKernel.lean`
14. `BlockedConsensus.lean`

This order keeps algebraic proofs independent of chain-state proofs and keeps blocked consensus goals out of
the sound kernel.

## Required Cross-Checks Against Rust

Before a theorem can be marked mechanized and implementation-backed, require:

1. Rust function or transition name is listed beside the theorem.
2. Input/output shape conventions match Rust exactly.
3. Error cases are either modeled or explicitly outside the theorem premise.
4. Hash domains used by Rust are listed when the theorem depends on transcript binding.
5. Existing Rust tests are listed as regression evidence, not as proof.
6. Any theorem involving signatures imports the signature assumptions.
7. Any theorem involving randomness imports the random-vector assumptions.
8. Any theorem involving chain state states whether it models current v1 behavior or future v2 behavior.

## Blocked Consensus Stubs

These names may be reserved, but they must remain unproved or explicitly assumed until the implementation
surface exists:

```text
canonical_settled_receipt_blockspace
block_checks_root_recomputable
useful_verification_pow_valid
finality_implies_v2_block_valid
zero_receipt_pow_skip_fallback_live
public_data_availability_retention
operator_independence_for_quorum
production_signature_authentication
```

Each blocked stub should point back to `mvp_core_negative_proofs.md` and `mvp_core_proof_completion_audit.md`
so nobody mistakes the name for a completed theorem.

## Definition Of Done For Mechanization

The kernel mechanization is done only when:

1. Every theorem in the checklist has a completed proof or an explicit assumption-bound theorem.
2. `SoundKernel.lean` imports no blocked consensus theorem.
3. `BlockedConsensus.lean` contains no admitted proof that would imply current v2 consensus soundness.
4. Every assumption appears in a visible assumptions section and is referenced by theorem name.
5. Every theorem has a Rust evidence note and a scope note.
6. The public docs still say the full MVP core is not sound until v2 consensus gates are implemented.

## Current Judgment

This checklist can move the proof work from paper-proof inventory to a concrete mechanization plan. It does
not make the MVP core sound by itself. The only mechanization that is honest today is the narrow sound
kernel; the reviewed v2 consensus theorem remains blocked.
