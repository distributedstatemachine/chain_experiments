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
4. Pass Gate 0: the default-feature CPU local multi-participant testnet with mandatory libp2p node paths,
   separate participant identities/endpoints, and no simulation or local-only networking-shim credit.
5. Provide a local preflight harness and evidence validator for Acceptance Criterion 13.
6. Run and document the required workspace verification commands.
7. Maintain 100% line coverage for `crates/tensor_vm/src` or document justified uncovered lines.
8. Provide real CUDA/C++ kernels for any claimed GPU path and verify them against CPU semantics.
9. Use production libp2p for node discovery, gossip, and request/response propagation.
10. Deploy RPC, explorer, faucet, and telemetry services outside the local test harness.
11. Run a 7-day public testnet with independent external operators.
12. Publish independently checkable evidence for the public run and link it from implementation status.

## Prompt-To-Artifact Checklist

| Requirement | Evidence | Status |
| --- | --- | --- |
| Required Cargo workspace structure | Root `Cargo.toml`, `crates/pearl_chain`, `crates/tensor_vm`, `docs/tensorvm`, and linked READMEs | Present |
| TensorVM crate is independent of Pearl Chain | `cargo tree -p tensor_vm` shows no `pearl_chain` dependency; rust-libp2p and serde are direct TensorVM runtime dependencies | Present |
| Recommended TensorVM modules exist | `api`, `chain`, `challenge`, `cli`, `error`, `explorer`, `faucet`, `jobs`, `merkle`, `miner`, `p2p`, `rpc`, `runtime`, `scheduler`, `storage`, `study`, `telemetry`, `tensor`, `tensor_server`, `testnet`, `txpool`, `types`, `validator`, `verify`, `vm`, `watcher` under `crates/tensor_vm/src` | Present |
| Gate 0 CPU local multi-participant testnet | `cargo test -p tensor_vm local_testnet --release` covers the local 10-miner/5-validator bootstrap shape, separate participant identities/endpoints, mandatory libp2p node paths under default features, local matmul settlement/rewards, LinearTrainingStep state transition, tensor-server availability, no simulation or local-only networking-shim credit, and non-public-run evidence separation | Passed |
| Local Acceptance Criteria 1-12, 14, 15 | [`coverage_matrix.md`](coverage_matrix.md) maps each criterion to concrete tests and artifacts | Locally covered |
| AC13 local preflight | [`public_testnet_preflight.md`](public_testnet_preflight.md), `parse_public_testnet_preflight_manifest`, checked health/content surface plans, and `tvmd public-testnet preflight --manifest <path>` | Present |
| Public deployment scaffold | `deploy/tensorvm/` contains systemd, nginx, environment, operator runbook, preflight-manifest, and post-run evidence-manifest templates; the checked preflight example is parseable but rejects placeholder special-use hosts until replaced, and the checked evidence example validates structurally while intentionally reporting `public_evidence_full_spec=false` | Present as scaffold and non-full-spec example |
| AC13 public evidence validator | [`public_testnet_evidence.md`](public_testnet_evidence.md), `parse_public_testnet_evidence_manifest`, `PublicTestnetEvidenceBundle`, external publication URI validation including well-formed HTTPS authorities, HTTPS evidence URI path enforcement with query/fragment rejection, raw-whitespace rejection, exact untrimmed manifest URI/path field validation, special-use DNS and single-label DNS rejection, and well-formed `ipfs://`/`ar://` identifiers, verified manifest publication signatures, signed independent-auditor records whose IDs differ from the manifest signer, signed wall-clock run-window evidence, signed block/finality/network-runtime/data-availability/invalid-work/reward-settlement summary roots, exact run-derived block/finality/data-availability/invalid-work summary counts, signed production-libp2p network-observation coverage for counted public operators, signed external artifact locators for raw supporting records, signed operator-attestation-derived external-operator evidence with disjoint miner/validator operator IDs and no overreported operator-attestation counts, run-window-bounded observation timestamps, heartbeat counts covering observed blocks, internally consistent finality/data-availability counters, signed deployed-service health counts covering observed blocks, signed deployed-service content-root evidence observed inside the signed run window with matching service-health HTTPS authorities, `deploy/tensorvm/RUNBOOK.md`, `deploy/tensorvm/manifests/public-testnet.evidence.example`, `tvmd public-evidence validate --manifest <path>`, `tvmd public-evidence publication ...`, `tvmd public-evidence auditor-record ...`, `tvmd public-evidence run-window ...`, `tvmd public-evidence node-heartbeat ...`, `tvmd public-evidence operator-attestation ...`, `tvmd public-evidence service-health ...` signed service-record generation, `tvmd public-evidence service-content ...` signed service-content generation, `tvmd public-evidence network-observation ...` signed public libp2p observation-record generation with non-public multiaddr, malformed DNS-label, and single-label DNS rejection, `tvmd public-evidence record-summary ...` signed supporting-record generation, `tvmd public-evidence record-artifact ...` signed raw-record artifact locator generation, and `tvmd public-evidence record-summary-from-roots ...` deterministic supporting-record root aggregation | Present |
| Miner, validator, and service CLI surfaces | `cli::parse_cli_args`, `cli::execute_reference_cli_command`, libp2p multiaddr validation for miner/validator nodes, required `tvmd service serve --p2p-listen <multiaddr>`, and `tvmd` binary entrypoint | Reference implementation present |
| CPU reference backend | `runtime::CpuReferenceBackend` and runtime tests | Present |
| GPU miner backend | `runtime::GpuMinerBackend` reports the selected CUDA device and rejects execution when native CUDA kernels are not compiled; CPU execution remains a separate `runtime::CpuReferenceBackend` | Present locally |
| Native CUDA/C++ checked against CPU outputs | `cuda-kernels` feature builds `kernels/cuda/field_matmul.cu`; `runtime::tests::cuda_kernel_matches_canonical_field_matmul_edges`, `runtime::tests::cuda_kernels_match_canonical_linear_tensor_ops`, and `runtime::tests::cpu_and_gpu_backends_match_linear_step` | Present for matmul and linear-step tensor ops |
| Restartable node storage | `storage::NodeStore`, snapshots, append-only block log, chain state, peer book tests | Reference implementation present |
| P2P/RPC runtime and socket tests | `p2p` rust-libp2p swarm, background libp2p service runtime, Kademlia/bootstrap, protocol tests, P2P codec tests, `rpc` socketed HTTP and health-route tests, `tvmd service init/serve` launch validation, public network-observation evidence rejected without nonzero TCP listen ports, and public evidence service URLs rejected when local, private, special-use DNS, single-label DNS, documentation, shared-address, benchmarking, multicast, reserved, or not exact query-free service paths | Reference implementation present |
| Required commands documented | [`implementation_status.md`](implementation_status.md) and [`tarpaulin_report.md`](tarpaulin_report.md) | Present |
| `cargo fmt --check --all` | Latest iteration evidence records pass from workspace root | Passed |
| `cargo test --workspace --release` | Latest iteration evidence records 14 `pearl_chain` and 167 `tensor_vm` tests | Passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Latest iteration evidence records pass from workspace root | Passed |
| `cargo tarpaulin` | [`tarpaulin_report.md`](tarpaulin_report.md) records 181 instrumented tests | Passed |
| TensorVM line coverage | [`tarpaulin_report.md`](tarpaulin_report.md) records 100.00% `tensor_vm` crate line coverage | Passed |
| CUDA feature gate | [`implementation_status.md`](implementation_status.md) records 171 `tensor_vm` tests under `--features cuda-kernels` | Passed locally |

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
| 13 | Network runs 7 consecutive days with independent nodes | Preflight and evidence validators exist, and the deploy evidence example is parser-checked as non-full-spec sample evidence, but no external run evidence is present | Not complete |
| 14 | Genesis and zero-work epochs have fallback proposer path | Fallback proposer and liveness tests | Locally met |
| 15 | Reward concentration, validator disagreement, and data withholding are reported | Telemetry, study, and watcher tests | Locally met |

