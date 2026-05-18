# TensorVM Implementation Status

This tracks the implementation of [`mvp_spec.md`](mvp_spec.md). The
acceptance-criterion test map is in [`coverage_matrix.md`](coverage_matrix.md).

## Implemented In `crates/tensor_vm`

- Deterministic finite-field tensors and TensorVM operations
- Self-contained TensorVM field arithmetic, SHA-256 hashing, and oracle RNG primitives; `tensor_vm` does
  not depend on `pearl_chain`
- Bounds-checked tensor row/cell access and invalid-index rejection
- Full direct TensorVM wrapper and program-hash variant coverage
- Tensor descriptors, Merkle commitments, chunk openings, and row access
- Synthetic matmul jobs, TensorOp receipts, and trace commitments
- Full-output Freivalds verification and row-sampled audit checks
- Row-sampling sparse-corruption probability calculator
- Milestone -1 study utilities for threat model, Freivalds false-accept bounds, randomness grindability,
  data withholding, collusion thresholds, TensorWork concentration, verification cost, and zero-work
  liveness fallback
- LinearTrainingStep execution and verification
- Random-linear checks for `dY = Y - T` and `W_next = W - lr * grad_W`
- Sparse-corruption rejection tests for TensorOp outputs, `dY`, and `W_next`
- Receipt digest/signature checks and trace-root recomputation
- Validator attestations with registered-stake quorum enforcement
- Stake-weighted block-finality votes, duplicate-vote rejection, finalized block tracking, and finality-rate
  telemetry
- Duplicate registration, duplicate receipt, and duplicate validator-attestation rejection
- Account, miner, validator, job, receipt, attestation, reward, and model-state registries
- Miner hardware-class profiles with bounded reported GPU utilization for telemetry
- Content roots for jobs, receipts, attestations, rewards, and full chain state
- Receipt settlement, 70/20/5/5 reward allocation, proposer/treasury rewards, reward accounting without
  repeated payout, and no-quorum rejection
- MVP v0 penalty handling for data-unavailable receipts and mismatched attestations
- Settled prior-epoch TensorWork proposer selection, pending-work exclusion, and zero-work fallback
- Receipt-bound validation seeds derived from finalized randomness
- Model-state transition sequencing and conflicting-root settlement delay for training steps
- Txpool with reference transaction payload parsing, receipt deduplication, and multi-validator attestation flow
- Negative-path coverage for transaction parsing, chain registration/receipt/attestation/block-vote rejection,
  verifier metadata/commitment mismatch rejection, RPC route validation, HTTP parsing/socket error responses,
  faucet exhaustion, malformed P2P payloads, and malformed peer-book records
- Full line coverage for TensorVM Merkle helpers, tensor server access, type/signature helpers, validator
  root-availability handling, tensor primitives, TensorVM wrappers, CLI parsing, runtime backends, explorer,
  faucet, miner, scheduler, storage, watcher, and local testnet/public-evidence modules
- Deterministic job scheduler, operator-separated miner replication assignment with fallback when
  diversity is insufficient, and validator assignment
- Redundant miner-output agreement quorum before settlement, with disagreement/fewer-than-quorum receipts
  delayed rather than rewarded
- Miner node executor with receipt submission and tensor serving
- Validator node attestation flow for TensorOp and LinearTrainingStep receipts
- Server-backed TensorOp data availability verification with unavailable attestations
- Tensor server for descriptors, rows, chunks, Merkle openings, and retention-window pruning
- End-to-end local matmul round: schedule, mine, serve tensors, verify via tensor server, attest, settle, and produce block
- End-to-end local LinearTrainingStep round: register model, mine, verify, attest, settle, update model state, and produce block
- P2P message enum, byte codec, generic framed `Read`/`Write` codec, framed TCP send/receive transport,
  libp2p-oriented topic/protocol mapping,
  bounded local network simulation, peer discovery, Kademlia-style closest-peer directory/bootstrap,
  durable peer-book storage, peer scoring, rate-limit backoff, peer-count admission, and
  backpressure/drop accounting
- Documented network-stack recommendation that chooses libp2p as the primary MVP control plane and keeps
  Iroh as a later tensor/blob data-plane candidate
- Node/tensor RPC route handling, explorer/telemetry/faucet RPC endpoints, browser-facing
  explorer/telemetry/faucet HTML pages, mutable transaction submission, job lookup, HTTP response
  formatting, generic HTTP request reading, socketed stdlib HTTP serving, and gateway
  auth/body-size/rate-limit enforcement
