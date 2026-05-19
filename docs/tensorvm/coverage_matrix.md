# TensorVM Coverage Matrix

This maps [`mvp_spec.md`](mvp_spec.md) acceptance criteria to concrete
implementation artifacts and tests.

## Gate 0

The first non-skippable spec gate is the default-feature CPU local multi-participant testnet. It is
checked with:

```bash
cargo test -p tensor_vm local_testnet --release
```

That filtered test run covers:

- `testnet::tests::local_testnet_bootstraps_required_public_shape`
- `testnet::tests::local_testnet_runs_full_matmul_receipt_attestation_settlement_round`
- `testnet::tests::local_testnet_runs_linear_training_receipt_state_transition_round`
- `p2p::tests::local_testnet_libp2p_swarms_exchange_gossip_and_request_response`

These tests exercise the CPU reference path with the default local 10-miner/5-validator shape, separate
local participant identities and libp2p endpoints, a live mandatory libp2p control-plane startup under
default features, real loopback libp2p delivery across every TensorVM gossip topic and request-response
message family, local block production, matmul receipt validation/attestation/settlement/rewards,
LinearTrainingStep validation and state transition, local tensor-server availability, no simulation or
local-only networking-shim credit, and the explicit separation between local evidence and the 7-day public
deployment gate.

## Acceptance Criteria

