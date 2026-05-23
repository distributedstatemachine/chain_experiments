# TensorVM MVP Core Signature Authentication Boundary

Status: documentation-only signature proof boundary compiled from the current worktree.

Purpose: separate the current reference signature relation from production authentication. Several current
proof claims depend on "signature validity," but the Rust helper is a deterministic hash relation, not a
private-key signature scheme. This document states exactly what can be proved today and what must remain an
assumption or implementation gate.

This document does not change code and does not mark the MVP core sound.

## Current Evidence

Current Rust signature surface:

```text
type Address = [u8; 32]
type Signature = [u8; 32]
sign(address, message) = H("tensor-vm-signature-v1", address, message)
verify_signature(address, message, signature) = sign(address, message) == signature
```

Current use sites include:

- Receipt verifier paths checking miner receipt signatures.
- Attestation admission checking validator attestation signatures.
- Block vote admission checking validator vote signatures.

This is useful for testing message binding and admission plumbing. It is not proof of private-key control.

## The Current Theorem

The theorem currently supported by the reference core is:

```text
If verify_signature(address, message, signature) is true,
then signature equals H("tensor-vm-signature-v1", address, message).
```

This theorem is deterministic and local. It can support these claims:

1. The checked message bytes are bound to the claimed address under the reference relation.
2. Receipt, attestation, and vote admission paths call the relation on their expected message digest.
3. Mutating the address, message, or signature changes whether the relation holds.

It cannot support production actor-control claims.

## Claims Not Proven Today

| Claim | Why Not Proven |
| --- | --- |
| Address owner signed the message. | There is no private key or public-key verification relation. |
| Signatures are unforgeable. | Anyone who knows `address` and `message` can compute the helper signature. |
| Address ownership was registered securely. | `Address` is a byte string in the reference model. |
| Signatures are replay-safe across all contexts. | Domain separation exists for helper hashes, but no production replay model or key policy exists. |
| Aggregated validator signatures are production BFT certificates. | The current finality path counts individual reference vote signatures over known current block hashes. |
| Evidence-manifest signatures prove independent public actors. | Public evidence needs a separate production signature/key identity model. |

## Required Production Model

A production authentication theorem should introduce:

```text
PublicKey
PrivateKey
Address = address_of(PublicKey)
Sign(PrivateKey, Domain, Message) -> Signature
Verify(PublicKey, Domain, Message, Signature) -> Bool
```

Required assumptions:

1. Existential unforgeability under chosen-message attack for the signature scheme.
2. Binding between address and public key.
3. Key ownership and custody for miners, validators, operators, auditors, and services.
4. Replay domain separation for receipts, attestations, block votes, block proposals, evidence records, and
   public evidence manifests.
5. Key registration, rotation, revocation, and slashing/evidence semantics where relevant.
6. Canonical message encoding before signing.

These assumptions should be visible in theorem statements. They are not discharged by swapping helper
function names.

## Message Domains To Model

| Domain | Actor | Message Must Bind |
| --- | --- | --- |
| Receipt | Miner | receipt id, job id, primitive, roots, trace root, submitted height, work units. |
| Attestation | Validator | receipt id, job id, primitive, result, checks root, DA bit, stake snapshot or validator identity domain. |
| Block vote | Validator | block hash, height, parent or epoch context, stake snapshot, vote domain/version. |
| Block proposal | Proposer | v2 block header, proposer, selected receipt root, checks root, beacon, target, nonce. |
| Challenge | Challenger or validator | challenged receipt/block, opening, evidence root, timeout context. |
| Public evidence | Operator/auditor/service | bundle id, URI, record root, count, observation time/window, service/operator identity. |

Every signature theorem should name the domain. A generic "signed hash" theorem is too weak for production
authentication.

## Interaction With Existing Proof Nodes

| Proof Node | Current Signature Meaning | Production Upgrade Gate |
| --- | --- | --- |
| `K-TOP-001` TensorOp completeness | Receipt signature satisfies the reference relation. | Miner public key controls receipt address and signs canonical receipt domain. |
| `K-LIN-001` LinearTrainingStep completeness | Same reference receipt relation. | Same production receipt theorem. |
| `K-SIG-001` accepted statement signature | Statement satisfies `verify_signature` helper. | Replace with production `Verify` and unforgeability assumption. |
| `K-ATT-001` attestation admission | Assigned validator submitted a statement satisfying the reference relation. | Validator key ownership, replay-safe attestation domain, and receipt-lifecycle assignment. |
| `K-QUO-001` quorum | Unique assigned validators have accepted reference statements. | Production signatures plus still-syntactic quorum unless verifier evidence is bound. |
| `V2-FIN-001` vote admission | Current votes satisfy reference relation over known current block hash. | Vote must be over a block that passed `validate_block_v2`, with production validator signatures. |
| Public evidence gates | Evidence records may have formatted signatures. | External key identity and independently verifiable production signatures. |

## Bad Assumptions Rejected

| Bad Assumption | Why It Is Wrong |
| --- | --- |
| The reference `sign` helper proves actor control. | It is public deterministic hashing over address and message. |
| Signature tests prove production authentication. | They exercise message-flow plumbing only. |
| Finality signatures imply v2 validity. | Even production signatures over the wrong object would not validate canonical blockspace or useful-PoW. |
| Public evidence signatures prove independent operators by themselves. | Operator identity, key ownership, hosting independence, and observation windows are separate evidence. |
| Domain-separated hashes are the same as replay-safe signatures. | Replay safety needs explicit signed domains and state rules. |

## Discharge Gate

Do not mark `K-SIG-001`, `AD-009`, or production-authentication claims discharged until:

1. The production signature scheme and address derivation are specified.
2. Canonical signed-message encodings exist for receipts, attestations, block votes, block proposals, and
   public evidence records.
3. Replay domains and version tags are explicit.
4. Key registration and rotation rules exist for miners, validators, operators, auditors, and services.
5. Tests cover wrong key, wrong domain, replay, mutated message, unknown key, and revoked/rotated key cases.
6. The formal proof imports signature unforgeability and key-ownership assumptions by name.
7. The bad-assumption ledger no longer needs to describe reference signatures as the active production
   boundary.

## Current Judgment

The current signature relation is acceptable for local reference plumbing and syntactic theorem statements.
It is not production authentication. The full MVP core cannot be called sound while receipt authority,
validator votes, operator evidence, or finality certificates depend on this helper without an explicit
production signature model.
