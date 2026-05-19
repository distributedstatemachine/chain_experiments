# TensorVM Implementation Status

This tracks the implementation of [`mvp_spec.md`](mvp_spec.md). The
acceptance-criterion test map is in [`coverage_matrix.md`](coverage_matrix.md).

## Implemented In `crates/tensor_vm`

- Deterministic finite-field tensors and TensorVM operations
- TensorVM field arithmetic, SHA-256 hashing, oracle RNG primitives, and standalone consensus logic;
  `tensor_vm` does not depend on `experiments`
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
- Profile-neutral `ChainEngine`, file-backed `ChainStore`, and shared `ChainProfile`/`NodeConfig`
  boundaries so local CPU, public testnet, and future mainnet profiles build the same transition engine
- Receipt-bound validation seeds derived from finalized randomness
- Model-state transition sequencing and conflicting-root settlement delay for training steps
- Txpool with reference transaction payload parsing, receipt deduplication, and multi-validator attestation flow
- Negative-path coverage for transaction parsing, chain registration/receipt/attestation/block-vote rejection,
  verifier metadata/commitment mismatch rejection, RPC route validation, HTTP parsing/socket error responses,
  faucet exhaustion, malformed P2P payloads, and malformed peer-book records
- Full line coverage for TensorVM Merkle helpers, tensor server access, type/signature helpers, validator
  root-availability handling, tensor primitives, TensorVM wrappers, CLI parsing, runtime backends,
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
- Library-owned local CPU synthetic round producer that schedules jobs, executes CPU miner work, verifies,
  settles, finalizes, and advances blocks; `tvmd service serve` now calls this shared protocol path
- P2P message enum, deterministic byte codec, rust-libp2p runtime dependency, TCP/TLS/Yamux swarm
  construction, Gossipsub topic subscriptions for block/job/receipt/attestation/peer announcements,
  Identify protocol wiring, Kademlia discovery/address registration, JSON request-response protocols for
  tensor chunks, tensor rows, and program fetches, `tvmd service peer add` bootstrap seeding,
  `tvmd service readiness` short startup checks for the mandatory libp2p control-plane runtime,
  `tvmd service serve` long-running startup of the same runtime, and durable libp2p bootstrap peer-book
  storage with checksum validation and `/p2p/<peer-id>` dial multiaddr loading
- Documented network-stack recommendation that makes libp2p the mandatory MVP runtime for consensus
  propagation and bounded tensor/program fetches
- Node/tensor RPC route handling, state-root-bearing `/chain/head` responses, service and per-surface
  health endpoints, explorer data RPC endpoints, `/explorer/ws` WebSocket polling for browser explorers,
  telemetry/faucet RPC endpoints, browser-facing explorer/telemetry/faucet HTML pages, mutable
  transaction submission, job lookup, HTTP response formatting, generic HTTP request reading, socketed
  stdlib HTTP serving, `tvmd service init/peer add/readiness/serve` launch
  configuration for a `NodeStore`-backed service process with mandatory rust-libp2p listen configuration,
  and gateway auth/body-size/rate-limit enforcement
- CLI parser and `tvmd` binary entrypoint for documented miner/validator commands, with local stake,
  wallet, device, mandatory libp2p node-endpoint validation, and structured readiness reports
- CPU reference backend for portable default builds, plus a CUDA-only `GpuMinerBackend` that reports
  the selected device and rejects execution unless native CUDA kernels are compiled
- Miner CLI readiness now treats `--device cpu` as the portable reference backend and requires
  `--features cuda-kernels` plus an available CUDA device before `--device cuda:N` can report GPU miner
  readiness
- Optional `cuda-kernels` feature that builds `kernels/cuda/field_matmul.cu` with `nvcc`, routes the
  `GpuMinerBackend` matmul path and LinearTrainingStep forward, backward, error, update, transpose, and
  loss substeps through native CUDA kernels, and checks CUDA outputs against canonical CPU outputs
