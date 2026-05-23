# TensorVM MVP Core Data Availability Boundary

Status: documentation-only proof boundary compiled from the current worktree.

Purpose: separate verification-time artifact retrieval from public data availability. The current worktree
contains in-progress remote validator tensor-fetch evidence. That improves the local verifier path, but it
does not make durable public DA, independent retention, or v2 consensus sound.

This document is intentionally conservative. It records what the current artifact-availability path can
support as a proof claim and what must remain an assumption or blocked theorem.

The canonical encoding and commitment model for root binding is specified in
[`mvp_core_canonical_encoding_commitment_model.md`](mvp_core_canonical_encoding_commitment_model.md).

## Executive Boundary

Allowed claim:

```text
A validator role can attempt bounded peer request-response fetches for missing tensors by commitment root,
decode a returned tensor payload, check that the decoded tensor commitment root equals the requested root,
insert the tensor locally, and then run the reference verifier over a locally available artifact bundle.
```

Disallowed claim:

```text
Remote tensor fetch proves public data availability.
```

The allowed claim is verification-time availability in a local/runtime sense. It is not durable retention,
independent hosting, censorship resistance, or public measurement over active and retention windows.

## Current Worktree Evidence

The following evidence is present in the current worktree:

- `P2pMessage::RequestTensorByCommitmentRoot` and `TensorByCommitmentRootResponse`.
- `TensorVmLibp2pService::register_tensor` keeps a service-local tensor store.
- `TensorVmLibp2pService::request_response` sends bounded request-response messages to connected peers.
- `response_for_request` answers by commitment root with an encoded tensor payload or `None`.
- `fetch_validator_role_missing_tensors` requests missing receipt tensor roots from connected peers.
- The validator fetch path decodes payloads and rejects them unless `tensor.commitment_root()` equals the
  requested root before insertion.
- Validator remote-fetch counters are exposed in runtime state/status fields:
  `validator_remote_tensor_fetch_attempts`, `validator_remote_tensor_fetch_successes`,
  `validator_remote_tensor_fetch_failures`, `validator_remote_tensor_fetch_bytes`, and
  `validator_remote_tensors_inserted`.
- P2P tests exercise successful tensor-by-root response and not-found response.

This is evidence for a local retrieval path. It is not evidence that all receipt tensors are publicly
available from independent operators for the required retention window.

## Proof Claims

### DA-LOCAL-001: Root-Matched Fetch Safety

Statement:

```text
If fetch_validator_role_missing_tensors inserts a tensor for requested commitment root r, then the decoded
tensor payload had commitment_root(tensor) = r at insertion time.
```

Status: local-proof-ready for the current worktree path.

Proof sketch:

The fetch path requests a specific `commitment_root`. It accepts only a
`TensorByCommitmentRootResponse { commitment_root, payload: Some(payload) }` whose response root equals the
requested root. It then decodes the tensor payload and checks `tensor.commitment_root() == root` before
calling `node.insert_tensor`.

What this proves:

The validator does not blindly insert arbitrary peer bytes as the missing artifact for a receipt root.

What this does not prove:

The peer is honest, the tensor is retained long term, the tensor is served to everyone, or enough
independent peers can serve it under adversarial conditions.

### DA-LOCAL-002: Remote Fetch Enables Local Verifier Execution

Statement:

```text
If all required receipt roots are either already local or are fetched and inserted through DA-LOCAL-001, then
role_receipt_bundle_from_local_tensors can construct the artifact bundle needed by the validator role
verifier for the supported primitive.
```

Status: local-proof-ready for current TensorOp and LinearTrainingStep bundle construction.

Proof sketch:

For TensorOp receipts, the role bundle requires the two input roots and one output root. For
LinearTrainingStep receipts, the role bundle requires `y_root`, `grad_w_root`, and `weight_root_after`, while
the synthetic-local job supplies the batch tensors and expected starting weights. Once every required root
has a locally stored tensor, the role can build a `RoleReceiptBundle` and call the reference validator.

