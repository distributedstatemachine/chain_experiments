# TensorVM Coverage Matrix

This maps [`mvp_spec.md`](mvp_spec.md) acceptance criteria to concrete
implementation artifacts and tests.

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
| 10 | Honest miners produce identical output roots. | `runtime::tests::cpu_and_gpu_backends_match_canonical_matmul`, `runtime::tests::cpu_and_gpu_backends_match_linear_step`, `runtime::tests::cuda_kernel_matches_canonical_field_matmul_edges` under `--features cuda-kernels`, `chain::tests::redundant_agreement_quorum_is_required_before_settlement`, `scheduler::tests::miner_assignment_prefers_operator_separation`, `scheduler::tests::miner_assignment_falls_back_when_operator_diversity_is_insufficient` |
| 11 | Validators spend materially less compute than full recomputation. | `study::matmul_verification_cost_study`, `study::tests::matmul_verification_cost_is_lower_than_execution_for_mvp_shape`, `telemetry::estimated_verification_to_execution_ratio` |
| 12 | Tensor data availability exceeds 95% during active and retention windows. | `validator::tests::validator_attests_unavailable_when_server_lacks_tensor_roots`, `tensor_server::tests::tensor_server_retains_through_deadline_and_prunes_afterward`, `telemetry::data_availability_rate`; public-network measurement remains deployment-gated |
| 13 | Network runs for 7 consecutive days with independent nodes. | Not locally complete; `testnet::tests::local_testnet_bootstraps_required_public_shape`, `testnet::tests::public_testnet_run_evidence_requires_independent_external_operators`, `testnet::tests::public_testnet_run_evidence_requires_production_runtime_and_reachable_services`, `testnet::tests::public_testnet_evidence_bundle_requires_publication_and_audit_records`, `testnet::tests::public_testnet_evidence_manifest_parses_into_bundle`, `testnet::tests::public_testnet_evidence_manifest_rejects_malformed_input`, and `testnet::tests::public_testnet_run_evidence_filters_unsigned_and_short_lived_nodes` validate the evidence gate for expected block count, distinct external operators, signed heartbeats, run continuity, finality, data availability, invalid-work rejection, reward-settlement records, production libp2p runtime use, deployed RPC/explorer/faucet/telemetry service reachability, independently checkable evidence-bundle publication, and manifest parsing |
| 14 | Genesis and zero-work epochs have fallback proposer path. | `chain::tests::proposer_selection_uses_fallback_until_work_settles`, `study::tests::zero_work_liveness_study_produces_blocks_from_fallback` |
| 15 | Reward concentration, validator disagreement, and data withholding are reported. | `telemetry::TelemetrySnapshot`, `study::tensorwork_concentration`, `study::data_withholding_study`, `study::collusion_simulation`, `watcher::ChainWatcher`, `telemetry::tests::telemetry_reports_block_timing_and_concentration`, `telemetry::tests::telemetry_reports_security_compute_and_economic_success_metrics`, `telemetry::tests::telemetry_reports_hardware_classes_and_gpu_utilization`, `telemetry::tests::telemetry_reports_linear_receipt_bandwidth_and_missing_job_edges`, `watcher::tests::watcher_reports_invalid_receipts_and_data_withholding`, `watcher::tests::watcher_flags_validator_misconduct_in_audited_state`, `watcher::tests::watcher_flags_malformed_attestation_evidence`, `watcher::tests::watcher_reports_conflicting_linear_transitions` |

## Non-Local Gaps

- Optional native CUDA field-matmul kernel support exists behind `--features cuda-kernels` and is checked
  against canonical CPU outputs locally. Production GPU miner packaging and a broader optimized kernel suite
  remain outside the local reference crate.
- Public 7-day independent-node testnet evidence is not available in this repository; typed evidence
  validation exists for checking it when a real external run is available, including invalid-work rejection
  evidence, reward-settlement records, production libp2p runtime use, and deployed public-service
  reachability. The required public evidence-bundle shape is documented in
  [`public_testnet_evidence.md`](public_testnet_evidence.md), but no complete external bundle is linked yet.
- Production libp2p transport, HTTP deployment, full durable database, and deployed browser web services remain outside the local reference crate. The crate has local libp2p-shaped P2P simulation, generic framed `Read`/`Write` codec, framed stdlib TCP P2P send/receive, Kademlia-style closest-peer directory/bootstrap, durable peer-book persistence, peer-count admission, score-based drops, deterministic rate-limit/backoff policy checks, generic HTTP request reading, a socketed stdlib RPC server with auth/body/rate-limit policy checks, explorer/telemetry/faucet RPC endpoints, local browser-facing explorer/telemetry/faucet HTML pages, a documented libp2p-primary/Iroh-later networking choice, and a restartable reference `NodeStore` data directory with consistency-checked snapshot, append-only block-log, full-chain state, and peer-book persistence.
- Instrumented line coverage has been generated with Tarpaulin; see `tarpaulin_report.md`.
  Branch coverage is not reported because the installed Tarpaulin version lists branch coverage as not implemented.