- Restartable `NodeStore` data directory that persists chain snapshots, append-only block logs, and the
  durable peer book with fixed-format encoding, checksum validation, parent-link checks, append-only sync,
  full-chain state snapshots for restart, and snapshot/block-log/state mismatch detection
- Watcher tooling that scans chain evidence for invalid receipts, data withholding, validator misconduct,
  missing quorum, missing redundant agreement, and conflicting learning-state transitions
- Faucet, explorer WebSocket summaries, full local telemetry success metrics, local testnet bootstrap, and
  public-testnet evidence reporting that separates local readiness from external 7-day run proof
- Typed public-testnet run evidence evaluation for disjoint distinct miner/validator operators, one-to-one
  matching between live operator IDs and live node addresses for counted public participants,
  signature-verified node heartbeat summaries that cover the observed block count, signed wall-clock
  run-window evidence, observed block continuity, finality rate, data-availability rate, invalid-work
  rejection evidence, reward-settlement records, production libp2p runtime evidence, internally consistent
  finalized-block and available-receipt counters, and deployed RPC/explorer/faucet/telemetry service
  reachability with exactly one service-health and one service-content record per deployed service kind,
  reachable and signed health-check summaries that cover the observed block count, rejection of
  overreported reachable counts above signed health-check counts, signed content-root observations bound
  to external HTTPS service URLs and paths, requiring distinct service endpoint IDs and distinct
  service-content roots across the four deployed service kinds
- Typed public-testnet evidence-bundle evaluation that additionally requires an external public manifest
  location, exactly one verified manifest publication signature in the current manifest format, signed
  independent auditor records bound to external audit URIs, distinct from the manifest signer, and observed
  at or after the signed run-window end with an exact match to `independent_auditor_count`, a signed
  run-window record, block/finality history, signed
  operator identity attestations observed inside the signed run window and matched exactly to the
  independent operator/address pairs selected by criteria-aware one-to-one public matching, so a
  validator-satisfying match is not rejected merely because greedy role ordering or address choice consumed
  a shared address, live but uncounted nodes cannot satisfy a missing counted operator attestation, and
  missing, duplicate, extra, or overreported operator-attestation records are rejected, signed
  per-operator production libp2p network-observation records, signed
  block/finality/network-runtime/data-availability/invalid-work/reward-settlement summary roots, signed
  external artifact locators for the raw records behind each summary root with exactly one locator for
  each required supporting-record kind, well-formed whitespace-free
  `ipfs://`/`ar://` content identifiers with traversal/query/fragment path rejection, HTTPS evidence URI
  concrete-path enforcement with root-only/query/fragment rejection, exact untrimmed URI/path manifest-field
  validation, duplicate scalar manifest-field rejection, whitespace-padded field-key and scalar-value rejection,
  whitespace-padded repeated-record value rejection, and
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
  requiring exact untrimmed service URL/path manifest fields and exact comma-separated `service=...`
  values, a `cuda_ready_miner_count` that matches the planned public miner count, a
  `libp2p_ready_node_count` that matches the planned miner plus validator count and can be derived from
  process-level `tvmd service readiness` checks that load the initialized node store, load the durable peer
  book, start the real rust-libp2p control plane, report `libp2p_ready=true`, and exit, plus distinct
  endpoint IDs for exactly one ready RPC, explorer, faucet, and telemetry service plan on the planned
  public content paths used by post-run evidence, with missing, duplicate, or extra preflight service plans
  rejected by the public service plan gate
- `tvmd` binary tests for the documented spec-path pending manifest commands, proving
  `tvmd public-testnet preflight --manifest docs/tensorvm/public-testnet.preflight` reads the checked
  manifest and reports `public_testnet_preflight_ready=false`, while
  a process-level generated external-addressed preflight manifest reports
  `public_testnet_preflight_ready=true`, and
  `tvmd public-evidence validate --manifest docs/tensorvm/public-testnet.evidence` reads the checked
  manifest and reports `public_evidence_full_spec=false`
