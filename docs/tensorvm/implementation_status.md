# TensorVM Implementation Status

This tracks the implementation of [`mvp_spec.md`](mvp_spec.md). The
acceptance-criterion test map is in [`coverage_matrix.md`](coverage_matrix.md).

## Implemented In `crates/tensor_vm`

- Deterministic finite-field tensors and TensorVM operations
- TensorVM field arithmetic, SHA-256 hashing, oracle RNG primitives, and standalone consensus logic;
  `tensor_vm` does not depend on `pearl_chain`
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
- P2P message enum, deterministic byte codec, rust-libp2p runtime dependency, TCP/TLS/Yamux swarm
  construction, Gossipsub topic subscriptions for block/job/receipt/attestation/peer announcements,
  Identify protocol wiring, Kademlia discovery/address registration, JSON request-response protocols for
  tensor chunks, tensor rows, and program fetches, `tvmd service peer add` bootstrap seeding,
  `tvmd service serve` startup of the mandatory libp2p control-plane runtime, and durable libp2p bootstrap
  peer-book storage with checksum validation and `/p2p/<peer-id>` dial multiaddr loading
- Documented network-stack recommendation that makes libp2p the mandatory MVP runtime for consensus
  propagation and bounded tensor/program fetches
- Node/tensor RPC route handling, service and per-surface health endpoints, explorer/telemetry/faucet RPC endpoints, browser-facing
  explorer/telemetry/faucet HTML pages, mutable transaction submission, job lookup, HTTP response
  formatting, generic HTTP request reading, socketed stdlib HTTP serving, `tvmd service init/peer add/serve`
  launch configuration for a `NodeStore`-backed service process with mandatory rust-libp2p listen
  configuration, and gateway auth/body-size/rate-limit enforcement
- CLI parser and `tvmd` binary entrypoint for documented miner/validator commands, with local stake,
  wallet, device, mandatory libp2p node-endpoint validation, and structured readiness reports
- CPU reference backend for portable default builds, plus a CUDA-only `GpuMinerBackend` that reports
  the selected device and rejects execution unless native CUDA kernels are compiled
- Optional `cuda-kernels` feature that builds `kernels/cuda/field_matmul.cu` with `nvcc`, routes the
  `GpuMinerBackend` matmul path and LinearTrainingStep forward, backward, error, update, transpose, and
  loss substeps through native CUDA kernels, and checks CUDA outputs against canonical CPU outputs
- Restartable `NodeStore` data directory that persists chain snapshots, append-only block logs, and the
  durable peer book with fixed-format encoding, checksum validation, parent-link checks, append-only sync,
  full-chain state snapshots for restart, and snapshot/block-log/state mismatch detection
- Watcher tooling that scans chain evidence for invalid receipts, data withholding, validator misconduct,
  missing quorum, missing redundant agreement, and conflicting learning-state transitions
- Faucet, explorer summaries, full local telemetry success metrics, local testnet bootstrap, and
  public-testnet evidence reporting that separates local readiness from external 7-day run proof
- Typed public-testnet run evidence evaluation for disjoint distinct miner/validator operators,
  signature-verified node heartbeat summaries that cover the observed block count, signed wall-clock
  run-window evidence, observed block continuity, finality rate, data-availability rate, invalid-work
  rejection evidence, reward-settlement records, production libp2p runtime evidence, internally consistent
  finalized-block and available-receipt counters, and deployed RPC/explorer/faucet/telemetry service
  reachability with reachable and signed health-check summaries that cover the observed block count plus
  signed content-root observations bound to external HTTPS service URLs and paths, requiring distinct
  service endpoint IDs and distinct service-content roots across the four deployed service kinds
- Typed public-testnet evidence-bundle evaluation that additionally requires an external public manifest
  location, exactly one verified manifest publication signature in the current manifest format, signed
  independent auditor records bound to external audit URIs, distinct from the manifest signer, and observed
  at or after the signed run-window end, a signed run-window record, block/finality history, signed
  operator identity attestations observed inside the signed run window
  matched to signed node-heartbeat records with no overreported operator-attestation counts, signed
  per-operator production libp2p network-observation records, signed
  block/finality/network-runtime/data-availability/invalid-work/reward-settlement summary roots, signed
  external artifact locators for the raw records behind each summary root, well-formed whitespace-free
  `ipfs://`/`ar://` content identifiers, HTTPS evidence URI path enforcement with query/fragment
  rejection, exact untrimmed URI/path manifest-field validation, duplicate scalar manifest-field
  rejection, whitespace-padded field-key rejection, and
  exact run-derived block/finality/network-runtime/data-availability/invalid-work summary counts, distinct node-address
  counting for public operators, plus network-runtime observation rejection for missing records,
  unmatched operators, non-public listen addresses, stale timestamps, undercounts, and overcounts against
  every counted public operator before full-spec evidence can be considered
  independently checkable; the `public_evidence_full_spec`
  report flag also requires the default 7-day, 10-miner, 5-validator public-testnet criteria or stricter
  criteria, so relaxed local harness criteria cannot mark an evidence bundle full-spec
