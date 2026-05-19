# TensorVM Completion Audit

Audit date: May 19, 2026.

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
| Gate 0 CPU local multi-participant testnet | `cargo test -p tensor_vm local_testnet --release` covers the local 10-miner/5-validator bootstrap shape, separate participant identities/libp2p endpoints, live mandatory libp2p control-plane startup under default features, real loopback libp2p delivery across every TensorVM gossip topic and request-response message family, local matmul settlement/rewards, LinearTrainingStep state transition, tensor-server availability, no simulation or local-only networking-shim credit, and non-public-run evidence separation | Passed |
| Local Acceptance Criteria 1-12, 14, 15 | [`coverage_matrix.md`](coverage_matrix.md) maps each criterion to concrete tests and artifacts | Locally covered |
| AC13 local preflight | [`public_testnet_preflight.md`](public_testnet_preflight.md), checked pending manifest [`public-testnet.preflight`](public-testnet.preflight), `parse_public_testnet_preflight_manifest`, checked CUDA-ready miner count matching planned miner count, checked libp2p-ready node count matching planned miner plus validator count, checked health/content surface plans, `tvmd public-testnet preflight --manifest <path>`, a `tvmd` binary file-read test, a `tvmd` CLI integration test for `docs/tensorvm/public-testnet.preflight` reporting `public_testnet_preflight_ready=false` while placeholder hosts remain, and a generated external-addressed preflight manifest read from disk by `tvmd` reporting `public_testnet_preflight_ready=true` | Present |
| Public deployment scaffold | `deploy/tensorvm/` contains systemd, nginx, environment, operator runbook, preflight-manifest, and post-run evidence-manifest templates; the checked preflight example and [`public-testnet.preflight`](public-testnet.preflight) are parseable but reject placeholder special-use hosts until replaced, and the checked evidence example plus [`public-testnet.evidence`](public-testnet.evidence) validate structurally while intentionally reporting `public_evidence_full_spec=false` | Present as scaffold and non-full-spec examples |
| AC13 public evidence validator | [`public_testnet_evidence.md`](public_testnet_evidence.md), checked pending manifest [`public-testnet.evidence`](public-testnet.evidence), `parse_public_testnet_evidence_manifest`, `PublicTestnetEvidenceBundle`, external publication URI validation including well-formed HTTPS authorities, HTTPS evidence URI concrete-path enforcement with root-only/query/fragment rejection, raw-whitespace rejection, exact untrimmed manifest URI/path field validation, special-use DNS and single-label DNS rejection, and well-formed `ipfs://`/`ar://` identifiers with traversal/query/fragment path rejection, verified manifest publication signatures, signed independent-auditor records whose IDs differ from the manifest signer and whose valid signed count exactly matches `independent_auditor_count`, signed wall-clock run-window evidence, signed block/finality/network-runtime/data-availability/invalid-work/reward-settlement summary roots, exact run-derived block/finality/network-runtime/data-availability/invalid-work summary counts, signed per-operator production-libp2p network-observation records exactly matching counted public operators and aggregating to the network-runtime root, exactly one signed external artifact locator for each required raw supporting-record kind, signed operator-attestation-derived external-operator evidence with disjoint miner/validator operator IDs and no overreported operator-attestation counts, run-window-bounded observation timestamps, heartbeat counts covering observed blocks, internally consistent finality/data-availability counters, signed deployed-service health counts covering observed blocks, signed deployed-service content-root evidence observed inside the signed run window with matching service-health HTTPS authorities, `deploy/tensorvm/RUNBOOK.md`, `deploy/tensorvm/manifests/public-testnet.evidence.example`, `tvmd public-evidence validate --manifest <path>` plus `tvmd` binary and CLI integration tests for `docs/tensorvm/public-testnet.evidence` reporting `public_evidence_full_spec=false` while placeholder evidence remains, process-level generation of a short external-addressed evidence manifest from `tvmd public-evidence ...` subcommands that validates from disk as `independently_checkable=true` and `public_evidence_full_spec=false`, `tvmd public-evidence publication ...`, `tvmd public-evidence auditor-record ...`, `tvmd public-evidence run-window ...`, `tvmd public-evidence node-heartbeat ...`, `tvmd public-evidence node-heartbeat-from-file ...` derivation from saved contiguous per-block heartbeat-observation files with duplicate/gap/identity-mismatch/unsupported-line rejection, `tvmd public-evidence operator-attestation ...`, `tvmd public-evidence service-health ...` signed service-record generation with root-only/query/fragment health URL rejection, `tvmd public-evidence service-health-from-file ...` derivation from saved contiguous per-block health-observation files with duplicate/gap/unsupported-line rejection, `tvmd public-evidence service-content ...` signed service-content generation with root-only/query/fragment content URL rejection, `tvmd public-evidence service-content-from-bytes ...` signed service-content generation from captured response bytes, `tvmd public-evidence service-content-from-file ...` signed service-content generation from captured response files, `tvmd public-evidence network-observation ...` signed public libp2p observation-record generation, `tvmd public-evidence network-observation-from-service-log ...` signed public libp2p observation-record generation from captured `tvmd service serve` logs, with process-level public-address observation root extraction from live service peer/protocol/control fields plus non-public multiaddr, malformed DNS-label, and single-label DNS rejection, `tvmd public-evidence record-summary ...` signed supporting-record generation, `tvmd public-evidence record-artifact ...` signed raw-record artifact locator generation, `tvmd public-evidence record-artifact-from-roots ...` and `tvmd public-evidence record-summary-from-roots ...` deterministic root aggregation, plus `tvmd public-evidence record-artifact-from-file ...` and `tvmd public-evidence record-summary-from-file ...` derivation from saved raw-record files including process-observed network-runtime records and typed block/finality/data-availability/invalid-work/reward supporting-record lines with kind-specific field validation, exact-line hashing, whitespace-padded record rejection, and empty-field rejection | Present |
| Miner, validator, and service CLI surfaces | `cli::parse_cli_args`, `cli::execute_reference_cli_command`, libp2p multiaddr validation for miner/validator nodes, `tvmd service peer add --peer-id <peer-id> --address <multiaddr>` durable bootstrap seeding, required `tvmd service serve --p2p-listen <multiaddr>`, `tvmd` binary entrypoint, and a process-level `tvmd service init` / `tvmd service peer add` / bounded `tvmd service serve` integration smoke test for the required health and public content routes | Reference implementation present |
| CPU reference backend | `runtime::CpuReferenceBackend` and runtime tests | Present |
| GPU miner backend | `runtime::GpuMinerBackend` reports the selected CUDA device and rejects execution when native CUDA kernels are not compiled; `tvmd miner start --device cuda:N` requires compiled CUDA kernels plus an available CUDA device before reporting GPU readiness, while `--device cpu` remains the portable reference backend | Present locally |
| Native CUDA/C++ checked against CPU outputs | `cuda-kernels` feature builds `kernels/cuda/field_matmul.cu`; `runtime::tests::cuda_kernel_matches_canonical_field_matmul_edges`, `runtime::tests::cuda_kernels_match_canonical_linear_tensor_ops`, and `runtime::tests::cpu_and_gpu_backends_match_linear_step` | Present for matmul and linear-step tensor ops |
| Restartable node storage | `storage::NodeStore`, snapshots, append-only block log, chain state, peer book tests | Reference implementation present |
| P2P/RPC runtime and socket tests | `p2p` rust-libp2p swarm, background libp2p service runtime, Kademlia/bootstrap, protocol tests, P2P codec tests, peer-book upsert and `/p2p/<peer-id>` bootstrap address loading, `rpc` socketed HTTP and health-route tests, state-root-bearing `/chain/head` responses, `tvmd service init/peer add/serve` launch validation, `tvmd_cli::service_cli_lifecycle_starts_libp2p_and_serves_public_surfaces` covering unauthenticated request rejection, `/health`, `/rpc/health`, `/explorer/health`, `/faucet/health`, `/telemetry/health`, `/chain/head`, `/epoch/current`, `/jobs/current`, the empty-chain `/chain/block/0` route response, `/explorer`, `/faucet/page`, `/telemetry/dashboard`, mutable `/tx`, `/receipt`, and `/attestation` submissions, registered miner/validator state read-back, process-level signed `service-health` generation for reached RPC/explorer/faucet/telemetry health responses, captured `/chain/head`, `/explorer`, `/faucet/page`, and `/telemetry/dashboard` response-body evidence generation through matching `service-content-from-bytes` and `service-content-from-file` CLI outputs, process-derived libp2p peer/protocol/control data accepted only when bound to an external public multiaddr directly and through `network-observation-from-service-log`, immediately summarized/artifact-bound from the resulting network-runtime root, and the same local libp2p peer/protocol data rejected as public network-observation evidence when bound to loopback, public network-observation evidence rejected without nonzero TCP listen ports, and public evidence/preflight service URLs rejected when local, private, special-use DNS, single-label DNS, documentation, shared-address, benchmarking, multicast, reserved, root-only, query/fragment-bearing, or not exact query-free service paths | Reference implementation present |
| Required commands documented | [`implementation_status.md`](implementation_status.md) and [`tarpaulin_report.md`](tarpaulin_report.md) | Present |
| `cargo fmt --check --all` | Latest iteration evidence records pass from workspace root | Passed |
| `cargo test --workspace --release` | Latest iteration evidence records 14 `pearl_chain`, 173 `tensor_vm` library tests, 2 `tvmd` binary tests, and 5 `tvmd` CLI integration tests | Passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Latest iteration evidence records pass from workspace root | Passed |
| `cargo tarpaulin` | [`tarpaulin_report.md`](tarpaulin_report.md) records 187 instrumented tests | Passed |
| TensorVM line coverage | [`tarpaulin_report.md`](tarpaulin_report.md) records 100.00% `tensor_vm` crate line coverage, 7912/7912 lines covered | Passed |
| CUDA feature gate | [`implementation_status.md`](implementation_status.md) records 177 `tensor_vm` tests under `--features cuda-kernels` | Passed locally |

