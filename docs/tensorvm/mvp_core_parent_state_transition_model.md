# TensorVM MVP Core Parent-State Transition Model

Status: documentation-only model for v2 parent-state validation, block application, and finality
certificates.

Purpose: define the missing transition theorem behind `valid_v2_block(parent_state, block)`. A block root
or validation helper is not enough unless every consensus-relevant field is interpreted against the exact
parent state and the resulting child state is deterministic.

This document is a proof target. It does not mark the dirty v2-block candidate or current implementation
sound.

Reward finality and challenge-window state used by `reward_root` are specified in
[`mvp_core_reward_finality_challenge_model.md`](mvp_core_reward_finality_challenge_model.md).

## Current Verdict

The reviewed v2 finality theorem is still blocked at the parent-state transition layer.

The required theorem shape is:

```text
ValidParent(S, B.parent_hash)
&& ValidV2Header(S, B)
&& ValidV2Body(S, B)
&& apply_v2_block(S, B) = S'
&& B.state_root = state_root(S')
&& B.reward_root = reward_root(S')
&& FinalityCertificate(S, B)
-> finalized_v2(B, S')
```

The false shortcut is:

```text
recompute roots from current node state and accept the block if they match
```

That shortcut can validate against the wrong state after later receipts, attestations, rewards, randomness,
or finalized blocks have changed. It also hides failed-transition atomicity bugs, where partial state
mutation can survive a rejected block.

## Required State Objects

The v2 transition theorem needs explicit state snapshots:

| Object | Meaning | Proof Role |
| --- | --- | --- |
| `S_parent` | The exact parent state identified by `B.parent_hash`. | All validation predicates read from this state. |
| `B` | Candidate v2 block. | Carries selected root, checks root, beacon, target, nonce, proposer, state root, reward root. |
| `S_child` | Deterministic result of applying `B` to `S_parent`. | Must match `B.state_root` and `B.reward_root`. |
| `Certificate` | Votes or fallback evidence that finalizes `B`. | Must prove votes were admitted only after v2 validation. |
| `ValidationContext` | Parameters, hash domains, signature domains, and cost/version settings. | Prevents validation rules from changing silently. |

The proof must reject any validation function whose inputs are only `(current_chain, block)` unless
`current_chain` is proven to expose the exact parent snapshot for `block`.

## Predicate Split

`validate_block_v2` should be a conjunction of named predicates, not a single opaque boolean.

```text
validate_block_v2(S_parent, B) =
  parent_link_valid(S_parent, B)
  && beacon_valid(S_parent, B.beacon)
  && proposer_valid(S_parent, B.proposer)
  && selected_receipts_valid(S_parent, B)
  && check_leaves_valid(S_parent, B)
  && useful_pow_structural_valid(S_parent, B)
  && state_transition_valid(S_parent, B)
  && reward_transition_valid(S_parent, B)
  && signatures_and_domains_valid(S_parent, B)
```

Each predicate has a different proof dependency and different adversarial tests. A finality theorem should
not import a weaker subset by accident.

## Parent-Link Rule

The parent rule must identify a unique parent state:

```text
parent_link_valid(S_parent, B) =
  state_hash(S_parent.last_block) = B.parent_hash
  && B.height = S_parent.height
  && B.epoch = epoch_of(S_parent.height)
```

or an equivalent rule for genesis.

Bad assumptions rejected:

- "Any local block with `candidate.height + 1 == B.height` can be the parent."
- "The current tip at vote time is necessarily the parent."
- "A parent hash check is enough without a parent-state snapshot."

## Beacon Rule

The beacon must be derived from parent state or a committed randomness transition:

```text
beacon_valid(S_parent, B.beacon) =
  B.beacon = S_parent.finalized_randomness
```

or:

```text
beacon_valid(S_parent, B.beacon) =
  B.beacon = beacon_transition(S_parent, parent_certificate)
```

The theorem should not allow the proposer to pick an arbitrary beacon that makes canonical receipt
selection or verifier challenges favorable.

## Body Rule

The body rule ties selected receipts and check evidence to the parent state:

```text
selected_receipts_valid(S_parent, B) =
  selected = canonical_selected_receipts(S_parent, B.beacon, caps)
  && B.settled_receipt_set_root = root(selected_receipt_leaf(r) for r in selected)

check_leaves_valid(S_parent, B) =
  leaves = recompute_or_validate_check_leaves(S_parent, B, selected)
  && B.checks_root = root(leaves)
```

This imports the canonical blockspace model and verifier evidence model. A root over signed check claims is
not enough for `check_leaves_valid`.

## Apply Rule

`apply_v2_block` must be deterministic and total on valid blocks:

```text
apply_v2_block(S_parent, B) = S_child
```

Required mutations:

1. Mark selected settled receipts as included or spent.
2. Preserve nonselected eligible receipts unless expired, challenged, or pruned by rule.
3. Advance height and epoch.
4. Update finalized randomness through a defined beacon transition.
5. Update block index and finality/certificate state.
6. Update reward state only according to reward-finality rules.
7. Update challenge-window state for selected receipt evidence.
8. Preserve unrelated maps and balances except through named transitions.

The proof must include a frame theorem:

```text
unrelated_state_fields_preserved(S_parent, S_child, B)
```

so that block application cannot smuggle unrelated state changes under a matching root.

## State Root Rule

`B.state_root` must bind the child state after application:

```text
state_transition_valid(S_parent, B) =
  let S_child = apply_v2_block(S_parent, B)
  B.state_root = state_root(S_child)
```

Do not validate `B.state_root` against `S_parent` unless the block intentionally represents a pre-state
root, and if so the field must be named as such. The reviewed v2 theorem needs the finalized block to imply
a deterministic child state.

## Reward Rule

Reward mutation must be atomic with successful block admission:

```text
reward_transition_valid(S_parent, B) =
  let S_child = apply_v2_block(S_parent, B)
  B.reward_root = reward_root(S_child.rewards)
  && rewards_change_only_by_valid_rules(S_parent, S_child, B)
```

Required restrictions:

- No proposer reward is credited before block validity succeeds.
- Miner and validator rewards that depend on verifier evidence wait for direct recomputation or challenge
  finality and remain pending until the reward-finality model settles them.
- Fallback blocks carry reduced proposer rewards and no miner TWU rewards for empty blockspace.
- Failed block production or failed vote admission leaves rewards unchanged.

## Finality Certificate Rule

Finality must carry a certificate, not only a hash in a set:

```text
FinalityCertificate(S_parent, B) =
  enough unique registered validator stake signed vote_domain(B.hash, B.height, S_parent.epoch)
  && each vote was admitted only after validate_block_v2(S_parent, B)
  && no duplicate validator counted
  && stake snapshot is from S_parent or a named finality snapshot
```

The finalized-set mutation theorem should be:

```text
B.hash in S_child.finalized_blocks ->
  exists S_parent, Certificate.
    validate_block_v2(S_parent, B)
    && FinalityCertificate(S_parent, B)
```

Direct mutation of `finalized_blocks` outside this path is a proof hole.

## Atomicity Rule

Every fallible transition needs a no-partial-mutation theorem:

```text
produce_or_admit_block(S, input) = Err(e) -> S_after = S
submit_vote(S, vote) = Err(e) -> S_after = S
```

If implementation uses mutation before validation, the proof must show rollback or refactor to validate
before mutating. Reward credit before block validation is a direct counterexample to this rule.

## Theorem Split

| ID | Target Theorem | Depends On | Current Status |
| --- | --- | --- | --- |
| PST-001 | Parent hash identifies the exact parent state used for validation. | block log/state snapshot model | Blocked. |
| PST-002 | Beacon is valid for the parent state. | randomness/beacon model | Blocked. |
| PST-003 | `validate_block_v2` imports all required predicates, not a weaker subset. | v2 predicate split | Blocked. |
| PST-004 | `apply_v2_block` is deterministic for valid blocks. | selected receipt lifecycle, rewards, beacon transition | Blocked. |
| PST-005 | `state_root` and `reward_root` bind the deterministic child state. | canonical encoding, apply theorem | Blocked. |
| PST-006 | Failed block production/admission is atomic. | implementation transition model | Blocked. |
| PST-007 | Vote admission stores a validation certificate for the exact parent/block pair. | finality vote model | Blocked. |
| PST-008 | Finalized block implies prior `validate_block_v2(S_parent, B)`. | PST-001 through PST-007 | Blocked. |

## Bad Assumptions Rejected

1. Matching roots against current state proves validity against parent state.
2. A block's beacon is valid because it is included in the block hash.
3. `state_root` proves a transition if it is computed before applying the block.
4. `reward_root` is safe if rewards are credited before block validation or before challenge-window reward
   finality.
5. Vote admission can skip state-transition roots and still prove finality validity.
6. Finalized hash membership is a certificate.
7. A build-clean local implementation is enough without adversarial tests for wrong parent, wrong beacon,
   wrong child root, failed admission atomicity, and duplicate inclusion.

## Discharge Gate

Do not mark V2-STATE-001, V2-FIN-001, or V2-FIN-002 proof-ready until:

1. `validate_block_v2(S_parent, B)` is represented as a named predicate or equivalent committed API.
2. Parent-state lookup is deterministic and replay-safe.
3. Beacon, target, selected receipts, checks, proposer, state transition, reward transition, and signatures
   are all validated from the parent state.
4. `apply_v2_block` mutates selected receipt lifecycle, challenge state, height/epoch, randomness, and
   rewards deterministically.
5. `state_root` and `reward_root` are child-state roots, or fields are renamed if they intentionally mean
   pre-state roots.
6. Failed production/admission/vote paths have no partial state mutation.
7. Finality certificates retain enough evidence to prove votes imported v2 validation.
8. Adversarial tests cover wrong parent, wrong beacon, wrong selected root, wrong checks root, wrong state
   root, wrong reward root, invalid proposer, invalid nonce/target, failed reward mutation, and direct
   finalized-set mutation.

## Current Judgment

The next soundness upgrade is not another root field. It is a parent-state transition theorem. Until the
chain can prove that finality votes were admitted for a block validated against its exact parent state and
that applying the block deterministically produced the committed child roots, the reviewed v2 finality
theorem remains blocked.