| # | Criterion | Evidence |
| --- | --- | --- |
| 1 | Miners execute deterministic tensor jobs. | `miner::tests::miner_solves_matmul_and_serves_tensors`, `miner::tests::miner_solves_linear_step_and_serves_intermediates`, `runtime::tests::cpu_and_gpu_backends_match_canonical_matmul` |
| 2 | Validators verify block-eligible matmul jobs with full-output Freivalds or bounded equivalent. | `verify::full_freivalds`, `verify::tests::full_freivalds_accepts_honest_and_rejects_corruption`, `verify::tests::tensor_op_verifier_rejects_metadata_and_shape_mismatches`, `validator::tests::validator_verifies_matmul_from_tensor_server` |
| 3 | Row-sampled checks are audits unless false-accept bounds are documented. | `verify::row_sample_detection_probability`, `study::row_sampling_study`, `study::tests::row_sampling_study_blocks_sparse_row_sampled_only_acceptance` |
| 4 | Blocks use settled prior-epoch TensorWork. | `chain::LocalChain::proposer_for_next_epoch`, `chain::tests::proposer_selection_ignores_pending_tensorwork` |
| 5 | Rewards are distributed by verified settled TensorWork. | `chain::tests::chain_settles_valid_tensorwork_and_rewards_participants`, `chain::tests::reward_allocation_matches_mvp_split_and_credits_proposer_and_treasury` |
| 6 | Validation randomness is unbiasable after receipt roots are committed. | `chain::LocalChain::validation_seed`, `study::assess_randomness`, `chain::tests::validation_seed_is_bound_to_finalized_randomness_and_receipt` |
| 7 | Invalid tensor outputs are rejected in dense and sparse corruption tests. | `verify::tests::tensor_op_verifier_rejects_bad_output`, `verify::tests::full_freivalds_accepts_honest_and_rejects_corruption` |
| 8 | LinearTrainingStep receipts validate forward/backward/error/update structure. | `verify::verify_linear_training_step`, `verify::tests::linear_training_verifier_rejects_metadata_and_commitment_mismatches`, `vm::tests::linear_backward_and_sgd_match_equations`, `jobs::tests::linear_receipt_commits_to_learning_step` |
| 9 | Sparse corruptions in `dY` and `W_next` are rejected with stated probability. | `verify::tests::linear_training_verifier_rejects_sparse_error_poisoning`, `verify::tests::linear_training_verifier_rejects_sparse_weight_poisoning` |
| 10 | Honest miners produce identical output roots. | `runtime::tests::gpu_backend_reports_device_and_requires_cuda_kernels`, `runtime::tests::cpu_and_gpu_backends_match_canonical_matmul`, `runtime::tests::cpu_and_gpu_backends_match_linear_step`, `runtime::tests::cuda_kernel_matches_canonical_field_matmul_edges`, and `runtime::tests::cuda_kernels_match_canonical_linear_tensor_ops` under `--features cuda-kernels`, `chain::tests::redundant_agreement_quorum_is_required_before_settlement`, `scheduler::tests::miner_assignment_prefers_operator_separation`, `scheduler::tests::miner_assignment_falls_back_when_operator_diversity_is_insufficient` |
| 11 | Validators spend materially less compute than full recomputation. | `study::matmul_verification_cost_study`, `study::tests::matmul_verification_cost_is_lower_than_execution_for_mvp_shape`, `telemetry::estimated_verification_to_execution_ratio` |
| 12 | Tensor data availability exceeds 95% during active and retention windows. | `validator::tests::validator_attests_unavailable_when_server_lacks_tensor_roots`, `tensor_server::tests::tensor_server_retains_through_deadline_and_prunes_afterward`, `telemetry::data_availability_rate`; public-network measurement remains deployment-gated |
| 13 | Network runs for 7 consecutive days with independent nodes. | Not locally complete; `testnet::tests::local_testnet_bootstraps_required_public_shape`, `testnet::tests::public_testnet_preflight_manifest_reports_launch_readiness`, `testnet::tests::deployed_public_testnet_preflight_example_rejects_placeholder_domains`, `testnet::tests::docs_public_testnet_preflight_manifest_rejects_placeholder_domains`, `testnet::tests::public_testnet_preflight_manifest_rejects_malformed_input`, `cli::tests::execute_reference_cli_command_reports_miner_and_validator_readiness`, `cli::tests::validate_public_testnet_preflight_manifest_reports_launch_readiness`, `tvmd` binary `tests::docs_public_testnet_preflight_command_reports_pending_status`, `tvmd` binary `tests::docs_public_testnet_evidence_command_reports_non_full_spec_status`, `tvmd_cli::documented_public_testnet_preflight_command_reports_pending_status`, `tvmd_cli::generated_public_testnet_preflight_manifest_reports_ready`, `tvmd_cli::documented_public_testnet_evidence_command_reports_non_full_spec_status`, `tvmd_cli::generated_public_evidence_manifest_round_trips_through_tvmd_validator`, `tvmd_cli::service_cli_lifecycle_starts_libp2p_and_serves_public_surfaces`, `p2p::tests::peer_book_store_upserts_bootstrap_records_with_peer_ids`, `rpc::tests::node_rpc_serves_head_and_blocks`, `rpc::tests::node_rpc_serves_explorer_telemetry_and_faucet_routes`, `testnet::tests::public_testnet_run_evidence_requires_independent_external_operators`, `testnet::tests::public_testnet_run_evidence_requires_production_runtime_and_reachable_services`, `testnet::tests::public_testnet_evidence_bundle_requires_publication_and_audit_records`, `testnet::tests::public_testnet_evidence_manifest_parses_into_bundle`, `testnet::tests::deployed_public_testnet_evidence_example_is_parseable_but_not_full_spec`, `testnet::tests::docs_public_testnet_evidence_manifest_is_parseable_but_not_full_spec`, `testnet::tests::public_testnet_evidence_manifest_rejects_malformed_input`, and `testnet::tests::public_testnet_run_evidence_filters_unsigned_and_short_lived_nodes` validate the local launch preflight plus service-launch config and health/content endpoints, checked spec-path pending manifests and deploy preflight/evidence examples with planned public content paths, actual `tvmd` file-reading and process invocation behavior for the documented pending-manifest commands, process-generated launch-ready external-addressed preflight manifest validation from disk, process-generated short-run evidence-manifest assembly from signed `tvmd public-evidence ...` generator commands that validates from disk as independently checkable without setting the full-spec flag, bounded process-level service init/peer-add/serve lifecycle with mandatory libp2p startup, unauthenticated request rejection, authenticated `/health`, `/rpc/health`, `/explorer/health`, `/faucet/health`, `/telemetry/health`, process-level signed service-health generation from reached RPC/explorer/faucet/telemetry health responses, state-root-bearing `/chain/head`, `/epoch/current`, `/jobs/current`, the empty-chain `/chain/block/0` route response, `/explorer`, `/faucet/page`, `/telemetry/dashboard`, mutable `/tx`, `/receipt`, and `/attestation` submissions, registered miner/validator state read-back, captured `/chain/head`, `/explorer`, `/faucet/page`, and `/telemetry/dashboard` response-body evidence generation through matching `service-content-from-bytes` and `service-content-from-file` CLI outputs, process-derived local libp2p peer/protocol data accepted only when bound to an external public multiaddr and then summarized/artifact-bound from its network-runtime observation root, the same process-derived data rejected as public network-observation evidence when bound to loopback, exact query-free service URL path enforcement, and placeholder-domain rejection, signed publication/auditor-record/run-window/node-heartbeat/operator-attestation CLI generation and invalid argument rejection, service peer-book bootstrap seeding with peer-ID-preserving `/p2p/<peer-id>` dial addresses, service-health and service-content CLI manifest-line generation, byte-derived and file-derived service-content root generation, plus invalid argument rejection, signed production-libp2p network-observation CLI generation and invalid argument rejection including malformed DNS-label and single-label DNS multiaddrs, signed supporting-record summary generation, signed external supporting-record artifact locator generation, signed artifact locator generation from derived aggregate roots, plus deterministic root aggregation for block/finality/network-runtime/data-availability/invalid-work/reward-settlement evidence, evidence gate for signed 7-day wall-clock run-window evidence, expected block count, distinct external operators, signature-verified heartbeat summaries, run continuity, finality, data availability, invalid-work rejection, reward-settlement records, production libp2p runtime use, signed per-operator production libp2p network-observation records exactly matching counted public operators and aggregating to signed network-runtime summary roots, deployed RPC/explorer/faucet/telemetry service reachability with signed health summaries and signed content roots bound to external HTTPS URLs, matching and distinct endpoint IDs, distinct service-content roots, and the required content paths, external public evidence publication URI validation including special-use DNS and single-label DNS rejection, verified manifest publication signatures, signed independent-auditor records, signed block/finality/data-availability/invalid-work/reward-settlement summary roots, signed operator-attestation-derived external-operator evidence, independently checkable evidence-bundle publication, and manifest parsing |
| 14 | Genesis and zero-work epochs have fallback proposer path. | `chain::tests::proposer_selection_uses_fallback_until_work_settles`, `study::tests::zero_work_liveness_study_produces_blocks_from_fallback` |
| 15 | Reward concentration, validator disagreement, and data withholding are reported. | `telemetry::TelemetrySnapshot`, `study::tensorwork_concentration`, `study::data_withholding_study`, `study::collusion_risk_assessment`, `watcher::ChainWatcher`, `telemetry::tests::telemetry_reports_block_timing_and_concentration`, `telemetry::tests::telemetry_reports_security_compute_and_economic_success_metrics`, `telemetry::tests::telemetry_reports_hardware_classes_and_gpu_utilization`, `telemetry::tests::telemetry_reports_linear_receipt_bandwidth_and_missing_job_edges`, `watcher::tests::watcher_reports_invalid_receipts_and_data_withholding`, `watcher::tests::watcher_flags_validator_misconduct_in_audited_state`, `watcher::tests::watcher_flags_malformed_attestation_evidence`, `watcher::tests::watcher_reports_conflicting_linear_transitions` |