Current AC13 evidence tooling also includes `tvmd public-evidence run-window-from-file ...`, which derives
signed run-window evidence from saved contiguous `run_window_observation=<block>,<unix-seconds>` records
while rejecting duplicate blocks, gaps, zero timestamps, decreasing timestamps, unsupported lines, and
whitespace-padded records.
File-derived `network-runtime` summaries now validate each signed
`network_runtime_observation=...` raw record before aggregation, including the libp2p peer ID, public
multiaddr, nonzero counters, observation root, and observation signature.
File-derived block/finality/data-availability/invalid-work/reward summaries now validate typed raw-record
fields before exact-line hashing, including block/receipt roots, status enums, numeric block fields, and
empty-field rejection.

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

| Missing full-spec artifact | Required evidence | Current repository evidence | Blocker |
| --- | --- | --- | --- |
| Production public libp2p operation | Signed per-operator `network_runtime_observation=...` records for every counted public miner and validator operator, proving discovery, gossip, request/response, and DoS controls, with roots aggregated into the signed network-runtime summary | Local runtime wiring, checked bootstrap peer seeding/loading, CLI record generation, manifest validation, and non-full-spec examples | Needs external observers and public libp2p addresses from a real run |
| Deployed RPC service | External HTTPS RPC endpoint, signed service-health record, signed `/chain/head` content-root record, matching endpoint ID and authority | `tvmd service serve`, nginx/systemd templates, local route tests, and service-content verifier | Needs owned public DNS/TLS and a reachable deployed service |
| Deployed explorer service | External HTTPS explorer endpoint, signed service-health record, signed `/explorer` content-root record, matching endpoint ID and authority | Local explorer route, nginx/systemd templates, and service-content verifier | Needs owned public DNS/TLS and a reachable deployed service |
| Deployed faucet service | External HTTPS faucet endpoint, signed service-health record, signed `/faucet/page` content-root record, matching endpoint ID and authority | Local faucet route, nginx/systemd templates, and service-content verifier | Needs owned public DNS/TLS and a reachable deployed service |
| Deployed telemetry service | External HTTPS telemetry endpoint, signed service-health record, signed `/telemetry/dashboard` content-root record, matching endpoint ID and authority | Local telemetry route, nginx/systemd templates, and service-content verifier | Needs owned public DNS/TLS and a reachable deployed service |
| 7-day public testnet run | Signed wall-clock run window of at least 604800 seconds, at least 100800 observed blocks at default block time, 10 independent miners, and 5 independent validators | Gate 0 local CPU multi-participant testnet and AC13 evidence validator | Needs independent external operators and a completed public run |
| Raw supporting records | Exactly one external artifact locator signed against each block/finality/network-runtime/data-availability/invalid-work/reward-settlement summary root | CLI generators and validator support for exact `record_artifact=...` lines | Needs published raw records from the external run |
| Independent audit records | Signed auditor records whose auditor IDs differ from the manifest signer, whose observations occur at or after the run end, and whose valid signed count exactly matches `independent_auditor_count` with extra lines rejected | CLI generator and manifest validation | Needs independent external auditors or verifiers |
| Published evidence bundle | Public `https://`, `ipfs://`, or `ar://` manifest URI that validates with `public_evidence_full_spec=true` and is linked from [`implementation_status.md`](implementation_status.md) | Manifest format, parser, validator, and non-full-spec example | Needs the completed external run bundle to be published |