Boundary:

This is still a local artifact-bundle theorem. It does not prove that the original miner served the tensor,
that an assigned miner retained it, or that public observers can retrieve it later.

### DA-LOCAL-003: Fetch Counters Are Observability, Not Security

Statement:

```text
Remote-fetch counters record role-loop attempts, successes, failures, bytes, and inserted tensors for the
current runtime surface.
```

Status: local evidence.

Proof sketch:

The role runtime records fetch attempts and outcomes into status fields after a fetch pass. These counters
can help local gates distinguish remotely fetched tensors from tensors already present by deterministic
replay.

Boundary:

Counters are telemetry. They are not a cryptographic proof of availability, not signed external evidence,
and not sufficient for public DA.

## Claims Still Not Proven

| Claim | Status | Reason |
| --- | --- | --- |
| Durable public DA | Not proven | No retention-window measurement, public observer signatures, or independent hosting evidence. |
| Miner-specific availability | Not proven | A successful peer response by root does not prove the receipt miner served the tensor. |
| Censorship-resistant retrieval | Not proven | The local fetch path tries connected peers; it does not prove enough peers will answer under adversarial conditions. |
| Complete receipt artifact retention | Not proven | The local verifier only knows about roots it needs for the current receipt bundle and active validation. |
| Challenge-window availability | Not proven | The current counters do not prove tensors remain retrievable through the challenge/retention window. |
| Public operator independence | Not proven | Local peer IDs and containers are not independent principals. |
| v2 block validity | Not proven | Data fetch does not add canonical blockspace, block-level `checks_root`, PoW target, or finality validation. |

## Bad Assumptions To Reject

1. **"Remote fetch means public DA."**
   A successful request-response proves one runtime could retrieve a matching tensor at that moment.

2. **"Commitment-root match proves retention."**
   The root check proves payload binding, not future availability.

3. **"Fetch counters are proof evidence by themselves."**
   Counters are useful local observability. Public proof needs signed measurements or reproducible external
   evidence.

4. **"Any peer response proves the miner served the tensor."**
   Root-addressed fetch can be answered by any peer with the tensor unless the protocol binds service to the
   receipt miner or assigned storage role.

5. **"Verification-time availability is enough for v2 consensus."**
   v2 still needs canonical receipt selection, recomputable block-level checks, useful-PoW, challenge rules,
   and reward finality.

## Proof/Implementation Gates For Stronger DA Claims

Before saying TensorVM has public DA, require all of:

1. Receipt metadata binds expected artifact roots, byte sizes, primitive type, miner, and retention deadline.
2. A public observer or validator measurement format signs successful and failed retrieval attempts.
3. Measurements cover active validation and challenge/retention windows, not only immediate local runtime.
4. Evidence identifies which operator served the tensor and whether that operator is independent of the
   miner/proposer when independence is claimed.
5. The checker verifies retrieval from multiple counted operators or public observers, not only the gateway.
6. Challenge rules specify what happens when required artifacts become unavailable after settlement.
7. Public evidence manifests carry DA records and signatures with replayable locators.

## How This Fits The Sound Kernel

The sound kernel may include root-matched verification-time retrieval as an availability precondition for
running the verifier. It must not include public DA or durable retention as a proven property.

Correct phrasing:

```text
The current worktree has a proof-ready local root-matched tensor retrieval invariant for validator
verification-time artifact fetches.
```

Incorrect phrasing:

```text
TensorVM proves data availability.
```

## Current Judgment

Remote validator tensor fetch is a useful upgrade to the local verification path. It narrows the gap between
role-owned validation and deterministic local replay. It does not change the core consensus finding:
TensorVM is still not sound for the reviewed v2 MVP until useful-verification PoW, canonical settled-receipt
blockspace, block-level `checks_root`, and v2 finality validation exist.
