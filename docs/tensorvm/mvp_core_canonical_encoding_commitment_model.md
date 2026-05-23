# TensorVM MVP Core Canonical Encoding And Commitment Model

Status: documentation-only encoding and commitment proof boundary compiled from the current worktree.

Purpose: separate three things that are easy to conflate:

1. Canonical pre-hash encoding.
2. Hash/Merkle binding under cryptographic assumptions.
3. Consensus meaning of the object being committed.

The current proof corpus can use existing roots as deterministic local commitments, but not every current
root commits the object needed by the reviewed v2 MVP theorem.

The selected-receipt object that a future `settled_receipt_set_root` must encode is specified in
[`mvp_core_settled_receipt_blockspace_model.md`](mvp_core_settled_receipt_blockspace_model.md).

## Current Evidence

Current Rust surfaces:

- `hash_bytes(domain, parts)` length-prefixes the hash domain and each part before hashing.
- Tensor ids hash tensor shape, dtype, layout, and field-element values.
- Tensor commitment roots are Merkle roots over chunk leaves that bind tensor id, chunk index, and chunk
  bytes.
- Job, receipt, attestation, state, reward, and finality roots encode deterministic Rust maps/sets and hash
  the resulting bytes under domain tags.
- Current `receipt_root` commits the global receipt map.
- Current `settled_receipt_root` commits only a set of settled receipt ids.
- Current blocks do not contain a v2 `settled_receipt_set_root` or aggregate `checks_root`.

## Proof Layers

| Layer | What Can Be Proved Locally | What Remains Assumed Or Missing |
| --- | --- | --- |
| Canonical byte encoding | Given a fixed encoder, each modeled object maps to one byte string. | Need an injectivity proof for every encoded object shape used in theorem statements. |
| Domain separation | Different hash domains are syntactically distinct where domains are enumerated. | Hash security still depends on the cryptographic model. |
| Hash binding | If hashes collide only negligibly, an accepted root binds its encoded preimage. | Collision resistance is a permanent assumption, not a Lean theorem. |
| Merkle binding | A proof binds a leaf to a root under the Merkle hash construction. | The leaf schema and tree construction must be modeled exactly. |
| Consensus object meaning | A root proves only the object it encodes. | Current v1 roots do not encode v2 selected blockspace or block checks. |

## Theorems Needed

| ID | Theorem | Status Today | Notes |
| --- | --- | --- | --- |
| ENC-001 | `hash_bytes` domain and part encoding is injective before the final hash. | Formalizable. | Requires modeling the length-prefix format exactly. |
| ENC-002 | Tensor id preimage is canonical for shape, dtype, layout, and field values. | Formalizable. | Must account for shape length and field normalization. |
| ENC-003 | Tensor commitment root binds tensor chunks under Merkle/hash assumptions. | Assumption-bound. | Requires leaf format, chunk size, chunk order, and hash collision resistance. |
| ENC-004 | Receipt id preimage is canonical for receipt metadata and tensor roots. | Formalizable plus hash assumption. | Needed before receipt/root binding can support verifier soundness. |
| ENC-005 | Current v1 state roots are deterministic over current maps/sets. | Formalizable for v1 behavior. | Does not imply v2 block validity. |
| ENC-006 | v2 selected receipt root binds exactly canonical selected receipts. | Implementation-blocked. | Requires selected receipt leaf schema and selector state. |
| ENC-007 | v2 checks root binds every selected receipt check leaf. | Implementation-blocked. | Requires check leaf schema and recomputation path. |
| ENC-008 | Public evidence roots bind signed external observations. | Evidence-bound. | Requires production signature/key model and public evidence schema. |

## Current Root Inventory

| Root Or Digest | Encoded Object Today | Safe Claim | Unsafe Claim |
| --- | --- | --- | --- |
| `tensor_id` | Tensor shape, dtype, layout, values. | Identifies canonical tensor preimage under hash assumption. | Identifies real-valued tensor semantics. |
| `commitment_root` | Merkle root over tensor chunk leaves. | Supports root-matched tensor fetch and chunk proofs. | Proves public DA or durable retention. |
| `receipt_id` | Receipt metadata, input roots, output roots, trace root, work/timing fields. | Binds receipt statement under hash assumption. | Proves miner owns key or verifier ran. |
| `receipt_root` | Global current receipt map. | Commits current receipt state content. | Defines v2 selected settled-receipt blockspace. |
| `settled_receipt_root` | Set of settled receipt ids. | Commits a set of settled ids. | Encodes eligibility, expiry, caps, spent/carry-over, or selected blockspace. |
| `attestation_root` | Current attestation map and statement fields. | Commits signed statement records. | Proves statement truth or verifier execution. |
| `state_root` | Current v1/reference chain state root components. | Deterministic root for current reference state. | Proves v2 block transition validity. |
| `reward_root` | Current reward balances and treasury. | Commits current reward state. | Proves challenge-window reward finality for v2. |
| future `settled_receipt_set_root` | Not present. | No current claim. | Any current blockspace soundness claim. |
| future `checks_root` | Not present as block root. | No current block-level claim. | Proposer verified canonical receipt set. |

## Required v2 Leaf Schemas

Before v2 block validity can be proved, these leaf schemas must exist and be canonical:

```text
selected_receipt_leaf =
  domain
  receipt_id
  receipt_hash
  primitive_type
  tensor_work_units
  byte_size
  miner
  settled_height
  expiry_height
  data_availability_status
  spent_or_carry_over_marker

check_leaf =
  domain
  block_parent
  block_beacon
  receipt_id
  primitive_type
  validation_seed_or_seed_anchor
  verifier_transcript_root
  data_availability_root
  result
```

These schemas must include enough metadata to make omission, substitution, replay, and wrong-seed
counterexamples fail.

## Bad Assumptions Rejected

| Bad Assumption | Why It Is Wrong |
| --- | --- |
| A hash root automatically proves the intended consensus object. | A root only commits the bytes actually encoded. |
| Current `receipt_root` is equivalent to v2 selected blockspace. | It commits the global receipt map, not deterministic selected eligible receipts. |
| A set root over receipt ids proves eligibility and caps. | Eligibility, expiry, spent/carry-over, TWU, byte size, and order are not encoded. |
| Per-attestation `checks_root` can stand in for block `checks_root`. | It is a statement field, not an aggregate root over canonical selected check leaves. |
| Merkle root matching proves public availability. | Root matching proves payload integrity, not retention or reachability. |
| Encoding tests prove collision resistance. | Tests can check deterministic encoding paths, not hash security. |

## Discharge Gate

Do not move `K-COM-001`, `ENC-*`, `V2-BLK-002`, or `V2-CHK-002` to a stronger status until:

1. Every encoded object used in the theorem has a named schema.
2. Lengths, type tags, optional fields, ordering, and numeric endianness are specified.
3. Pre-hash injectivity is proved or narrowed to the exact encoded subset.
4. Hash collision resistance is imported as an explicit assumption.
5. Merkle leaf and parent hashing rules are modeled exactly.
6. v2 selected receipt and check leaf schemas exist in implementation.
7. Tests cover type confusion, order changes, missing fields, duplicate leaves, wrong domains, and root
   substitution.
8. The theorem statement says which object the root commits and does not infer stronger consensus meaning.

## Current Judgment

The current encoding/root story is useful but incomplete for full MVP soundness. Tensor and receipt roots
can support verifier-local proofs under canonical encoding plus hash assumptions. They do not support the
reviewed v2 consensus theorem until the chain exposes roots for canonical selected blockspace and
recomputable block-level verification checks.
