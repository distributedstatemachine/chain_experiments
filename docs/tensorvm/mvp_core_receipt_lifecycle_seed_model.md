# TensorVM MVP Core Receipt-Lifecycle Seed Model

Status: documentation-only seed model for the verifier soundness boundary.

Purpose: make the hidden-challenge assumption precise. Freivalds and random-linear soundness require the
miner to commit to outputs before validation challenges are known. In TensorVM, that requirement should be
enforced by a receipt-lifecycle validation seed that stays fixed for the receipt. The current implementation
does not yet expose that seed as stable receipt state.

This document does not change code and does not mark the full core sound. It records the model that future
implementation and formal proofs must satisfy.

## Current Evidence

Current Rust behavior:

- `validation::seed(finalized_randomness, receipt_id)` derives the verifier seed from finalized randomness
  and the receipt id.
- `LocalChain::validation_seed(receipt_id)` calls `validation::seed(&state.finalized_randomness,
  receipt_id)`.
- `chain::validation::assigned_validators` calls `JobScheduler::assign_validators(chain, receipt_id,
  &chain.state.finalized_randomness)`.
- `submit_attestation` and `has_attestation_quorum` use assignment recomputed from current chain
  `finalized_randomness`.
- Verifier functions receive a `validation_seed` argument; the chain does not persist a receipt-local seed
  alongside the receipt.

This is deterministic for a fixed chain state, but it is not a lifecycle-stable receipt seed.

## Required Model

For every admitted receipt `r`, define:

```text
receipt_id(r)          hash-bound receipt identity
commitment_time(r)     height or state at which the receipt is admitted
seed_anchor(r)         finalized randomness or beacon fixed at commitment_time(r)
validation_seed(r)     H("tensor-vm-validation-seed-v2", seed_anchor(r), receipt_id(r))
assignment_seed(r)     H("tensor-vm-assignment-seed-v2", seed_anchor(r), receipt_id(r))
```

The key invariant is:

```text
admit_receipt(S, r) = S'
->
S'.receipts[r.id].validation_seed = validation_seed(r)
and future transitions never mutate that seed
```

Validator assignment, verifier challenge derivation, check leaves, and attestation admission must all use
the stored lifecycle seed or an equivalent immutable anchor.

## Theorems Needed

| ID | Theorem | Status Today | Required Evidence |
| --- | --- | --- | --- |
| SEED-001 | Receipt admission fixes a validation seed before verifier challenges are derived. | Implementation-blocked. | Stored seed or immutable anchor in receipt state. |
| SEED-002 | Receipt output commitments are bound before the seed is used for challenges. | Assumption-bound. | Receipt id/root binding plus admission order theorem. |
| SEED-003 | Delayed attestation uses the original receipt seed, not current finalized randomness. | Contradicted/blocked by current assignment path. | Attestation admission reads stored receipt seed/anchor. |
| SEED-004 | Validator assignment is stable for a receipt across later beacon changes. | Contradicted/blocked by current assignment recomputation. | Assignment function parameterized by stored seed/anchor. |
| SEED-005 | Freivalds/random-linear challenge domains are derived from the lifecycle seed with distinct tags. | Formalizable after implementation. | Domain tag inventory and sampling proof. |
| SEED-006 | A check leaf binds the seed used to derive every probabilistic check. | Implementation-blocked for v2. | Check leaf schema and block-level `checks_root`. |

## Current Counterexample

Witness:

```text
Receipt r is admitted while finalized_randomness = R0.
The chain advances and finalized_randomness becomes R1.
An attestation for r is submitted after the randomness change.
```

Current admission recomputes assigned validators using `R1`, not a seed fixed at receipt admission. A proof
that says `receipt_id(r)` is always validated under `R0` cannot be stated over the current chain state.

This breaks two proof bridges:

1. The Freivalds/random-linear non-adaptivity premise is not a state invariant.
2. The syntactic attestation theorem is stable only relative to current assignment, not receipt-lifecycle
   assignment.

## Allowed Claim Today

The honest claim is:

```text
Verifier soundness is proven under a hidden lifecycle-stable challenge seed assumption. Current chain
admission uses deterministic current-state assignment and does not yet persist that lifecycle seed.
```

Do not claim:

```text
Current attestations prove validation under the receipt's original challenge seed.
```

## Required State Fields Or Equivalent Anchors

One of these must exist before the seed assumption is implementation-discharged:

| Option | Required Data | Proof Burden |
| --- | --- | --- |
| Stored seed | `validation_seed`, `assignment_seed`, seed domain/version. | Prove stored seeds are set at receipt admission and immutable. |
| Stored seed anchor | receipt admission height, parent/finality beacon, seed domain/version. | Prove every node recomputes the same seed from immutable state. |
| Transcript anchor | check leaf includes seed anchor and challenge transcript root. | Prove check leaves cannot be replayed under a different seed. |

The stored seed option is simplest for proof review. The anchor option can be equivalent if the anchor is
immutable and unambiguous.

## Interaction With v2 Block Validity

The v2 `check_leaf` and `checks_root` proofs should import the seed invariant:

```text
check_leaf_recomputable(r, B)
requires:
  seed_used_by_verifier = stored_validation_seed(r)
  assignment_used_by_attestation = stored_assignment_seed(r)
```

Without this, a block can aggregate statements whose assignments or verifier challenges are judged under a
different beacon than the receipt lifecycle intended.

## Bad Assumptions Rejected

| Bad Assumption | Why It Is Wrong |
| --- | --- |
| `hash(current_finalized_randomness, receipt_id)` is lifecycle-stable. | Current finalized randomness can change after receipt admission. |
| The receipt id alone fixes the verifier challenge. | The current seed also depends on finalized randomness. |
| Delayed attestations are harmless because assignment is deterministic. | Deterministic under the wrong seed is still the wrong lifecycle theorem. |
| Validator quorum can be multiplied into Freivalds probability without seed stability. | Different validators are not proven to use immutable independent receipt challenges. |
| A future `checks_root` can omit the seed. | The transcript must bind the exact seed/challenge domain used for every check. |

## Discharge Gate

Do not mark `A-COMMIT-BEFORE-CHALLENGE`, `AD-005`, `INV-002`, or `SEED-*` discharged until:

1. Receipt state stores a validation seed or immutable seed anchor.
2. Validator assignment for a receipt uses that stored seed/anchor.
3. Verifier challenge derivation uses that stored seed/anchor.
4. Check leaves include the seed domain/version or a transcript root derived from it.
5. Delayed attestation tests cover finalized-randomness changes after receipt admission.
6. The proof manifest maps seed stability to Freivalds and random-linear soundness.
7. The negative proof `CEX-006` no longer constructs.

## Current Judgment

The receipt-lifecycle seed is one of the smallest but most important unsound edges in the current proof
story. The verifier algebra is credible only under hidden committed challenges; the chain needs a stable
seed model before that assumption becomes an implementation-backed invariant.