Local evidence validators reject malformed HTTPS authorities, userinfo, whitespace, invalid DNS host
labels, invalid ports, missing or root-only HTTPS evidence paths, HTTPS evidence query strings or fragments, localhost,
`.local`, `.localhost`, `.test`, `.example`, `.invalid`, RFC example domains, private, link-local,
documentation, shared-address, benchmarking,
multicast, reserved, empty publication endpoints, and malformed `ipfs://`/`ar://` identifiers or path segments, and they verify manifest publication
signatures with the current exact one-signature manifest count, reject duplicate scalar manifest fields,
reject whitespace-padded field keys, reject duplicate supporting-record roots, signed run-window records, signed
block/finality/data-availability/invalid-work/reward-settlement summary roots, signed production libp2p
network-observation roots exactly matching counted public operators, signed external artifact locators for the raw records behind each summary root,
signed service-health records bound to external HTTPS URLs, matching health paths, and observed-block
coverage, signed
independent-auditor records bound to
external audit URIs with observations at or after the signed run end and exact
`independent_auditor_count` matching, signed service-content roots bound to
external HTTPS URLs, matching service endpoint IDs, distinct deployed service endpoint IDs, matching
service-health HTTPS authorities, distinct service-content roots, required content paths, 64-byte minimum
content proofs, exact run-derived supporting-record counts, rejection of
full-spec evidence status under relaxed local harness criteria, and external-operator evidence derived from signed manifest
operator-attestation records plus distinct node-heartbeat addresses and counts that cover the observed block
count while rejecting overreported operator-attestation counts.
These blockers therefore require real external infrastructure rather than loopback, private-network,
reserved-range, unsigned, cross-authority service-content, or out-of-band manifests.

## Completion Decision

The local deterministic reference implementation is strongly evidenced by the current tests, coverage, and
docs. The full `mvp_spec.md` objective is not complete because the deployment-gated requirements above
require external infrastructure and independently checkable public-run evidence that is not present in this
repository.