- Public deployment scaffold under `deploy/tensorvm/` with an environment template, systemd unit for the
  explicit `tvmd` binary target, nginx HTTPS reverse-proxy template for RPC/explorer/faucet/telemetry
  hostnames, a template guard test that requires mandatory libp2p startup, durable data-dir use,
  auth-token wiring, hardened systemd settings, TLS proxying, and the required public HTTPS surfaces, an
  operator runbook guard test that requires the preflight status flags, evidence generator commands, daily
  checkpoint requirements, post-run validation flags, publication artifacts, and explicit no-real-run
  blocker, a deployment README guard test that requires the scaffold file list, public service routes,
  minimal operator flow, evidence commands, and non-evidence boundary, a preflight manifest
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
  wall-clock run-window, and external-operator heartbeat fields, plus
  `tvmd public-evidence run-window-from-file ...` generation that derives the signed run-window manifest
  fields from saved contiguous per-block `run_window_observation=...` files with
  duplicate-block, gap, zero-timestamp, decreasing-timestamp, unsupported-line, and
  whitespace-padded-record rejection,
  `tvmd public-evidence node-heartbeat-from-file ...` generation that derives signed `node=...` lines
  from saved contiguous per-block `node_heartbeat_observation=...` files with duplicate-block, gap,
  identity-mismatch, unsupported-line, and whitespace-padded-record rejection,
  `tvmd public-evidence operator-attestation ...` generation for signed operator identity records bound to
  external identity URIs,
  `tvmd public-evidence service-health ...` generation for exact signed RPC/explorer/faucet/telemetry
  `service=...` manifest records bound to external HTTPS health URLs and observation counts, with
  root-only, query-string, fragment, and non-exact health URL rejection, plus
  `tvmd public-evidence service-health-from-file ...` generation that derives the same signed
  `service=...` line from saved contiguous per-block `service_health_observation=...` files with
  duplicate-block, gap, unsupported-line, and whitespace-padded-record rejection,
  `tvmd public-evidence service-content ...` generation for exact signed RPC/explorer/faucet/telemetry
  `service_content=...` manifest records bound to external HTTPS content URLs, required content paths,
  matching service endpoint IDs, matching service-health HTTPS authorities, exact query-free URL paths,
  root-only, query-string, fragment, and non-exact content URL rejection, distinct content roots, and at
  least 64 observed bytes, plus
  `tvmd public-evidence service-content-from-bytes ...` generation that derives those content roots from
  exact captured response-body bytes and `tvmd public-evidence service-content-from-file ...` generation
  that derives them directly from captured response-body files,
  `tvmd public-evidence network-observation ...` generation for signed public libp2p runtime observation
  records with missing TCP listen port, zero TCP port, non-public multiaddr, malformed DNS-label, and
  single-label DNS rejection, plus `tvmd public-evidence network-observation-from-service-log ...`
  generation that derives the peer ID, protocol counts, bootstrap-peer count, and DoS-control settings
  from captured `tvmd service serve` logs while still requiring a public listen multiaddr, plus manifest
  validation that binds one such signed raw record to every counted public operator and to the aggregate
  network-runtime root; the process-level `tvmd` service smoke test now derives a public-address
  observation root from the live libp2p peer/protocol/control stdout and feeds that root through
  `record-summary-from-roots`, `record-artifact-from-roots`, and the matching file-derived commands,
  `tvmd public-evidence record-summary ...` generation for signed
  block/finality/network-runtime/data-availability/invalid-work/reward-settlement summary fields including
  production libp2p network-observation roots,
  `tvmd public-evidence record-artifact ...` generation for signed external raw-record artifact locators,
  `tvmd public-evidence record-artifact-from-roots ...` generation that signs artifact locators from the
  same derived aggregate root and count as summary generation, `tvmd public-evidence
  record-summary-from-roots ...` deterministic root aggregation for post-run supporting records with
  duplicate-root and whitespace-padded root-list rejection, plus `tvmd public-evidence record-summary-from-file ...` and
  `tvmd public-evidence record-artifact-from-file ...` generation from saved raw-record files containing
  `record_root=...` lines, fully verified signed `network_runtime_observation=...` lines, or typed
  `block_history_record=...`, `finality_history_record=...`, `data_availability_measurement=...`,
  `invalid_work_rejection=...`, and `reward_settlement=...` supporting-record lines with kind-specific
  field validation, including hex reward-settlement participant IDs, exact-line hashing, and
  whitespace-padded or empty-field rejection; network-runtime file
  derivation rejects malformed peer IDs, non-public multiaddrs, zero counters, and mismatched observation
  roots or signatures before aggregation; a process-level `tvmd` integration test now assembles a short
  external-addressed evidence manifest entirely from the signed generator subcommands, validates it from
  disk, and proves it is independently checkable without allowing the default full-spec flag to pass