## Remaining Full-Spec Blockers

These are not satisfied by local tests or manifests alone:

- independently checkable public-run evidence must show production libp2p was used for discovery, gossip,
  and request/response propagation
- RPC, explorer, faucet, and telemetry must be deployed outside the local harness
- a public testnet must run for 7 consecutive days with independent external miner and validator operators
- the public evidence bundle must include a signed wall-clock run window, signed node heartbeats,
  block/finality history, signed independent-auditor records, operator attestations, data-availability
  measurements, invalid-work rejection evidence, reward-settlement records, signed production libp2p
  network-observation records, signed external raw-record artifact locators, signed deployed-service health
  records that cover the observed block count, and signed deployed-service content-root records bound to
  external HTTPS URLs
- the external evidence bundle must be published and linked from [`implementation_status.md`](implementation_status.md)

Local evidence validators reject malformed HTTPS authorities, userinfo, whitespace, invalid DNS host
labels, invalid ports, missing HTTPS evidence paths, HTTPS evidence query strings or fragments, localhost,
`.local`, `.localhost`, `.test`, `.example`, `.invalid`, RFC example domains, private, link-local,
documentation, shared-address, benchmarking,
multicast, reserved, empty publication endpoints, and malformed `ipfs://`/`ar://` identifiers, and they verify manifest publication
signatures, signed run-window records, signed
block/finality/data-availability/invalid-work/reward-settlement summary roots, signed production libp2p
network-observation roots covering counted public operators, signed external artifact locators for the raw records behind each summary root,
signed service-health records bound to external HTTPS URLs, matching health paths, and observed-block
coverage, signed
independent-auditor records bound to
external audit URIs, signed service-content roots bound to external HTTPS URLs, matching service endpoint
IDs, matching service-health HTTPS authorities, required content paths, exact run-derived supporting-record
counts, and external-operator evidence derived from signed manifest
operator-attestation records plus node heartbeat counts that cover the observed block count while rejecting
overreported operator-attestation counts.
These blockers therefore require real external infrastructure rather than loopback, private-network,
reserved-range, unsigned, cross-authority service-content, or out-of-band manifests.

## Completion Decision

The local deterministic reference implementation is strongly evidenced by the current tests, coverage, and
docs. The full `mvp_spec.md` objective is not complete because the deployment-gated requirements above
require external infrastructure and independently checkable public-run evidence that is not present in this
repository.