- Dependency-free public-testnet preflight manifest parsing plus a CLI launch-readiness surface for
  `tvmd public-testnet preflight --manifest <path>`, with public service endpoint checks rejecting local,
  private, link-local, special-use DNS, single-label DNS, documentation, shared-address, benchmarking,
  multicast, reserved, and malformed HTTPS authorities, rejecting service URL query strings/fragments, and
  requiring exact untrimmed service URL/path manifest fields plus distinct endpoint IDs for the planned
  public content paths used by post-run evidence
- `tvmd` binary tests for the documented spec-path pending manifest commands, proving
  `tvmd public-testnet preflight --manifest docs/tensorvm/public-testnet.preflight` reads the checked
  manifest and reports `public_testnet_preflight_ready=false`, while
  `tvmd public-evidence validate --manifest docs/tensorvm/public-testnet.evidence` reads the checked
  manifest and reports `public_evidence_full_spec=false`
- Public deployment scaffold under `deploy/tensorvm/` with an environment template, systemd unit for the
  explicit `tvmd` binary target, nginx HTTPS reverse-proxy template for RPC/explorer/faucet/telemetry
  hostnames, an operator runbook for external launch/evidence collection/publication, a preflight manifest
  example that parses but does not report launch readiness until special-use placeholder hosts are
  replaced, checked spec-path pending manifests at `docs/tensorvm/public-testnet.preflight` and
  `docs/tensorvm/public-testnet.evidence` that parse from the documented CLI paths while intentionally
  reporting not-ready/non-full-spec until replaced by owned public infrastructure and real run records, and
  a checked post-run evidence manifest example that validates structurally while still reporting
  `public_evidence_full_spec=false`
- Dependency-free public evidence manifest parsing plus a CLI validation surface for
  `tvmd public-evidence validate --manifest <path>`, plus
  `tvmd public-evidence publication ...`, `tvmd public-evidence auditor-record ...`,
  `tvmd public-evidence run-window ...`, and
  `tvmd public-evidence node-heartbeat ...` generation for signed publication, independent-auditor,
  wall-clock run-window, and external-operator heartbeat fields,
  `tvmd public-evidence operator-attestation ...` generation for signed operator identity records bound to
  external identity URIs,
  `tvmd public-evidence service-health ...` generation for exact signed RPC/explorer/faucet/telemetry
  `service=...` manifest records bound to external HTTPS health URLs and observation counts,
  `tvmd public-evidence service-content ...` generation for exact signed RPC/explorer/faucet/telemetry
  `service_content=...` manifest records bound to external HTTPS content URLs, required content paths,
  matching service endpoint IDs, matching service-health HTTPS authorities, exact query-free URL paths,
  distinct content roots, and at least 64 observed bytes, plus
  `tvmd public-evidence service-content-from-bytes ...` generation that derives those content roots from
  exact captured response-body bytes and `tvmd public-evidence service-content-from-file ...` generation
  that derives them directly from captured response-body files,
  `tvmd public-evidence network-observation ...` generation for signed public libp2p runtime observation
  records with missing TCP listen port, zero TCP port, non-public multiaddr, malformed DNS-label, and
  single-label DNS rejection, plus manifest validation that binds one such signed raw record to every
  counted public operator and to the aggregate network-runtime root,
  `tvmd public-evidence record-summary ...` generation for signed
  block/finality/network-runtime/data-availability/invalid-work/reward-settlement summary fields including
  production libp2p network-observation roots,
  `tvmd public-evidence record-artifact ...` generation for signed external raw-record artifact locators,
  `tvmd public-evidence record-artifact-from-roots ...` generation that signs artifact locators from the
  same derived aggregate root and count as summary generation, and `tvmd public-evidence
  record-summary-from-roots ...` deterministic root aggregation for post-run supporting records with
  duplicate-root rejection

## Verified Gates

Current local verification commands:

```bash
cargo test -p tensor_vm local_testnet --release
cargo fmt --check --all
cargo test --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
cargo tarpaulin
cargo test -p tensor_vm --features cuda-kernels --release
cargo clippy -p tensor_vm --features cuda-kernels --all-targets -- -D warnings
```

Gate 0 is the first non-skippable CPU local multi-participant testnet required before CUDA, public
preflight, public evidence, or deployment-gated work can count:

- `cargo test -p tensor_vm local_testnet --release`: 3 TensorVM tests passed, covering the local
  10-miner/5-validator bootstrap shape, separate participant identities and libp2p endpoints, live
  mandatory libp2p control-plane startup under default features, matmul settlement/rewards,
  LinearTrainingStep state transition, tensor-server availability, no simulation or local-only
  networking-shim credit, and the explicit non-public-run evidence boundary

The workspace currently has 185 passing library tests under Tarpaulin:

- 14 in `pearl_chain`
- 171 in `tensor_vm`

`cargo test --workspace --release` also runs 2 `tvmd` binary unit tests and 3 `tvmd` CLI integration
tests for the documented spec-path pending manifest commands plus a supervised
`tvmd service init` / `tvmd service peer add` / bounded `tvmd service serve` lifecycle smoke
test that starts the mandatory libp2p service path and serves authenticated `/health`, `/rpc/health`,
`/explorer/health`, `/faucet/health`, `/telemetry/health`, `/chain/head`, `/epoch/current`,
`/jobs/current`, the empty-chain `/chain/block/0` route response, `/explorer`, `/faucet/page`, and
`/telemetry/dashboard` from the process-level service, plus authenticated mutable `/tx`, `/receipt`, and
`/attestation` submissions with reference payloads, read-back of registered miner/validator state, and
unauthenticated request rejection.

The current instrumented Tarpaulin line coverage is documented in
[`tarpaulin_report.md`](tarpaulin_report.md):

- 98.97% workspace line coverage
- 7897/7979 workspace lines covered
- 100.00% `tensor_vm` crate line coverage
- 7329/7329 `tensor_vm` lines covered

The CUDA feature gate was also checked locally on an NVIDIA B200 with CUDA 12.8:

- `cargo test -p tensor_vm --features cuda-kernels --release`: 171 TensorVM tests passed, including
  `runtime::tests::cuda_kernel_matches_canonical_field_matmul_edges` and
  `runtime::tests::cuda_kernels_match_canonical_linear_tensor_ops`
- `cargo clippy -p tensor_vm --features cuda-kernels --all-targets -- -D warnings`: passed

## Still Not A Production/Public Testnet

These spec items require real deployment or non-reference infrastructure and are not complete:

- production GPU-miner packaging and a broader optimized CUDA/C++ kernel suite; the current native kernel
  coverage includes CUDA field-matmul plus linear-step sub/scalar/transpose/squared-error kernels checked
  against canonical CPU outputs
- long-running public 7-day testnet with independent external operators; current implementation exposes
  typed `PublicTestnetRunEvidence`/`PublicTestnetEvidence` so this criterion can be measured without
  treating a local test harness as public proof, and now requires a signed wall-clock run window,
  invalid-work rejection plus reward-settlement records, signed per-operator production libp2p runtime
  observation records that aggregate to the network-runtime root, signed external artifact locators for raw supporting records, deployed public-service
  reachability, and signed public-service content roots before public evidence can satisfy the gate
- published external public-testnet evidence bundle; the required bundle shape is documented in
  [`public_testnet_evidence.md`](public_testnet_evidence.md), and
  `deploy/tensorvm/RUNBOOK.md` records the external collection and publication flow, while
  `docs/tensorvm/public-testnet.evidence` and
  `deploy/tensorvm/manifests/public-testnet.evidence.example` are checked as non-full-spec format
  examples, but no complete external bundle is available yet
- externally observed production libp2p operation during a public testnet; current implementation starts
  the mandatory rust-libp2p service runtime locally with bounded Gossipsub payloads, request timeouts,
  concurrent stream limits, idle connection timeouts, Kademlia discovery/address registration, and durable
  bootstrap peer-book persistence loaded as peer-ID-preserving dial multiaddrs, and the public evidence validator now requires signed
  network-observation records, but no independently checkable public-run network evidence is available yet
- production HTTP deployment and full durable database; current implementation has a stdlib socketed HTTP
  wrapper, `tvmd service init/peer add/serve` launch wiring, in-process auth/body-size/rate-limit enforcement, and a
  restartable reference `NodeStore` data directory with consistency-checked snapshot, append-only
  block-log, full-chain state, and peer-book persistence, plus deployable systemd/nginx templates, while
  public evidence validation now rejects local, private, special-use DNS, single-label DNS, documentation,
  shared-address, benchmarking, multicast, reserved, malformed service URLs, and service URLs with query
  strings or fragments
- deployed browser explorer, faucet, and telemetry web services; current implementation exposes node RPC
  endpoints and local browser-facing HTML pages for explorer summaries, telemetry snapshots, and local
  faucet claims

The current crate is a complete deterministic reference core and local test harness, not a production
network release.
