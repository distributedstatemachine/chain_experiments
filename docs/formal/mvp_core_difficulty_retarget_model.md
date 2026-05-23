# TensorVM MVP Core Difficulty Retarget Model

Status: documentation-only difficulty and retarget proof model for the reviewed v2 MVP core.

Purpose: define when a v2 block's `difficulty_target` is valid. A nonce predicate is only meaningful if
the target is derived from parent consensus state, bounded by retarget rules, and compared against the PoW
hash with unambiguous byte/number semantics.

This document refines `UPOW-005` and the difficulty side of `V2-POW-001`. It does not prove useful-work
dominance, implement retargeting, or discharge public performance assumptions.

Related documents:

- [`mvp_core_useful_pow_work_model.md`](mvp_core_useful_pow_work_model.md) defines structural useful-PoW
  validity and the separate economic work-dominance assumption.
- [`mvp_core_parent_state_transition_model.md`](mvp_core_parent_state_transition_model.md) defines why the
  target must be validated against the exact parent state.
- [`mvp_core_fallback_liveness_model.md`](mvp_core_fallback_liveness_model.md) defines the liveness path
  when useful-PoW is unavailable.
- [`mvp_core_candidate_v2_block_audit.md`](mvp_core_candidate_v2_block_audit.md) records that the local
  v2-block reference path uses a static helper, not parent-state difficulty.

## Current Verdict

Difficulty validity is now paper-specified, but still implementation-blocked.

The safe claim today is:

```text
The docs define the target difficulty/retarget proof obligation.
Current/candidate v2 code does not discharge parent-state target validity or retarget economics.
```

The unsafe claim is:

```text
If a block's nonce is below the block's own target, useful-PoW difficulty is valid.
```

The target must not be self-authenticating. It must be derived from parent state and protocol parameters.

## Target Convention

The proof model uses the predicate:

```text
hash_to_uint(pow_hash) < difficulty_target
```

With this convention:

- a larger numeric target is easier;
- a smaller numeric target is harder;
- `hardest_allowed_target <= difficulty_target <= easiest_allowed_target`;
- `easiest_allowed_target` prevents trivial nonce search;
- `hardest_allowed_target` preserves liveness on the target hardware/network assumptions.

Docs and code must avoid ambiguous phrases like "difficulty floor" unless they state whether the floor is
over inverse difficulty or numeric target. The theorem should use the target inequality above.

## Required State

The future v2 consensus state needs a versioned difficulty object.

| Object | Required Fields | Proof Role |
| --- | --- | --- |
| `DifficultyParams` | target block time, retarget interval, max adjustment ratio, hardest/easiest target bounds, target encoding version, hash-to-uint version. | Makes target derivation deterministic and reviewable. |
| `DifficultyState` | current target, last retarget height/epoch, last retarget timestamp/slot, observed window accumulator, params version. | Parent-state source of the block target. |
| `RetargetWindow` | first/last block height, first/last time or slot, finalized block count, fallback count, useful-PoW count. | Provides deterministic observations for the next target. |
| `WorkFloorParams` | minimum selected receipt count/TWU/byte cost or verification-cost estimate for normal useful-PoW. | Prevents empty or tiny blocks from using the normal useful-PoW path. |
| `DifficultyCertificate` | parent hash, parent difficulty state root, derived target, params version, optional retarget event data. | Lets validation and finality show why the target was accepted. |

If wall-clock time is used, the clock source and skew bounds become consensus assumptions. If slot or height
time is used, liveness and retarget responsiveness must be stated under that model instead.

## Validity Predicate

A block target is valid only if:

```text
target_valid(S_parent, B) :=
  B.difficulty_target == expected_target(S_parent.difficulty_state, B.height)
  && target_bounds_valid(S_parent.params, B.difficulty_target)
  && target_encoding_valid(B.difficulty_target)
  && hash_to_uint_semantics_fixed(S_parent.params)
```

The structural useful-PoW predicate must import `target_valid` before checking the nonce:

```text
valid_nonce(S_parent, B) :=
  target_valid(S_parent, B)
  && hash_to_uint(H(pow_header(B) || B.nonce)) < B.difficulty_target
```

If `B.difficulty_target` is accepted from the block without this parent-state predicate, the prover can pick
an easy target and satisfy the nonce predicate cheaply.

## Retarget Function

The retarget function should be deterministic:

```text
expected_target(D, height) =
  if height is not retarget boundary:
    D.current_target
  else:
    clamp_target(
      D.current_target * bounded_ratio(observed_window_time(D) / target_window_time(D.params)),
      D.params.hardest_allowed_target,
      D.params.easiest_allowed_target
    )
```

Required details:

1. `bounded_ratio` must be clamped by `max_adjustment_ratio`.
2. `observed_window_time` must be derived from finalized parent-chain data, not untrusted candidate block
   fields.
3. `clamp_target` must respect the numeric-target convention above.
4. Retarget inputs must be replayable from parent state.
5. Genesis and first-window behavior must have explicit targets and parameter versions.
6. Fallback blocks must either be included, excluded, or weighted by a deterministic rule.
7. Parameter changes must be governed state transitions, not local config drift.

The exact formula can change, but any change must be versioned and hash-bound through the block/header
parameter version.

## Work Floor Coupling

Difficulty alone does not make work useful. A hard nonce target over an empty `checks_root` is ordinary PoW.

