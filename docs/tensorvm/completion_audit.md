# TensorVM Completion Audit

Audit date: May 18, 2026.

Objective audited: fully implement [`mvp_spec.md`](mvp_spec.md) for TensorVM.

This audit separates local reference completion from full-spec completion. Full-spec completion is not
achieved until deployment evidence exists for production networking, public services, and a 7-day external
public testnet run.

## Success Criteria

The objective decomposes into these deliverables:

1. Preserve the required Cargo workspace and TensorVM crate structure.
2. Keep `crates/tensor_vm` self-contained and independent of `crates/pearl_chain`.
3. Implement and test all local behavior needed for Acceptance Criteria 1-12, 14, and 15.
4. Provide a local preflight harness and evidence validator for Acceptance Criterion 13.
5. Run and document the required workspace verification commands.
6. Maintain 100% line coverage for `crates/tensor_vm/src` or document justified uncovered lines.
7. Provide real CUDA/C++ kernels for any claimed GPU path and verify them against CPU semantics.
8. Use production libp2p for node discovery, gossip, and request/response propagation.
9. Deploy RPC, explorer, faucet, and telemetry services outside the local test harness.
10. Run a 7-day public testnet with independent external operators.
11. Publish independently checkable evidence for the public run and link it from implementation status.

## Prompt-To-Artifact Checklist

| Requirement | Evidence | Status |
| --- | --- | --- |
| Required Cargo workspace structure | Root `Cargo.toml`, `crates/pearl_chain`, `crates/tensor_vm`, `docs/tensorvm`, and linked READMEs | Present |
| TensorVM crate is independent of Pearl Chain | `cargo tree -p tensor_vm` shows no `pearl_chain` dependency; rust-libp2p and serde are direct TensorVM runtime dependencies | Present |
| Recommended TensorVM modules exist | `api`, `chain`, `challenge`, `cli`, `error`, `explorer`, `faucet`, `jobs`, `merkle`, `miner`, `p2p`, `rpc`, `runtime`, `scheduler`, `storage`, `study`, `telemetry`, `tensor`, `tensor_server`, `testnet`, `txpool`, `types`, `validator`, `verify`, `vm`, `watcher` under `crates/tensor_vm/src` | Present |
| Local Acceptance Criteria 1-12, 14, 15 | [`coverage_matrix.md`](coverage_matrix.md) maps each criterion to concrete tests and artifacts | Locally covered |
| AC13 local preflight | [`public_testnet_preflight.md`](public_testnet_preflight.md), `parse_public_testnet_preflight_manifest`, and `tvmd public-testnet preflight --manifest <path>` | Present |
| Public deployment scaffold | `deploy/tensorvm/` contains systemd, nginx, environment, and preflight-manifest templates; `cargo run -p tensor_vm --bin tvmd -- public-testnet preflight --manifest deploy/tensorvm/manifests/public-testnet.preflight.example` reports launch readiness | Present as pre-run scaffold |
| AC13 public evidence validator | [`public_testnet_evidence.md`](public_testnet_evidence.md), `parse_public_testnet_evidence_manifest`, `PublicTestnetEvidenceBundle`, external publication URI validation, verified manifest publication signatures, signed wall-clock run-window evidence, signed block/finality/network-runtime/data-availability summary roots, signed operator-attestation-derived external-operator evidence, `tvmd public-evidence validate --manifest <path>`, `tvmd public-evidence publication ...`, `tvmd public-evidence run-window ...`, `tvmd public-evidence node-heartbeat ...`, `tvmd public-evidence operator-attestation ...`, `tvmd public-evidence service-health ...` signed service-record generation, `tvmd public-evidence network-observation ...` signed libp2p observation-record generation, and `tvmd public-evidence record-summary ...` signed supporting-record generation | Present |
| Miner, validator, and service CLI surfaces | `cli::parse_cli_args`, `cli::execute_reference_cli_command`, libp2p multiaddr validation for miner/validator nodes, required `tvmd service serve --p2p-listen <multiaddr>`, and `tvmd` binary entrypoint | Reference implementation present |
| CPU reference backend | `runtime::CpuReferenceBackend` and runtime tests | Present |
| GPU miner backend | `runtime::GpuMinerBackend` reports the selected CUDA device and rejects execution when native CUDA kernels are not compiled; CPU execution remains a separate `runtime::CpuReferenceBackend` | Present locally |
| Native CUDA/C++ checked against CPU outputs | `cuda-kernels` feature builds `kernels/cuda/field_matmul.cu`; `runtime::tests::cuda_kernel_matches_canonical_field_matmul_edges`, `runtime::tests::cuda_kernels_match_canonical_linear_tensor_ops`, and `runtime::tests::cpu_and_gpu_backends_match_linear_step` | Present for matmul and linear-step tensor ops |
| Restartable node storage | `storage::NodeStore`, snapshots, append-only block log, chain state, peer book tests | Reference implementation present |
| P2P/RPC runtime and socket tests | `p2p` rust-libp2p swarm, background libp2p service runtime, Kademlia/bootstrap, protocol tests, P2P codec tests, `rpc` socketed HTTP and health-route tests, `tvmd service init/serve` launch validation, and public evidence service URLs rejected when local or private | Reference implementation present |
| Required commands documented | [`implementation_status.md`](implementation_status.md) and [`tarpaulin_report.md`](tarpaulin_report.md) | Present |
| `cargo fmt --check --all` | Latest iteration evidence records pass from workspace root | Passed |
| `cargo test --workspace --release` | Latest iteration evidence records 14 `pearl_chain` and 165 `tensor_vm` tests | Passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Latest iteration evidence records pass from workspace root | Passed |
| `cargo tarpaulin` | [`tarpaulin_report.md`](tarpaulin_report.md) records 179 instrumented tests | Passed |
| TensorVM line coverage | [`tarpaulin_report.md`](tarpaulin_report.md) records 100.00% `tensor_vm` crate line coverage | Passed |
| CUDA feature gate | [`implementation_status.md`](implementation_status.md) records 169 `tensor_vm` tests under `--features cuda-kernels` | Passed locally |