## Non-Local Gaps

- Optional native CUDA kernel support exists behind `--features cuda-kernels` and covers field matmul plus
  linear-step sub/scalar/transpose/squared-error kernels checked against canonical CPU outputs locally.
  Miner CLI startup reports CPU reference readiness for `--device cpu` and rejects `--device cuda:N`
  unless CUDA kernels are compiled and the requested device is available.
  Production GPU miner packaging and a broader optimized kernel suite remain outside the local reference
  crate.
- Public 7-day independent-node testnet evidence is not available in this repository; typed evidence
  validation exists for checking it when a real external run is available, including signed wall-clock
  run-window evidence, invalid-work rejection evidence, reward-settlement records, production libp2p
  runtime use with signed per-operator network-observation records exactly matching counted public
  operators and aggregating to the signed network-runtime root,
  an exact one-signature manifest publication count for the current manifest format, exactly one signed
  external artifact locator for each required raw supporting-record kind, disjoint miner/validator operator IDs and node addresses, auditor IDs distinct
  from the manifest signer with auditor observations at or after the signed run end and valid signed
  auditor-record counts exactly matching `independent_auditor_count`,
  operator-attestation and service-content timestamps inside the signed run window, observed-block
  coverage for node heartbeat and service health counts, internally consistent finality/data-availability
  counters, exact run-derived supporting-record summary counts, non-public IP literal rejection,
  special-use DNS and single-label DNS rejection, plus
  malformed HTTPS authority rejection for public endpoints, raw-whitespace rejection for external evidence
  URLs and content-addressed identifiers including exact untrimmed manifest URI/path fields, HTTPS evidence
  URI path enforcement with query and fragment rejection, duplicate scalar manifest-field rejection,
  whitespace-padded field-key rejection, duplicate supporting-record root rejection, repeated node-address count rejection, exact service URL path matching with query and fragment rejection, no overreported operator-attestation counts,
  full-spec flag rejection for relaxed local harness criteria, well-formed `ipfs://`/`ar://` identifier
  validation, and deployed public-service reachability plus distinct endpoint IDs and distinct content
  roots with at least 64 observed bytes bound to external HTTPS URLs. A local launch
  preflight manifest is documented in
  [`public_testnet_preflight.md`](public_testnet_preflight.md), requires a CUDA-ready miner count matching
  the planned miner count before deployment readiness can pass, and deployment templates plus checked
  preflight and non-full-spec post-run evidence example manifests live under `deploy/tensorvm/`,
  `deploy/tensorvm/RUNBOOK.md` records the external evidence collection and publication flow, signed public
  libp2p network-observation CLI generation rejects missing or zero TCP listen ports plus non-public and
  single-label DNS multiaddrs, `network-observation-from-service-log` derives signed observation records
  from captured `tvmd service serve` logs while still requiring public listen multiaddrs, process-level
  network-runtime observation roots can be summarized and artifact-bound from external-addressed records or
  saved raw-record files, and file-derived block/finality/data-availability/invalid-work/reward supporting
  record summaries can hash exact typed raw-record lines while rejecting whitespace-padded records,
  `run-window-from-file` derives signed run-window manifest lines from saved
  contiguous per-block observation files while rejecting duplicate blocks, gaps, zero timestamps,
  decreasing timestamps, unsupported lines, and whitespace-padded records, `node-heartbeat-from-file`
  derives signed node-heartbeat manifest lines from saved contiguous per-block observation files while
  rejecting duplicate blocks, gaps, identity mismatches, unsupported lines, and whitespace-padded records,
  `service-health-from-file` derives signed
  service-health manifest lines from saved contiguous per-block observation files while rejecting duplicate
  blocks, gaps, unsupported lines, and whitespace-padded records, service health/content evidence must use
  matching HTTPS authorities for each endpoint ID, and the required post-run evidence-bundle shape is
  documented in
  [`public_testnet_evidence.md`](public_testnet_evidence.md), but no complete external bundle is linked yet.
- Public production libp2p run evidence, HTTP deployment, full durable database, and deployed browser web
  services remain outside the local reference crate. The crate has mandatory rust-libp2p runtime wiring with
  TCP/TLS/Yamux swarm construction, Gossipsub subscriptions, Identify, Kademlia discovery/address
  registration, JSON request-response protocols, `tvmd service peer add` bootstrap seeding,
  `tvmd service serve` startup of a libp2p control-plane runtime, durable bootstrap peer-book persistence
  with peer-ID-preserving dial multiaddrs, generic HTTP request reading, a socketed stdlib RPC server with auth/body/rate-limit policy checks,
  explorer/telemetry/faucet RPC endpoints, local browser-facing explorer/telemetry/faucet HTML pages,
  `tvmd service init/peer add/serve` launch validation with required libp2p listen multiaddrs, deployable
  systemd/nginx templates, a documented mandatory-libp2p networking choice, and a restartable reference
  `NodeStore` data
  directory with consistency-checked snapshot, append-only block-log, full-chain state, and peer-book
  persistence.
- Instrumented line coverage has been generated with Tarpaulin; see `tarpaulin_report.md`.
  Branch coverage is not reported because the installed Tarpaulin version lists branch coverage as not implemented.