Normal nonfallback blocks need:

```text
nonfallback_work_floor(S_parent, B) :=
  selected_receipt_count(S_parent, B) >= min_count
  && selected_receipt_twu(S_parent, B) >= min_twu
  && estimated_verification_cost(S_parent, B) >= min_verification_cost
```

The policy can use count, TWU, bytes, measured verifier cost, or another consensus-visible proxy. It must be
deterministic from parent state and selected receipt leaves. Empty or below-floor cases go to the fallback
model and must not receive useful-PoW wording or normal useful-PoW rewards.

## Theorem Split

### DIFF-001: Target Is Parent-State Derived

Statement:

```text
If target_valid(S_parent, B), then B.difficulty_target equals the target derived from
S_parent.difficulty_state and the committed difficulty parameter version.
```

Current status: `implementation-blocked`.

### DIFF-002: Retarget Is Bounded

Statement:

```text
Across a retarget boundary, the next target changes by no more than max_adjustment_ratio and remains between
hardest_allowed_target and easiest_allowed_target.
```

Current status: `implementation-blocked`.

### DIFF-003: Hash-To-Target Semantics Are Canonical

Statement:

```text
Every honest node interprets the PoW hash and difficulty target as the same unsigned integer values before
checking hash_to_uint(pow_hash) < difficulty_target.
```

Current status: `formalizable after encoding schema`, implementation-blocked for consensus use.

### DIFF-004: Nonfallback Work Floor Excludes Empty Useful-PoW

Statement:

```text
No valid nonfallback useful-PoW block can have empty or below-floor selected verification work.
```

Current status: `implementation-blocked`.

### DIFF-005: Static Difficulty Is Local-Only Evidence

Statement:

```text
A static test target can support local regression tests, but it does not discharge parent-state retargeting,
liveness, or useful-work economics.
```

Current status: `paper-specified`.

### DIFF-006: Difficulty Updates Are Consensus State

Statement:

```text
Any parameter or target update that affects block validity is represented as a deterministic state
transition and is included in child state/root validation.
```

Current status: `implementation-blocked`.

## Attack Cases

| Attack | Required Rejection |
| --- | --- |
| Candidate block supplies an easy target. | Reject unless target equals parent-derived expected target. |
| Retarget changes by an unbounded amount after slow blocks. | Clamp by max adjustment ratio and hardest/easiest target bounds. |
| Nodes disagree on hash endianness or target encoding. | Version and test hash-to-uint semantics and boundary vectors. |
| Empty selected set mines normal useful-PoW. | Reject through nonfallback work floor; use fallback instead. |
| Local config changes target without state transition. | Reject because difficulty params are consensus state. |
| Fallback periods collapse future useful-PoW into trivial targets. | Use deterministic fallback weighting/exclusion and target bounds. |
| Static local target is marketed as production retargeting. | Keep static target wording local/test-only. |

## Required Tests Before Upgrade

The proof status cannot move beyond paper-specified until tests cover at least:

1. Blocks with targets easier than the parent-derived target are rejected.
2. Blocks with targets harder than the parent-derived target are rejected unless the retarget formula derives
   them.
3. Non-retarget heights require the exact parent target.
4. Retarget-boundary heights apply the bounded formula exactly.
5. Target values are clamped to hardest/easiest allowed bounds.
6. Hash-to-target comparison has boundary vectors for zero, one, target minus one, target, target plus one,
   and maximum hash.
7. Endianness and target encoding are fixed by canonical test vectors.
8. Empty and below-floor selected sets are rejected as normal useful-PoW.
9. Fallback blocks do not update difficulty except by the specified rule.
10. Parameter-version changes alter validation roots and cannot occur by local config only.
11. Static target helpers are quarantined to tests/local mode or explicitly marked as local evidence.

## Bad Assumptions Added

This model adds or sharpens these bad assumptions:

- "A block target is valid because the nonce is below it."
- "Static local difficulty proves production useful-PoW economics."
- "Retargeting can be specified by config without being consensus state."
- "A target floor/ceiling is clear without declaring the numeric target convention."
- "Hash byte comparison is obvious and does not need boundary vectors."
- "Fallback or empty periods can safely retarget useful-PoW without a deterministic policy."

Correct framing:

```text
Difficulty validity is a parent-state theorem.
A valid nonce proves only hash-target success against the consensus-derived target.
Useful-work economics and proposer-local work remain separate assumptions.
```

## Discharge Gate

Do not classify difficulty validity or `UPOW-005` as locally proof-ready until all of these are true:

1. Parent state contains versioned `DifficultyState` and `DifficultyParams`.
2. `B.difficulty_target` is derived from parent state during block validation.
3. Retarget input data is replayable from finalized parent-chain state.
4. Retarget changes are bounded and clamped to hardest/easiest target bounds.
5. Hash-to-uint and target encoding semantics have canonical boundary vectors.
6. Nonfallback work floor is deterministic from selected receipt leaves and parent params.
7. Fallback treatment in retarget windows is specified.
8. Difficulty parameter changes are consensus state transitions.
9. Tests reject easy target injection, wrong retarget boundary, encoding mismatch, empty normal useful-PoW,
   and local-config-only target changes.
10. Public wording separates valid nonce, target validity, useful-work dominance, and proposer-local
    verification claims.
