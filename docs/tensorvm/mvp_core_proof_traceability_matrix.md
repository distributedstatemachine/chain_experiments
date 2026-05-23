# TensorVM MVP Core Proof Traceability Matrix

Status: documentation-only traceability matrix compiled from the current worktree.

Purpose: tie every current proof claim to the Rust surface, evidence class, allowed wording, bad assumption,
and next gate. This document is a control surface for avoiding accidental overclaiming. It is not a proof by
itself and it does not upgrade any blocked theorem.

Assumption categories and discharge gates are tracked in
[`mvp_core_assumption_discharge_plan.md`](mvp_core_assumption_discharge_plan.md). A traceability row can
move to a stronger status only when its assumption-discharge category has been satisfied.
The adversary model used to interpret those assumptions is stated in
[`mvp_core_adversary_model.md`](mvp_core_adversary_model.md).
The theorem dependency graph is maintained in
[`mvp_core_theorem_dependency_graph.md`](mvp_core_theorem_dependency_graph.md).
Verifier-local probability budgets are recorded in
[`mvp_core_probabilistic_soundness_budget.md`](mvp_core_probabilistic_soundness_budget.md).
Receipt-lifecycle seed requirements are specified in
[`mvp_core_receipt_lifecycle_seed_model.md`](mvp_core_receipt_lifecycle_seed_model.md).
Signature/authentication boundaries are specified in
[`mvp_core_signature_authentication_boundary.md`](mvp_core_signature_authentication_boundary.md).
Canonical encoding and commitment boundaries are specified in
[`mvp_core_canonical_encoding_commitment_model.md`](mvp_core_canonical_encoding_commitment_model.md).
V2 state invariants are tracked in
[`mvp_core_v2_state_invariants.md`](mvp_core_v2_state_invariants.md).

## Status Key

| Status | Meaning |
| --- | --- |
| Defensible kernel | The claim is inside the current sound kernel under stated assumptions. |
| Assumption-bound | The claim can be stated only with explicit cryptographic, randomness, availability, or honesty assumptions. |
| Syntactic only | The chain proves a signed/structured statement, not the semantic truth of the statement. |
| Reference-only | Current behavior is useful local/v1 behavior, not reviewed v2 MVP consensus. |
| Blocked | Required code object or transition does not exist. |
| Contradicted | Current implementation admits a counterexample to the stronger claim. |
| Missing evidence | External, public, or mechanized evidence is absent. |

## Sound-Kernel Traceability