- Local CPU Docker Compose deployment bundle under `deploy/tensorvm/local-cpu/`, with a CPU-only
  Dockerfile, explicit 10-miner/5-validator Compose topology, one durable volume per operator, mandatory
  libp2p readiness checks for all 15 operators, stable operator-ID-derived libp2p identities, CPU miner
  readiness, authenticated host gateway route checks, a seeded local CPU chain exposed through the gateway
  with settled matmul and LinearTrainingStep receipts, plus live synthetic CPU job production on the
  bootstrap gateway so post-startup blocks advance through receipts, attestations, settlement, proposer
  selection, and finality instead of a static snapshot, miner rewards, finality, data availability, a
  standalone explorer service that polls the TensorVM `/explorer/ws` WebSocket endpoint, a restart gate
  for `miner-03` and `validator-02`, a local-only evidence boundary, and
  `local_cpu_compose::local_cpu_compose_bundle_matches_spec_artifact_shape` guarding the artifact shape

## Implemented In `crates/tensor_vm_explorer`

- Standalone `tensorvm-explorer` binary that serves the browser explorer from `TENSORVM_EXPLORER_LISTEN`
  and publishes the TensorVM WebSocket URL configured by `TENSORVM_EXPLORER_WS_URL`
- Default terminal-style explorer UI shell, Ratzilla/Ratatui WASM entry point, and JSON view models for
  overview metrics, latest blocks, account lookup, miners, validators, receipts, and jobs
- Local CPU Compose integration on `127.0.0.1:8080`, configured to poll `miner-00` through
  `ws://127.0.0.1:8545/explorer/ws?token=local-cpu-testnet-token`

## Verified Gates

Current local verification commands:

```bash
cargo test -p tensor_vm local_testnet --release
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml build
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml restart miner-03 validator-02
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml down -v
cargo fmt --check --all
cargo test --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
cargo tarpaulin
cargo test -p tensor_vm --features cuda-kernels --release
cargo clippy -p tensor_vm --features cuda-kernels --all-targets -- -D warnings
```

The May 19, 2026 Compose verification on this host used
`TENSORVM_LOCAL_CPU_EXPLORER_PORT=18080` for `up --wait` and both check-script runs because host port
`8080` was already allocated; the Compose default remains `8080`.

Gate 0 is the first non-skippable CPU local multi-participant testnet required before CUDA, public
preflight, public evidence, or deployment-gated work can count:

- `cargo test -p tensor_vm local_testnet --release`: 4 TensorVM tests passed, covering the local
  10-miner/5-validator bootstrap shape, separate participant identities and libp2p endpoints, live
  mandatory libp2p control-plane startup under default features, real loopback libp2p delivery across every
  TensorVM gossip topic and request-response message family, matmul settlement/rewards, LinearTrainingStep
  state transition, tensor-server availability, no simulation or local-only
  networking-shim credit, and the explicit non-public-run evidence boundary