## Acceptance Criteria Audit

| # | Requirement | Current evidence | Status |
| --- | --- | --- | --- |
| 1 | Miners execute deterministic tensor jobs | Miner, runtime, scheduler, and local testnet tests listed in `coverage_matrix.md` | Locally met |
| 2 | Validators verify block-eligible matmul jobs with full-output Freivalds or bounded equivalent | `verify::full_freivalds` and validator tests | Locally met |
| 3 | Row-sampled checks are audits unless bounds are documented | Row-sampling probability/study tests | Locally met |
| 4 | Blocks use settled prior-epoch TensorWork | Proposer-selection chain tests | Locally met |
| 5 | Rewards use verified settled TensorWork | Settlement and reward allocation tests | Locally met |
| 6 | Validation randomness is unbiasable after receipt roots are committed | Finalized-randomness seed tests and study utilities | Locally met |
| 7 | Invalid tensor outputs are rejected in dense and sparse corruption tests | TensorOp verifier corruption tests | Locally met |
| 8 | LinearTrainingStep validates forward/backward/error/update structure | Linear verifier, VM, and job tests | Locally met |
| 9 | Sparse corruptions in `dY` and `W_next` are rejected | Linear sparse poisoning tests | Locally met |
| 10 | Honest miners produce identical output roots | CUDA-enabled CPU/GPU parity and CUDA edge tests | Locally met |
| 11 | Validators spend materially less compute than recompute | Verification-cost study and telemetry | Locally met |
| 12 | Tensor data availability exceeds 95% during active and retention windows | Tensor server, validator unavailable, and telemetry tests; public measurement remains external | Locally met, public proof missing |
| 13 | Network runs 7 consecutive days with independent nodes | Preflight and evidence validators exist, but no external run evidence is present | Not complete |
| 14 | Genesis and zero-work epochs have fallback proposer path | Fallback proposer and liveness tests | Locally met |
| 15 | Reward concentration, validator disagreement, and data withholding are reported | Telemetry, study, and watcher tests | Locally met |

## Remaining Full-Spec Blockers

These are not satisfied by local tests or manifests alone:

- independently checkable public-run evidence must show production libp2p was used for discovery, gossip,
  and request/response propagation
- RPC, explorer, faucet, and telemetry must be deployed outside the local harness
- a public testnet must run for 7 consecutive days with independent external miner and validator operators
- the public evidence bundle must include a signed wall-clock run window, signed node heartbeats,
  block/finality history, operator attestations, data-availability measurements, invalid-work rejection
  evidence, reward-settlement records, signed production libp2p network-observation records, and signed
  deployed-service health records bound to external HTTPS URLs
- the external evidence bundle must be published and linked from [`implementation_status.md`](implementation_status.md)

Local evidence validators reject localhost, private, link-local, and empty publication endpoints, and they
verify manifest publication signatures, signed run-window records, signed block/finality/data-availability
summary roots, signed production libp2p network-observation roots, signed service-health records bound to
external HTTPS URLs, and external-operator evidence derived from signed manifest operator-attestation
records.
These blockers therefore require real external infrastructure rather than loopback, private-network,
unsigned, or out-of-band manifests.

## Completion Decision

The local deterministic reference implementation is strongly evidenced by the current tests, coverage, and
docs. The full `mvp_spec.md` objective is not complete because the deployment-gated requirements above
require external infrastructure and independently checkable public-run evidence that is not present in this
repository.