| Claim ID | Current Claim | Rust Surface | Evidence Docs | Status | Allowed Wording | Bad Assumption To Reject | Next Gate |
| --- | --- | --- | --- | --- | --- | --- | --- |
| K-FIELD-001 | Consensus tensor operations are deterministic finite-field operations under the modeled CPU semantics. | `tensor.rs`, `field.rs`, `vm.rs` | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md` | Defensible kernel | "Canonical CPU/field semantics are deterministic." | "GPU or framework execution is automatically canonical." | Mechanize field/tensor definitions and prove Rust/formal equivalence. |
| K-TOP-001 | Honest TensorOp matmul receipts are accepted when roots, shapes, metadata, and signature relation match. | `verify.rs::verify_tensor_op`, `verify.rs::full_freivalds` | `mvp_core_sound_kernel.md`, `mvp_core_formal_proofs.md` | Defensible kernel | "TensorOp completeness is proof-ready." | "Acceptance proves production authentication." | Mechanize matmul/Freivalds completeness and keep signature as an explicit relation. |
| K-TOP-002 | Invalid TensorOp matmul outputs are caught with Freivalds false-accept probability bound. | `verify.rs::full_freivalds`, `tensor.rs::random_field_vector` | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md` | Assumption-bound | "Freivalds gives a probabilistic bound under hidden uniform-enough challenges." | "Freivalds proves every cell deterministically." | Mechanize one-round and repeated Freivalds soundness with randomness assumptions visible. |
| K-ROW-001 | Row sampling detection probability is hypergeometric audit math. | `verify.rs::row_sample_detection_probability` | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md` | Defensible kernel as audit math | "Row sampling is audit telemetry." | "Row sampling alone is block-validity security." | Keep row sampling outside block eligibility unless parameters meet explicit bounds. |
| K-LIN-001 | LinearTrainingStep acceptance proves deterministic finite-field training-step relations. | `verify.rs::verify_linear_training_step`, `vm.rs` | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md` | Defensible kernel | "LinearTrainingStep proves field-algebra transition consistency." | "This proves real-valued SGD or useful ML training." | Mechanize field algebra equations and separately define any fixed-point/real bridge if wanted. |
| K-LIN-002 | Random-linear equality checks have a false-accept bound under uniform challenge. | `verify.rs::random_linear_equal` | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md` | Assumption-bound | "Random-linear checks are probabilistically sound under explicit assumptions." | "A hash-derived vector is automatically unbiasable." | Mechanize nonzero linear polynomial lemma and imported randomness assumptions. |
| K-COM-001 | Canonical encodings can be hash-bound to receipt/root preimages. | `types.rs::hash_bytes`, `chain/roots.rs`, receipt recompute functions | `formal_proof_manifest_v0.md`, `mvp_core_mechanization_checklist.md` | Assumption-bound | "Encoding injectivity is mechanizable before hash; hash binding is assumed." | "Lean proves SHA-256 collision resistance for this protocol." | Define canonical encodings and import collision-resistance assumption. |
| K-SIG-001 | Accepted statements satisfy the current signature relation. | `types.rs::sign`, `types.rs::verify_signature`, verifier/admission/vote checks | `bad_assumptions_ledger.md`, `formal_proof_manifest_v0.md` | Assumption-bound | "Reference signatures test message-flow shape." | "Reference `sign` is production authentication." | Replace or wrap with production signature model before making actor-control claims. |
| K-ATT-001 | Attestation admission success implies registered assigned validator, matching receipt metadata, signature relation, and no duplicate. | `chain/validation.rs::submit_attestation`, `scheduler.rs::assign_validators` | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md` | Syntactic only | "Admission proves an assigned validator signed a matching statement." | "Admission proves the validator ran the verifier." | Store receipt-lifecycle seed and bind attestation evidence to recomputable checks before semantic upgrade. |
| K-QUO-001 | Quorum counts unique assigned signed Valid/DataAvailable statements under current rules. | `chain/validation.rs::has_attestation_quorum` | `mvp_core_sound_kernel.md`, `mvp_core_negative_proofs.md` | Syntactic only | "Quorum proves assigned signed agreement." | "Quorum proves verified tensor work." | Add recomputable/challengeable check leaves or keep theorem explicitly syntactic. |
| K-SET-001 | Current settlement follows syntactic quorum, redundant-agreement, unavailable, and conflict checks. | `chain/settlement.rs` | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md` | Reference-only for v1 settlement | "Local v1 settlement follows syntactic quorum rules." | "Settlement proves v2 block inclusion." | Add v2 settled-receipt pool and blockspace transition before claiming v2 settlement. |
| K-DA-001 | Root-matched remote/local tensor fetch can support verifier-time artifact use. | `p2p.rs` request-response path, `main.rs::fetch_validator_role_missing_tensors`, `rpc.rs::tensor_by_commitment_root` | `mvp_core_data_availability_boundary.md` | Defensible kernel for local retrieval, assumption-bound for availability | "Verification-time artifact retrieval checks requested roots." | "Remote fetch proves public DA." | Add signed public retention measurements before public DA claims. |

## Blocked V2 Consensus Traceability

| Claim ID | Target Claim | Current Rust Surface | Evidence Docs | Status | Why Not Proven | Gate To Upgrade |
| --- | --- | --- | --- | --- | --- | --- |
| V2-BLK-001 | Canonical settled-receipt selection is deterministic and cap-respecting. | Current `ChainState` has `settled_receipts: BTreeSet<Hash>`. | `mvp_core_v2_consensus_proof_obligations.md` | Blocked | No settled-receipt pool metadata, eligibility, expiry, spent/carry-over, byte/TWU/count caps. | Add selector state and prove deterministic inclusion/omission. |
| V2-BLK-002 | Block commits `settled_receipt_set_root` for canonical selected receipts. | Current `TensorBlock` has `receipt_root`, not selected receipt root. | `mvp_core_negative_proofs.md`, `formal_proof_manifest_v0.md` | Blocked | Global receipt map root is not deterministic blockspace. | Add selected receipt root and canonical leaf encoding. |
| V2-CHK-001 | Validators can recompute check leaves for selected receipts. | Verifier reports and attestation `checks_root` exist, but no block-level transcript object. | `mvp_core_v2_consensus_proof_obligations.md` | Blocked | No check leaf format, transcript root format, or opening/challenge path. | Define check leaves and transcript recomputation. |
| V2-CHK-002 | Block-level `checks_root` binds all selected receipt checks. | Current `TensorBlock` has no `checks_root`. | `mvp_core_negative_proofs.md` | Blocked | Per-attestation roots do not aggregate canonical block verification. | Add block-level checks root and recomputation validation. |
| V2-POW-001 | Useful-PoW nonce is bound to parent, selected receipt root, checks root, beacon, proposer, and target. | `chain/blocks.rs::produce` has no PoW predicate. | `mvp_core_v2_consensus_proof_obligations.md` | Blocked | Current block has no target or nonce, and nonce search is not tied to verification. | Add PoW header, difficulty target, nonce predicate, and validation. |
| V2-PROP-001 | Proposer is registered validator useful-PoW winner. | `produce_block(proposer, timestamp)` accepts caller-supplied address; `chain/proposer.rs` uses settled TensorWork. | `mvp_core_negative_proofs.md` | Contradicted | Current path can select/append non-v2 proposers. | Replace normal proposer path with validator useful-PoW eligibility and reject arbitrary proposer append. |
| V2-STATE-001 | Valid v2 block transition determines state and reward roots. | Current roots are v1/reference roots over global maps. | `mvp_core_v2_consensus_proof_obligations.md` | Blocked | No v2 selected-receipt application, spent/carry-over mutation, or challenge-window reward semantics. | Define v2 block apply transition and root checks. |
| V2-FIN-001 | Vote admission requires `validate_block_v2`. | `chain/validation.rs::submit_block_vote` checks voters and known block hash. | `mvp_core_negative_proofs.md`, `formal_proof_manifest_v0.md` | Blocked | It does not validate canonical blockspace, checks root, PoW, or proposer eligibility. | Add `validate_block_v2` and require it before counting votes. |
| V2-FIN-002 | Finality implies v2 block validity. | `state.finalized_blocks` can be updated after current stake threshold. | `mvp_core_proof_completion_audit.md` | Contradicted / blocked | Current finality certifies v1 block hashes, not v2 validity. | Restrict finalized-set mutation to validated v2 votes or validated fallback. |
| V2-FALLBACK-001 | PoW-skip fallback has explicit timeout, validator rotation, reduced reward, and no miner TWU rewards. | Existing fallback belongs to v1 proposer selection. | `formal_proof_manifest_v0.md`, `mvp_core_v2_consensus_proof_obligations.md` | Missing evidence | v2 fallback state and transition are not started. | Define fallback object, timeout, reward rules, and tests/theorem. |

## Evidence Class Matrix

| Evidence Class | Counts For | Does Not Count For |
| --- | --- | --- |
| Rust unit/integration tests | Regression evidence for implemented behavior. | Formal proof of consensus theorem. |
| Paper proof docs | Claim boundary, theorem statements, assumptions, negative cases. | Mechanized proof or implemented behavior. |
| Mechanization checklist | Future proof work planning. | Current proof completion. |
| Local Compose evidence | Local multi-participant/runtime shape. | Public operator independence or public DA. |
| Remote tensor fetch counters | Verification-time retrieval observability. | Durable public data availability. |
| Tarpaulin/coverage | Lines exercised by tests. | Threat-model soundness. |
| Current v1 finality tests | Stake-signature threshold for current block hashes. | v2 useful-verification PoW validity. |

## Claim Approval Rules

A claim may be used in public or release-facing text only if all of these are true:

1. It appears in this matrix or the proof manifest.
2. Its status is not `Blocked` or `Contradicted`.
3. Its assumptions are stated in the same document or linked boundary doc.
4. Its wording matches the "Allowed Wording" column.
5. It does not depend on a dirty/uncommitted code change unless that change is explicitly cited as
   uncommitted evidence.
6. Any related assumption-discharge gate is satisfied or still stated as an assumption.

If any condition fails, phrase it as a gap or target, not as a property.

## Current Dirty Worktree Note

At the time this matrix was created, the worktree contained unrelated uncommitted code and status-doc
changes. This matrix treats those files as current-state evidence when inspected, but it does not commit or
validate those implementation changes. The proof status remains conservative until implementation changes
are committed, tested, and mapped back into this matrix.

## Current Judgment

The traceability picture is simple: the verifier-local kernel has a credible proof path under assumptions;
the reviewed v2 consensus layer remains blocked or contradicted by current v1/reference surfaces. No claim
that "the MVP core is formally proven sound" should pass review until every blocked v2 consensus row has a
real implementation surface, tests, and formal proof mapping.