- Local CPU Compose gate: `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait`
  started all 15 operator containers as healthy; `deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`
  reported `local_cpu_testnet_ready=true`, `ready_miners=10`, `ready_validators=5`,
  `distinct_operator_ids=15`, `distinct_libp2p_peer_ids=15`, `distinct_node_multiaddrs=15`,
  `libp2p_ready_node_count=15`, `cpu_ready_miner_count=10`, `cuda_required_miner_count=0`,
  `settled_receipts=10`, `matmul_settled=true`, `linear_training_settled=true`, `rewarded_miners=9`,
  `finality_rate_bps=10000`, `data_availability_bps=10000`, `public_evidence_full_spec=false`, and
  `independently_checkable=false`, with `standalone_explorer_ready=true` and
  `standalone_explorer_websocket_polling=true`; the gate now also requires
  `live_block_production=true` and `live_synthetic_jobs=true`, proving `/chain/head` and explorer
  counters advance past the seeded two-block baseline; the same check passed again after
  `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml restart miner-03 validator-02`

The workspace currently has 206 passing library tests under Tarpaulin:

- 14 in `experiments`
- 191 in `tensor_vm`
- 1 in `tensor_vm_explorer`

`cargo test --workspace --release` also runs 2 `tvmd` binary unit tests, 1 local CPU Compose integration
test, and 6 `tvmd` CLI integration tests for the documented spec-path pending manifest commands, a
generated launch-ready preflight manifest round trip, a generated short-run evidence manifest round trip
that reports `independently_checkable=true` and `public_evidence_full_spec=false`, a local CPU seed command
that persists a settled two-block local chain, then proves bounded service startup can generate live
synthetic CPU jobs and advance `/chain/head` past that seed, plus a supervised
`tvmd service init` / `tvmd service peer add` / `tvmd service readiness` / bounded `tvmd service serve`
lifecycle smoke test that starts the mandatory libp2p service path and serves authenticated `/health`, `/rpc/health`,
`/explorer/health`, `/faucet/health`, `/telemetry/health`, `/chain/head`, `/epoch/current`,
`/jobs/current`, the empty-chain `/chain/block/0` route response, `/explorer`, `/faucet/page`, and
`/telemetry/dashboard` from the process-level service, plus authenticated mutable `/tx`, `/receipt`, and
`/attestation` submissions with reference payloads, read-back of registered miner/validator state, and
unauthenticated request rejection. The same process-level smoke test now captures the served
`/chain/head`, `/explorer`, `/faucet/page`, and `/telemetry/dashboard` response bodies and verifies that
`tvmd public-evidence service-content-from-bytes` and
`tvmd public-evidence service-content-from-file` emit identical signed service-content evidence for the
captured bodies, while generating signed `tvmd public-evidence service-health` lines from reached
RPC/explorer/faucet/telemetry health responses. It also derives the local libp2p peer ID and protocol
counts from service stdout and verifies that `tvmd public-evidence network-observation` rejects the
loopback listen address instead of counting local service startup as public network evidence.

The current instrumented Tarpaulin line coverage is documented in
[`tarpaulin_report.md`](tarpaulin_report.md):

- 99.15% workspace line coverage
- 9509/9591 workspace lines covered
- 100.00% `tensor_vm` crate line coverage
- 8670/8670 `tensor_vm` lines covered
- 100.00% `tensor_vm_explorer` crate line coverage
- 271/271 `tensor_vm_explorer` lines covered

The CUDA feature gate was also checked locally on an NVIDIA B200 with CUDA 12.8:

- `cargo test -p tensor_vm --features cuda-kernels --release`: 182 TensorVM tests passed, including
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
  wrapper, `tvmd service init/peer add/readiness/serve` launch wiring, in-process auth/body-size/rate-limit enforcement, and a
  restartable reference `NodeStore` data directory with consistency-checked snapshot, append-only
  block-log, full-chain state, and peer-book persistence, plus tested deployable systemd/env/nginx templates, while
  public evidence validation now rejects local, private, special-use DNS, single-label DNS, documentation,
  shared-address, benchmarking, multicast, reserved, malformed service URLs, root-only service URLs, and
  service URLs with query strings or fragments
- deployed browser explorer, faucet, and telemetry web services; current implementation exposes node RPC
  endpoints, a local standalone WebSocket explorer, and local browser-facing HTML pages for telemetry and
  local faucet claims

The current crate is a complete deterministic reference core and local test harness, not a production
network release.
