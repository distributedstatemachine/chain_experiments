# TensorVM MVP Core Settled-Receipt Blockspace Model

Status: documentation-only model for deterministic settled-receipt blockspace.

Purpose: define the state and theorem obligations behind `canonical_selected_receipts(parent_state, beacon,
caps)` and `settled_receipt_set_root`. A blockspace proof cannot be built from a bare map root or a set of
receipt ids. It needs lifecycle state, eligibility predicates, cap accounting, omission/carry-over rules,
and a selected receipt leaf schema.

This document is a proof target. It does not mark the current implementation sound.

## Current Verdict

The reviewed v2 blockspace theorem remains blocked.

Current proof-safe wording:

```text
current settlement can mark receipts as settled under syntactic quorum rules.
```

Unsafe wording:

```text
the settled receipt set is canonical v2 blockspace.
```

The missing distinction is that settlement says a receipt may become eligible for future blockspace.
Blockspace selection says exactly which eligible receipts are included in a specific block, which are carried
over, and which are excluded, expired, challenged, or spent. Those are different theorems.

## Required Objects

| Object | Meaning | Required For |
| --- | --- | --- |
| `SettledReceiptRecord` | Consensus record for one settled receipt after quorum settlement. | Eligibility, selected leaf, rewards, expiry. |
| `SettledReceiptPool` | Parent-state map of settled receipt records keyed by receipt id. | Deterministic selection and carry-over. |
| `BlockspaceCaps` | Parent-state or versioned parameters for TWU, byte, and receipt-count caps. | Cap-respecting selection. |
| `SelectedReceiptLeaf` | Canonical encoded leaf for a receipt included in a block. | `settled_receipt_set_root` binding. |
| `ReceiptLifecycleState` | Eligible, selected, spent, challenged, expired, pruned, or carried over. | No double inclusion and reward safety. |
| `SelectionRuleVersion` | Version of ordering, cap, and leaf semantics. | Replay and proof stability across upgrades. |

If any of these objects is implicit, the theorem must stay blocked.

## Settled Receipt Record

A v2 settled receipt record should bind, or be injectively derived from, at least:

```text
receipt_id
receipt_hash
job_id
primitive_type
miner
tensor_work_units
estimated_block_bytes
settled_height
settled_epoch
expiry_height_or_epoch
data_availability_status
challenge_status
included_block_hash_or_none
included_height_or_none
reward_status
validation_seed_anchor
```

The proof may allow implementation-specific compression, but it must show that every field needed for
eligibility, cap accounting, selected roots, rewards, and challenge windows is reconstructible from parent
state.

## Eligibility Predicate

The eligible set must be a deterministic function of parent state:

```text
eligible_for_blockspace(S_parent, receipt_id) =
  receipt_id in S_parent.settled_receipt_pool
  && receipt.lifecycle = settled
  && receipt.included_block_hash = none
  && receipt.expiry_height_or_epoch > S_parent.height_or_epoch
  && receipt.data_availability_status covers verification_and_challenge_window
  && receipt.challenge_status not in {proven_invalid, frozen}
  && receipt.reward_status not final_paid_for_prior_inclusion
```

Bad assumptions rejected:

- "Settled" automatically means eligible forever.
- Data available at attestation time means available through the challenge window.
- A receipt can be included repeatedly until pruned.
- A missing expiry field is equivalent to no expiry risk.

## Deterministic Ordering

The reviewed MVP ordering is:

```text
order_key = H(selection_domain || beacon || parent_hash || receipt_id)
```

Required theorem:

```text
same_parent_state_and_beacon -> same_ordered_eligible_list
```

The beacon must be parent-state valid. If the proposer can choose `beacon`, the selected set is grindable.

Tie-breaking must be explicit. A safe rule is:

```text
sort by (order_key, receipt_id)
```

## Cap Semantics

The spec and implementation must choose one cap policy. There are at least two possible policies:

| Policy | Rule | Consequence |
| --- | --- | --- |
| Stop-before-first-exceed | Iterate ordered eligible receipts and stop when the next receipt would exceed any cap. | Simple prefix theorem; one large receipt can block later smaller receipts until it expires or is challenged. |
| Skip-over-exceeding | Iterate ordered eligible receipts and skip any receipt that would exceed remaining caps. | Better utilization; more complex omission theorem and fairness story. |

The proof cannot treat these as interchangeable. The selector theorem must name the policy:

```text
canonical_selected_receipts(S_parent, beacon, caps, policy)
```

Required cap checks:

```text
sum(twu(selected)) <= caps.max_tensor_work_units
sum(bytes(selected)) <= caps.max_bytes
len(selected) <= caps.max_receipts
```

For every omitted eligible receipt, the theorem must prove whether omission happened because of cap policy,
expiry/challenge/pruning, or carry-over.

## Selected Receipt Leaf

`settled_receipt_set_root` should be a root over canonical selected receipt leaves, not only receipt ids.

Minimum leaf schema:

```text
leaf_version
selection_rule_version
selected_index
receipt_id
receipt_hash
job_id
primitive_type
miner
tensor_work_units
estimated_block_bytes
settled_height
validation_seed_anchor
challenge_window_end
```

The root theorem:

```text
B.settled_receipt_set_root =
  MerkleRoot(selected_receipt_leaf(r_i, i) for i in canonical_selected_receipts(S_parent, B.beacon, caps))
```

Receipt id may indirectly bind some fields if the receipt hash theorem is imported, but the selected leaf
should still include the fields used by blockspace and reward proofs. Otherwise the selected root does not
explain why a receipt was eligible, how much capacity it consumed, or which reward rule applies.

## Carry-Over And Spent Rules

Applying a valid block must update receipt lifecycle deterministically:

```text
for r in selected:
  r.included_block_hash = B.hash
  r.included_height = B.height
  r.lifecycle = included_pending_challenge_or_spent

for r in eligible_not_selected:
  r remains eligible_carry_over unless expired_or_challenged_by_rule

for r in expired:
  r.lifecycle = expired_or_pruned
```

Required safety theorems:

```text
selected_receipts_are_spent_or_included_once
nonselected_eligible_receipts_carry_over
expired_receipts_do_not_reenter_blockspace
challenged_invalid_receipts_do_not_reenter_blockspace
```

Without these, selected receipts can be included twice, omitted receipts can disappear silently, and reward
settlement can become detached from block inclusion.

## Omission Proof

For a selected set `R`, the theorem must explain every parent-state eligible receipt:

```text
forall r in eligible(S_parent):
  r in R
  || omitted_by_cap_policy(r, R, caps, ordered_eligible_list)
  || expired_or_challenged_during_apply(r)
```

This is the anti-censorship and determinism bridge. A root over selected receipts does not prove canonical
selection unless every omitted eligible receipt is accounted for.

## Relationship To Rewards

Receipt inclusion and reward finality are separate:

```text
included_in_block(r, B) -> eligible_for_reward_tracking(r, B)
reward_paid(r) -> included_in_valid_block(r) && challenge_window_closed_or_directly_verified(r)
```

The selected receipt root can prove inclusion. It does not prove final reward entitlement unless the reward
transition imports verifier evidence, challenge-window state, and public DA assumptions where needed.

## Theorem Split

| ID | Target Theorem | Depends On | Current Status |
| --- | --- | --- | --- |
| BLKSPACE-001 | `SettledReceiptRecord` carries all fields needed for eligibility and caps. | receipt state schema | Blocked. |
| BLKSPACE-002 | Eligibility is deterministic from parent state. | lifecycle, DA, challenge, expiry predicates | Blocked. |
| BLKSPACE-003 | Ordered eligible list is deterministic for parent state and valid beacon. | hash ordering, beacon model | Blocked. |
| BLKSPACE-004 | Selector respects named cap policy. | caps, policy version | Blocked. |
| BLKSPACE-005 | Selected receipt root binds canonical selected leaves. | leaf schema, encoding, hash assumption | Blocked. |
| BLKSPACE-006 | Applying a block spends/includes selected receipts exactly once. | parent-state transition model | Blocked. |
| BLKSPACE-007 | Nonselected eligible receipts carry over unless expired or challenged. | lifecycle transition | Blocked. |
| BLKSPACE-008 | Every omission is explained by cap policy or lifecycle rule. | selector theorem | Blocked. |

## Candidate Implementation Caveat

The local implementation contains a partial selector and selected root field, but the proof should not
upgrade yet:

- the selector lacks full settled receipt lifecycle state;
- the selected root is over receipt ids rather than full selected leaves;
- cap-exceeding receipt behavior must be reconciled with the spec;
- selected receipts are not proven spent/included exactly once;
- omissions do not yet have a formal carry-over theorem.

## Bad Assumptions Rejected

1. A bare `BTreeSet<Hash>` of settled receipt ids is a settled receipt pool.
2. A receipt id root proves blockspace eligibility, cap accounting, and reward entitlement.
3. Cap policy is an implementation detail that need not appear in the theorem.
4. Skipped receipts and stopped-before receipts have the same proof semantics.
5. Nonselected receipts can disappear without a carry-over or expiry theorem.
6. Selected receipts cannot be double-included unless there is explicit spent/included state.
7. Data-available once means data-available through the challenge window.
8. Selected receipt inclusion proves immediate reward finality.

## Discharge Gate

Do not mark V2-BLK-001 or V2-BLK-002 proof-ready until:

1. `SettledReceiptRecord` or equivalent state exists with eligibility, cap, expiry, DA, challenge, and
   inclusion fields.
2. `BlockspaceCaps` and `SelectionRuleVersion` are consensus parameters or committed constants.
3. The cap policy is explicitly specified and adversarially tested.
4. `SelectedReceiptLeaf` is defined and its encoding is injective before hashing.
5. The selected root is recomputed from canonical selected leaves.
6. Applying a valid block marks selected receipts spent/included exactly once.
7. Nonselected eligible receipts carry over unless a named expiry/challenge/pruning rule applies.
8. Omission proofs account for every eligible parent-state receipt.
9. Reward theorems import inclusion plus challenge-window or direct-verification finality.

## Current Judgment

The blockspace theorem is the consensus object that turns settled receipts into useful-PoW work. Without a
settled receipt pool, selected receipt leaves, explicit cap policy, carry-over/spent semantics, and omission
proofs, a block can commit to some receipt ids without proving it used the canonical blockspace.