- CLI parser and `tvmd` binary entrypoint for documented miner/validator commands, with local stake,
  wallet, device, and node-endpoint validation plus structured readiness reports
- CPU reference backend and deterministic GPU-miner backend shim for portable default builds
- Optional `cuda-kernels` feature that builds `kernels/cuda/field_matmul.cu` with `nvcc`, routes the
  `GpuMinerBackend` matmul path and LinearTrainingStep matmul substeps through a native CUDA field-matmul
  kernel, and checks CUDA outputs against canonical CPU outputs
- Restartable `NodeStore` data directory that persists chain snapshots, append-only block logs, and the
  durable peer book with fixed-format encoding, checksum validation, parent-link checks, append-only sync,
  full-chain state snapshots for restart, and snapshot/block-log/state mismatch detection
- Watcher tooling that scans chain evidence for invalid receipts, data withholding, validator misconduct,
  missing quorum, missing redundant agreement, and conflicting learning-state transitions
- Faucet, explorer summaries, full local telemetry success metrics, local testnet bootstrap, and
  public-testnet evidence reporting that separates local readiness from external 7-day run proof
- Typed public-testnet run evidence evaluation for distinct miner/validator operators, signed node
  heartbeats, observed block continuity, finality rate, data-availability rate, invalid-work rejection
  evidence, reward-settlement records, production libp2p runtime evidence, and deployed
  RPC/explorer/faucet/telemetry service reachability
- Typed public-testnet evidence-bundle evaluation that additionally requires a public manifest location,
  signatures, independent auditor records, block/finality history, operator attestations, and
  data-availability measurement records before full-spec evidence can be considered independently checkable
- Dependency-free public-testnet preflight manifest parsing plus a CLI launch-readiness surface for
  `tvmd public-testnet preflight --manifest <path>`
- Dependency-free public evidence manifest parsing plus a CLI validation surface for
  `tvmd public-evidence validate --manifest <path>`

## Verified Gates

Current local verification commands:

```bash
cargo fmt --check --all
cargo test --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
cargo tarpaulin
cargo test -p tensor_vm --features cuda-kernels --release
cargo clippy -p tensor_vm --features cuda-kernels --all-targets -- -D warnings
```

The workspace currently has 186 passing library tests under Tarpaulin:

- 14 in `pearl_chain`
- 172 in `tensor_vm`

The current instrumented Tarpaulin line coverage is documented in
[`tarpaulin_report.md`](tarpaulin_report.md):

- 98.58% workspace line coverage
- 5701/5783 workspace lines covered
- 100.00% `tensor_vm` crate line coverage

The CUDA feature gate was also checked locally on an NVIDIA B200 with CUDA 12.8:

- `cargo test -p tensor_vm --features cuda-kernels --release`: 173 TensorVM tests passed, including
  `runtime::tests::cuda_kernel_matches_canonical_field_matmul_edges`
- `cargo clippy -p tensor_vm --features cuda-kernels --all-targets -- -D warnings`: passed

## Still Not A Production/Public Testnet

These spec items require real deployment or non-reference infrastructure and are not complete:

- production GPU-miner packaging and a broader optimized CUDA/C++ kernel suite; the current native kernel
  coverage is an optional CUDA field-matmul path checked against canonical CPU outputs
- long-running public 7-day testnet with independent external operators; current implementation exposes
  typed `PublicTestnetRunEvidence`/`PublicTestnetEvidence` so this criterion can be measured without
  treating local simulation as public proof, and now requires invalid-work rejection plus reward-settlement
  records, production libp2p runtime evidence, and deployed public-service reachability before public
  evidence can satisfy the gate
- published external public-testnet evidence bundle; the required bundle shape is documented in
  [`public_testnet_evidence.md`](public_testnet_evidence.md), but no complete external bundle is available
  yet
- actual libp2p transport runtime with production DoS controls; current implementation is a local
  libp2p-shaped simulation plus stdlib framed TCP message transport, durable peer-book persistence,
  Kademlia-style closest-peer directory/bootstrap, peer-count admission, score-based drops, and
  deterministic rate-limit/backoff policy checks
- production HTTP deployment and full durable database; current implementation has a stdlib socketed HTTP
  wrapper, in-process auth/body-size/rate-limit enforcement, and a restartable reference `NodeStore` data
  directory with consistency-checked snapshot, append-only block-log, full-chain state, and peer-book
  persistence
- deployed browser explorer, faucet, and telemetry web services; current implementation exposes node RPC
  endpoints and local browser-facing HTML pages for explorer summaries, telemetry snapshots, and local
  faucet claims

The current crate is a complete deterministic reference core and local simulation harness, not a production
network release.
