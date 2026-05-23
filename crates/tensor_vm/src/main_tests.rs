use super::*;
use tensor_vm::{
    ChainCommand, ChainEngine, ChainNetworkPayloadProcessor, ChainParams, FreivaldsParams,
    NetworkPayloadApply, PendingNetworkPayloads, ValidatorAttestation, VerificationResult,
    encode_attestation_payload, encode_job_payload, encode_receipt_payload, network_ingest_order,
    node::{
        apply_network_attestation_payload, apply_network_job_payload,
        apply_network_receipt_payload, attestation_announcement_hash,
    },
};
use tensor_vm::{ChainSnapshot, types::address};

fn workspace_manifest_path(relative_path: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn docs_public_testnet_preflight_command_reports_pending_status() {
    let report = execute_command(&CliCommand::PublicTestnetPreflight {
        manifest: workspace_manifest_path("docs/tensorvm/public-testnet.preflight"),
    })
    .unwrap();

    assert!(report.contains("public_testnet_preflight_ready=false"));
    assert!(report.contains("local_shape_ready=true"));
    assert!(report.contains("deployment_plan_ready=false"));
    assert!(report.contains("miners=10"));
    assert!(report.contains("validators=5"));
    assert!(report.contains("production_libp2p_runtime=true"));
    assert!(report.contains("public_services_planned=false"));
}

#[test]
fn docs_public_testnet_evidence_command_reports_non_full_spec_status() {
    let report = execute_command(&CliCommand::PublicEvidenceValidate {
        manifest: workspace_manifest_path("docs/tensorvm/public-testnet.evidence"),
    })
    .unwrap();

    assert!(report.contains("public_evidence_full_spec=false"));
    assert!(report.contains("public_criterion=false"));
    assert!(report.contains("independently_checkable=false"));
    assert!(report.contains("published_evidence_bundle=false"));
    assert!(report.contains("signed_run_window=true"));
    assert!(report.contains("supporting_record_artifacts=false"));
    assert!(report.contains("deployed_public_service_content=false"));
    assert!(report.contains("required_run_duration=false"));
    assert!(report.contains("required_block_count=false"));
}

#[test]
fn service_init_recovers_torn_snapshot_and_block_log_from_chain_state() {
    let data_dir = std::env::temp_dir().join(format!(
        "tensor-vm-service-init-recovery-{}",
        std::process::id()
    ));
    let data_dir_text = data_dir.to_string_lossy().into_owned();
    let store = NodeStore::open(data_dir.clone());
    let mut chain = Chain::new(hash_bytes(b"test", &[b"service-init-recovery"]));
    let miner = address(b"service-init-recovery-miner");
    chain
        .register_miner(miner, chain.params().miner_min_stake)
        .unwrap();
    chain
        .register_validator(miner, chain.params().validator_min_stake)
        .unwrap();
    chain.produce_block(miner, 1_000).unwrap();
    chain.produce_block(miner, 1_006).unwrap();
    store.persist_chain(&chain).unwrap();

    let mut stale_snapshot = ChainSnapshot::from_chain(&chain);
    stale_snapshot.block_count = stale_snapshot.block_count.saturating_sub(1);
    store.snapshot_store().save(&stale_snapshot).unwrap();

    let report = init_service_store(&data_dir_text).unwrap();
    assert!(report.contains("command=service_init"));
    assert!(report.contains("existing_store=true"));
    assert!(report.contains("recovered_store=true"));
    assert!(report.contains("recovery_source=chain_state"));
    assert!(report.contains("block_count=2"));
    assert_eq!(store.load_chain().unwrap(), chain);

    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn role_runtime_read_only_rpc_does_not_persist_chain() {
    let data_dir = unique_temp_data_dir("role-runtime-read-only-rpc");
    let _ = std::fs::remove_dir_all(&data_dir);
    let config = test_service_runtime_config(&data_dir, "secret");
    let chain = config
        .node
        .build_chain(hash_bytes(b"test", &[b"read-only-rpc-no-persist"]));
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&chain).unwrap();
    let snapshot_modified = file_modified_at(store.snapshot_store().path());
    let chain_state_modified = file_modified_at(store.chain_state_store().path());
    thread::sleep(Duration::from_millis(1_100));

    let mut runtime = RoleRuntimeLoop::start(config).unwrap();
    let addr = runtime.server.local_addr().unwrap();
    let client = thread::spawn(move || {
        send_http_request(
            addr,
            "GET /chain/head HTTP/1.1\r\nhost: localhost\r\nx-tensorchain-auth: secret\r\n\r\n",
        )
    });

    runtime.serve_rpc_once().unwrap();
    let response = client.join().unwrap();

    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert_eq!(
        file_modified_at(store.snapshot_store().path()),
        snapshot_modified
    );
    assert_eq!(
        file_modified_at(store.chain_state_store().path()),
        chain_state_modified
    );
    assert_eq!(store.load_chain().unwrap(), chain);
    let status = std::fs::read_to_string(data_dir.join("role-runtime.status")).unwrap();
    assert!(status.contains("role_served_requests=1"));

    drop(runtime);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn role_runtime_mutating_rpc_persists_chain() {
    let data_dir = unique_temp_data_dir("role-runtime-mutating-rpc");
    let _ = std::fs::remove_dir_all(&data_dir);
    let config = test_service_runtime_config(&data_dir, "secret");
    let chain = config
        .node
        .build_chain(hash_bytes(b"test", &[b"mutating-rpc-persist"]));
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&chain).unwrap();
    let user = address(b"runtime-faucet-persist-user");

    let mut runtime = RoleRuntimeLoop::start(config).unwrap();
    let addr = runtime.server.local_addr().unwrap();
    let request = format!(
        "POST /faucet/claim/{} HTTP/1.1\r\nhost: localhost\r\nx-tensorchain-auth: secret\r\ncontent-length: 0\r\n\r\n",
        hex(&user)
    );
    let client = thread::spawn(move || send_http_request(addr, &request));

    runtime.serve_rpc_once().unwrap();
    let response = client.join().unwrap();

    assert!(response.starts_with("HTTP/1.1 200 OK"));
    let persisted = store.load_chain().unwrap();
    assert_eq!(persisted.state().rewards().balance(&user), 100);
    let status = std::fs::read_to_string(data_dir.join("role-runtime.status")).unwrap();
    assert!(status.contains("role_served_requests=1"));

    drop(runtime);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn validator_remote_tensor_fetch_status_does_not_persist_chain() {
    let data_dir = unique_temp_data_dir("validator-fetch-no-persist");
    let _ = std::fs::remove_dir_all(&data_dir);
    let data_dir_text = data_dir.to_string_lossy().into_owned();
    let validator = address(b"validator-fetch-no-persist-validator");
    let mut chain = Chain::with_params(
        ChainParams {
            freivalds: FreivaldsParams {
                validators_per_job: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        },
        hash_bytes(b"test", &[b"validator-fetch-no-persist"]),
    );
    let miner = address(b"validator-fetch-no-persist-miner");
    chain
        .register_miner(miner, chain.params().miner_min_stake)
        .unwrap();
    chain
        .register_validator(validator, chain.params().validator_min_stake)
        .unwrap();
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_state = tensor_vm::JobState::TensorOp(job);
    chain
        .apply_command(ChainCommand::SubmitJob(job_state.clone()))
        .unwrap();
    let bundle = CpuReferenceMinerRole::new(miner)
        .execute_job(&job_state, chain.state().height(), 1)
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt))
        .unwrap();
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&chain).unwrap();
    let snapshot_modified = file_modified_at(store.snapshot_store().path());
    let chain_state_modified = file_modified_at(store.chain_state_store().path());
    thread::sleep(Duration::from_millis(1_100));
    let config = ServiceRuntimeConfig {
        runtime_command: "validator_run",
        role: RuntimeRole::Validator,
        role_wallet_address: Some(validator),
        node: runtime_node_config(
            &data_dir_text,
            RuntimeRole::Validator,
            "127.0.0.1:0",
            "/ip4/127.0.0.1/tcp/0",
            Some(hash_bytes(b"test", &[data_dir_text.as_bytes()])),
            "secret",
            0,
        )
        .unwrap(),
    };
    let mut runtime = RoleRuntimeLoop::start(config).unwrap();

    runtime.tick_validator_role_work_once().unwrap();

    assert_eq!(
        file_modified_at(store.snapshot_store().path()),
        snapshot_modified
    );
    assert_eq!(
        file_modified_at(store.chain_state_store().path()),
        chain_state_modified
    );
    assert_eq!(store.load_chain().unwrap(), chain);
    let status = std::fs::read_to_string(data_dir.join("role-runtime.status")).unwrap();
    assert!(status.contains("validator_remote_tensor_fetch_failures=3"));
    assert!(status.contains("validator_attestations_submitted=0"));

    drop(runtime);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn network_event_ingest_accumulates_runtime_counters() {
    let mut cumulative = NetworkEventIngest {
        events: 2,
        block_announcements: 1,
        block_headers: 1,
        block_payloads: 1,
        block_payloads_applied: 1,
        block_votes: 1,
        block_votes_applied: 1,
        jobs: 1,
        job_payloads: 1,
        job_payloads_applied: 1,
        receipts: 0,
        receipt_payloads: 0,
        receipt_payloads_applied: 0,
        attestations: 0,
        attestation_payloads: 0,
        attestation_payloads_applied: 0,
        peers: 0,
        invalid_events: 0,
        applied_blocks: 1,
    };
    cumulative.accumulate(NetworkEventIngest {
        events: 4,
        block_announcements: 1,
        block_headers: 0,
        block_payloads: 2,
        block_payloads_applied: 2,
        block_votes: 2,
        block_votes_applied: 2,
        jobs: 0,
        job_payloads: 2,
        job_payloads_applied: 2,
        receipts: 1,
        receipt_payloads: 1,
        receipt_payloads_applied: 1,
        attestations: 1,
        attestation_payloads: 1,
        attestation_payloads_applied: 1,
        peers: 1,
        invalid_events: 1,
        applied_blocks: 2,
    });

    assert!(cumulative.has_activity());
    assert_eq!(cumulative.events, 6);
    assert_eq!(cumulative.block_announcements, 2);
    assert_eq!(cumulative.block_headers, 1);
    assert_eq!(cumulative.block_payloads, 3);
    assert_eq!(cumulative.block_payloads_applied, 3);
    assert_eq!(cumulative.block_votes, 3);
    assert_eq!(cumulative.block_votes_applied, 3);
    assert_eq!(cumulative.jobs, 1);
    assert_eq!(cumulative.job_payloads, 3);
    assert_eq!(cumulative.job_payloads_applied, 3);
    assert_eq!(cumulative.receipts, 1);
    assert_eq!(cumulative.receipt_payloads, 1);
    assert_eq!(cumulative.receipt_payloads_applied, 1);
    assert_eq!(cumulative.attestations, 1);
    assert_eq!(cumulative.attestation_payloads, 1);
    assert_eq!(cumulative.attestation_payloads_applied, 1);
    assert_eq!(cumulative.peers, 1);
    assert_eq!(cumulative.invalid_events, 1);
    assert_eq!(cumulative.applied_blocks, 3);
}

#[test]
fn service_runtime_state_owns_loop_counters_and_pending_payloads() {
    let mut state = NodeRuntimeState::default();
    state.record_served_request();
    state.record_produced_block();
    state.record_network_ingest(NetworkEventIngest {
        events: 1,
        receipt_payloads: 1,
        receipt_payloads_applied: 1,
        applied_blocks: 2,
        ..NetworkEventIngest::default()
    });

    assert_eq!(state.served_requests(), 1);
    assert_eq!(state.produced_blocks(), 1);
    assert_eq!(state.network_applied_blocks(), 2);
    assert_eq!(state.network_events().events, 1);
    assert_eq!(state.network_events().receipt_payloads, 1);
    assert_eq!(state.network_events().receipt_payloads_applied, 1);
    assert!(state.pending_payloads().is_empty());
    state.record_validator_block_vote_submission(1);
    assert_eq!(state.validator_block_votes_submitted(), 1);
}

#[test]
fn runtime_role_policy_allows_only_validator_local_production() {
    let profile = ChainProfile::local_cpu();
    assert!(
        !NodeConfig::new(profile.clone(), RuntimeRole::Service.node_role(), "service")
            .can_produce_local_blocks()
    );
    assert!(
        !NodeConfig::new(
            profile.clone(),
            RuntimeRole::Proposer.node_role(),
            "proposer"
        )
        .can_produce_local_blocks()
    );
    assert!(
        !NodeConfig::new(profile.clone(), RuntimeRole::Miner.node_role(), "miner")
            .can_produce_local_blocks()
    );
    assert!(
        NodeConfig::new(profile, RuntimeRole::Validator.node_role(), "validator")
            .can_produce_local_blocks()
    );

    assert_eq!(RuntimeRole::Service.label(), "service");
    assert_eq!(RuntimeRole::Miner.label(), "miner");
    assert_eq!(RuntimeRole::Validator.label(), "validator");
    assert_eq!(RuntimeRole::Proposer.label(), "proposer");
}

#[test]
fn role_loop_configs_bind_expected_runtime_roles_and_wallets() {
    let cases = [
        (
            RoleRunLoop::miner(),
            "miner_run",
            RuntimeRole::Miner,
            "miner",
        ),
        (
            RoleRunLoop::validator(),
            "validator_run",
            RuntimeRole::Validator,
            "validator",
        ),
        (
            RoleRunLoop::proposer(),
            "proposer_run",
            RuntimeRole::Proposer,
            "proposer",
        ),
    ];

    for (loop_config, runtime_command, role, wallet) in cases {
        let service_config = loop_config
            .service_runtime_config(RoleServiceConfig {
                wallet,
                device: Some("cpu"),
                node: "/ip4/127.0.0.1/tcp/4001",
                listen: "127.0.0.1:0",
                p2p_listen: "/ip4/127.0.0.1/tcp/0",
                data_dir: "role-loop-config-test",
                identity_seed: None,
                auth_token: "token",
                max_requests: 1,
            })
            .unwrap();

        assert_eq!(service_config.runtime_command, runtime_command);
        assert_eq!(service_config.role, role);
        assert_eq!(service_config.node.role, role.node_role());
        assert_eq!(
            service_config.node.can_produce_local_blocks(),
            matches!(role, RuntimeRole::Validator)
        );
        assert!(!service_config.node.local_synthetic_producer());
        assert_eq!(
            service_config.role_wallet_address,
            Some(address(wallet.as_bytes()))
        );
    }
}

#[test]
fn role_loop_reports_keep_role_specific_readiness_lines() {
    let config = RoleServiceConfig {
        wallet: "testnet-miner-0",
        device: Some("cpu"),
        node: "/ip4/127.0.0.1/tcp/4001",
        listen: "127.0.0.1:0",
        p2p_listen: "/ip4/127.0.0.1/tcp/0",
        data_dir: "role-loop-report-test",
        identity_seed: None,
        auth_token: "token",
        max_requests: 1,
    };

    let miner_report = RoleRunLoop::miner().format_report(config, "service_report=true");
    assert!(miner_report.contains("command=miner_run"));
    assert!(miner_report.contains("role=miner"));
    assert!(miner_report.contains("device=cpu"));
    assert!(miner_report.contains("role_runtime_ready=true"));

    let validator_report = RoleRunLoop::validator().format_report(config, "service_report=true");
    assert!(validator_report.contains("command=validator_run"));
    assert!(validator_report.contains("role=validator"));
    assert!(validator_report.contains("reference_verifier_ready=true"));
    assert!(validator_report.contains("role_runtime_ready=true"));

    let proposer_report = RoleRunLoop::proposer().format_report(config, "service_report=true");
    assert!(proposer_report.contains("command=proposer_run"));
    assert!(proposer_report.contains("role=proposer"));
    assert!(proposer_report.contains("proposer_ready=true"));
    assert!(proposer_report.contains("role_runtime_ready=true"));
}

#[test]
fn role_wallet_registration_matches_loaded_chain_role() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"role-wallet-registration"]));
    let miner = address(b"runtime-wallet-miner");
    let validator = address(b"runtime-wallet-validator");
    let unknown = address(b"runtime-wallet-unknown");
    chain
        .register_miner(miner, chain.params().miner_min_stake)
        .unwrap();
    chain
        .register_validator(validator, chain.params().validator_min_stake)
        .unwrap();

    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Miner, Some(miner), &chain),
        "miner"
    );
    assert!(runtime_role_wallet_registered(
        RuntimeRole::Miner,
        Some(miner),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Validator, Some(validator), &chain),
        "validator"
    );
    assert!(runtime_role_wallet_registered(
        RuntimeRole::Validator,
        Some(validator),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Proposer, Some(miner), &chain),
        "unregistered"
    );
    assert!(!runtime_role_wallet_registered(
        RuntimeRole::Proposer,
        Some(miner),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Proposer, Some(validator), &chain),
        "validator"
    );
    assert!(runtime_role_wallet_registered(
        RuntimeRole::Proposer,
        Some(validator),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Miner, Some(validator), &chain),
        "unregistered"
    );
    assert!(!runtime_role_wallet_registered(
        RuntimeRole::Miner,
        Some(validator),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Validator, Some(miner), &chain),
        "unregistered"
    );
    assert!(!runtime_role_wallet_registered(
        RuntimeRole::Validator,
        Some(miner),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Proposer, Some(unknown), &chain),
        "unregistered"
    );
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Service, None, &chain),
        "none"
    );
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Service, Some(miner), &chain),
        "none"
    );
    assert!(!runtime_role_wallet_registered(
        RuntimeRole::Service,
        None,
        &chain
    ));
}

#[test]
fn miner_role_work_observation_tracks_assigned_unreceipted_jobs() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"miner-work-observation"]));
    let miner = address(b"miner-work-observation-miner");
    chain
        .register_miner(miner, chain.params().miner_min_stake)
        .unwrap();
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_id = job.job_id;
    let job_state = tensor_vm::JobState::TensorOp(job);
    chain
        .apply_command(ChainCommand::SubmitJob(job_state.clone()))
        .unwrap();

    let observation = miner_role_work_observation(&chain, miner);
    assert_eq!(observation.assigned_jobs, BTreeSet::from([job_id]));
    assert_eq!(observation.unreceipted_jobs, BTreeSet::from([job_id]));

    let bundle = tensor_vm::roles::CpuReferenceMinerRole::new(miner)
        .execute_job(&job_state, chain.state().height(), 1)
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt))
        .unwrap();

    let observation = miner_role_work_observation(&chain, miner);
    assert_eq!(observation.assigned_jobs, BTreeSet::from([job_id]));
    assert!(observation.unreceipted_jobs.is_empty());
}

#[test]
fn miner_role_work_observation_ignores_unassigned_miners() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"miner-work-unassigned"]));
    let miner = address(b"miner-work-assigned");
    let unassigned = address(b"miner-work-unassigned");
    chain
        .register_miner(miner, chain.params().miner_min_stake)
        .unwrap();
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    chain
        .apply_command(ChainCommand::SubmitJob(tensor_vm::JobState::TensorOp(job)))
        .unwrap();

    assert_eq!(
        miner_role_work_observation(&chain, unassigned),
        MinerRoleWorkObservation::default()
    );
}

#[test]
fn miner_role_submits_assigned_unreceipted_tensor_op_once() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"miner-receipt-submit"]));
    let miner = address(b"miner-receipt-submit-miner");
    chain
        .register_miner(miner, chain.params().miner_min_stake)
        .unwrap();
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_id = job.job_id;
    chain
        .apply_command(ChainCommand::SubmitJob(tensor_vm::JobState::TensorOp(job)))
        .unwrap();
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    let submission = submit_miner_role_receipt(&mut node, miner, job_id)
        .unwrap()
        .expect("assigned unreceipted job should submit a receipt");

    assert_eq!(submission.receipts_submitted, 1);
    assert_eq!(submission.tensors_inserted, 3);
    assert_eq!(node.chain.state().receipts().len(), 1);
    let receipt = node
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("receipt should be stored");
    assert_eq!(receipt.job_id(), job_id);
    assert_eq!(receipt.miner(), miner);
    assert_tensor_count(&node, 3);
    let observation = miner_role_work_observation(&node.chain, miner);
    assert_eq!(observation.assigned_jobs, BTreeSet::from([job_id]));
    assert!(observation.unreceipted_jobs.is_empty());
}

#[test]
fn miner_role_receipt_submission_skips_duplicate_unregistered_and_unassigned_work() {
    let params = ChainParams {
        replication_factor: 1,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"miner-receipt-skip"]));
    let miner_a = address(b"miner-receipt-skip-a");
    let miner_b = address(b"miner-receipt-skip-b");
    let unknown = address(b"miner-receipt-skip-unknown");
    chain
        .register_miner(miner_a, chain.params().miner_min_stake)
        .unwrap();
    chain
        .register_miner(miner_b, chain.params().miner_min_stake)
        .unwrap();
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_id = job.job_id;
    chain
        .apply_command(ChainCommand::SubmitJob(tensor_vm::JobState::TensorOp(job)))
        .unwrap();
    let assignment = JobScheduler::with_small_shape((8, 8, 8)).assign_miners(
        &chain,
        job_id,
        &chain.state().finalized_randomness(),
    );
    let assigned = assignment.miners[0];
    let unassigned = [miner_a, miner_b]
        .into_iter()
        .find(|miner| *miner != assigned)
        .expect("replication factor one should leave one registered miner unassigned");
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    assert!(
        submit_miner_role_receipt(&mut node, unknown, job_id)
            .unwrap()
            .is_none()
    );
    assert!(
        submit_miner_role_receipt(&mut node, unassigned, job_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(node.chain.state().receipts().len(), 0);

    assert!(
        submit_miner_role_receipt(&mut node, assigned, job_id)
            .unwrap()
            .is_some()
    );
    assert_eq!(node.chain.state().receipts().len(), 1);
    assert_tensor_count(&node, 3);
    assert!(
        submit_miner_role_receipt(&mut node, assigned, job_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(node.chain.state().receipts().len(), 1);
    assert_tensor_count(&node, 3);
}

#[test]
fn validator_role_work_observation_tracks_assigned_unattested_receipts() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"validator-work-observation"]));
    let miner = address(b"validator-work-miner");
    let validator = address(b"validator-work-validator");
    chain
        .register_miner(miner, chain.params().miner_min_stake)
        .unwrap();
    chain
        .register_validator(validator, chain.params().validator_min_stake)
        .unwrap();
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_state = tensor_vm::JobState::TensorOp(job);
    chain
        .apply_command(ChainCommand::SubmitJob(job_state.clone()))
        .unwrap();
    let bundle = CpuReferenceMinerRole::new(miner)
        .execute_job(&job_state, chain.state().height(), 1)
        .unwrap();
    let receipt_id = bundle.receipt_id();
    chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt.clone()))
        .unwrap();
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    let observation = validator_role_work_observation(&node, validator);
    assert_eq!(observation.assigned_receipts, BTreeSet::from([receipt_id]));
    assert_eq!(
        observation.unattested_receipts,
        BTreeSet::from([receipt_id])
    );
    assert!(observation.artifact_ready_receipts.is_empty());
    assert_eq!(
        observation.artifact_missing_receipts,
        BTreeSet::from([receipt_id])
    );

    insert_bundle_tensors(&mut node, &bundle);
    let observation = validator_role_work_observation(&node, validator);
    assert_eq!(observation.assigned_receipts, BTreeSet::from([receipt_id]));
    assert_eq!(
        observation.unattested_receipts,
        BTreeSet::from([receipt_id])
    );
    assert_eq!(
        observation.artifact_ready_receipts,
        BTreeSet::from([receipt_id])
    );
    assert!(observation.artifact_missing_receipts.is_empty());
}

#[test]
fn validator_role_attestation_submission_skips_missing_unregistered_unassigned_and_duplicates() {
    let params = ChainParams {
        freivalds: FreivaldsParams {
            validators_per_job: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(
        params,
        hash_bytes(b"test", &[b"validator-attestation-skip"]),
    );
    let miner = address(b"validator-attestation-miner");
    let validator_a = address(b"validator-attestation-a");
    let validator_b = address(b"validator-attestation-b");
    let unknown = address(b"validator-attestation-unknown");
    chain
        .register_miner(miner, chain.params().miner_min_stake)
        .unwrap();
    chain
        .register_validator(validator_a, chain.params().validator_min_stake)
        .unwrap();
    chain
        .register_validator(validator_b, chain.params().validator_min_stake)
        .unwrap();
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_state = tensor_vm::JobState::TensorOp(job);
    chain
        .apply_command(ChainCommand::SubmitJob(job_state.clone()))
        .unwrap();
    let bundle = CpuReferenceMinerRole::new(miner)
        .execute_job(&job_state, chain.state().height(), 1)
        .unwrap();
    let receipt_id = bundle.receipt_id();
    chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt.clone()))
        .unwrap();
    let assignment = JobScheduler::with_small_shape((8, 8, 8)).assign_validators(
        &chain,
        receipt_id,
        &chain.state().finalized_randomness(),
    );
    let assigned = assignment.validators[0];
    let unassigned = [validator_a, validator_b]
        .into_iter()
        .find(|validator| *validator != assigned)
        .expect("one-validator assignment should leave one validator unassigned");
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    assert!(
        submit_validator_role_attestation(&mut node, unknown, receipt_id)
            .unwrap()
            .is_none()
    );
    assert!(
        submit_validator_role_attestation(&mut node, unassigned, receipt_id)
            .unwrap()
            .is_none()
    );
    assert!(
        submit_validator_role_attestation(&mut node, assigned, receipt_id)
            .unwrap()
            .is_none()
    );
    assert!(!node.chain.state().attestations().contains_key(&receipt_id));

    insert_bundle_tensors(&mut node, &bundle);
    let submission = submit_validator_role_attestation(&mut node, assigned, receipt_id)
        .unwrap()
        .expect("assigned validator with local tensors should submit attestation");
    assert_eq!(submission.attestations_submitted, 1);
    let attestations = node
        .chain
        .state()
        .attestations()
        .get(&receipt_id)
        .expect("attestation should be stored");
    assert_eq!(attestations.len(), 1);
    assert_eq!(attestations[0].validator, assigned);
    assert_eq!(attestations[0].result, VerificationResult::Valid);
    assert!(
        submit_validator_role_attestation(&mut node, assigned, receipt_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(node.chain.state().attestations()[&receipt_id].len(), 1);
    let observation = validator_role_work_observation(&node, assigned);
    assert_eq!(observation.assigned_receipts, BTreeSet::from([receipt_id]));
    assert!(observation.unattested_receipts.is_empty());
    assert!(observation.artifact_ready_receipts.is_empty());
    assert!(observation.artifact_missing_receipts.is_empty());
}

#[test]
fn validator_role_block_vote_submission_finalizes_only_through_votes() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"validator-block-vote"]));
    let validators = [
        address(b"validator-block-vote-a"),
        address(b"validator-block-vote-b"),
        address(b"validator-block-vote-c"),
    ];
    for validator in validators {
        chain
            .register_validator(validator, chain.params().validator_min_stake)
            .unwrap();
    }
    let block = chain.produce_block(validators[0], 1_000).unwrap();
    let block_hash = block.hash();
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    assert!(!node.chain.is_block_finalized(&block_hash));
    assert!(!node.chain.state().block_votes().contains_key(&block_hash));
    assert!(
        submit_validator_role_block_vote(&mut node, address(b"unknown-block-voter"))
            .unwrap()
            .is_none()
    );

    let first = submit_validator_role_block_vote(&mut node, validators[0])
        .unwrap()
        .expect("registered validator should vote on an unfinalized block");
    assert_eq!(first.block_votes_submitted, 1);
    assert!(!node.chain.is_block_finalized(&block_hash));
    assert_eq!(node.chain.state().block_votes()[&block_hash].len(), 1);
    assert!(
        submit_validator_role_block_vote(&mut node, validators[0])
            .unwrap()
            .is_none()
    );

    let second = submit_validator_role_block_vote(&mut node, validators[1])
        .unwrap()
        .expect("second validator should reach the finality threshold");
    assert_eq!(second.block_votes_submitted, 1);
    assert!(node.chain.is_block_finalized(&block_hash));
    assert!(
        submit_validator_role_block_vote(&mut node, validators[2])
            .unwrap()
            .is_none()
    );
}

#[test]
fn validator_remote_tensor_response_rejects_corrupt_or_mismatched_payloads() {
    let tensor =
        Tensor::from_vec(vec![2, 2], tensor_vm::DType::FieldElement, vec![1, 3, 5, 7]).unwrap();
    let requested_root = tensor.commitment_root();
    let payload = tensor_vm::encode_tensor_payload(&tensor);
    assert_eq!(
        validator_remote_tensor_response(
            requested_root,
            P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: requested_root,
                payload: Some(payload.clone()),
            },
        ),
        ValidatorRemoteTensorResponse::Found {
            tensor: tensor.clone(),
            bytes: payload.len(),
        }
    );
    assert_eq!(
        validator_remote_tensor_response(
            requested_root,
            P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: requested_root,
                payload: None,
            },
        ),
        ValidatorRemoteTensorResponse::Missing
    );
    assert_eq!(
        validator_remote_tensor_response(
            requested_root,
            P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: hash_bytes(b"test", &[b"wrong-response-root"]),
                payload: Some(payload.clone()),
            },
        ),
        ValidatorRemoteTensorResponse::Invalid
    );
    assert_eq!(
        validator_remote_tensor_response(
            requested_root,
            P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: requested_root,
                payload: Some(vec![255, 0, 1]),
            },
        ),
        ValidatorRemoteTensorResponse::Invalid
    );
    let other_tensor =
        Tensor::from_vec(vec![2, 2], tensor_vm::DType::FieldElement, vec![2, 3, 5, 7]).unwrap();
    assert_eq!(
        validator_remote_tensor_response(
            requested_root,
            P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: requested_root,
                payload: Some(tensor_vm::encode_tensor_payload(&other_tensor)),
            },
        ),
        ValidatorRemoteTensorResponse::Invalid
    );
}

#[test]
fn validator_role_fetches_remote_tensors_before_attesting() {
    let params = ChainParams {
        freivalds: FreivaldsParams {
            validators_per_job: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"validator-remote-fetch"]));
    let miner = address(b"validator-remote-fetch-miner");
    let validator = address(b"validator-remote-fetch-validator");
    chain
        .register_miner(miner, chain.params().miner_min_stake)
        .unwrap();
    chain
        .register_validator(validator, chain.params().validator_min_stake)
        .unwrap();
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_state = tensor_vm::JobState::TensorOp(job);
    chain
        .apply_command(ChainCommand::SubmitJob(job_state.clone()))
        .unwrap();
    let bundle = CpuReferenceMinerRole::new(miner)
        .execute_job(&job_state, chain.state().height(), 1)
        .unwrap();
    let receipt_id = bundle.receipt_id();
    chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt.clone()))
        .unwrap();
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    let port = free_tcp_port();
    let provider = spawn_libp2p_service(Libp2pControlPlaneConfig {
        listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{port}")],
        identity_seed: Some(hash_bytes(b"test", &[b"validator-remote-fetch-provider"])),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();
    for tensor in bundle.served_tensors() {
        provider.register_tensor(tensor);
    }
    let requester = spawn_libp2p_service(Libp2pControlPlaneConfig {
        listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
        bootstrap_addresses: vec![format!(
            "/ip4/127.0.0.1/tcp/{port}/p2p/{}",
            provider.peer_id()
        )],
        identity_seed: Some(hash_bytes(b"test", &[b"validator-remote-fetch-requester"])),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();
    wait_for_connected_role_services(&provider, &requester);

    let observation = validator_role_work_observation(&node, validator);
    assert_eq!(
        observation.artifact_missing_receipts,
        BTreeSet::from([receipt_id])
    );
    let report = fetch_validator_role_missing_tensors(&mut node, &requester, receipt_id).unwrap();
    assert_eq!(report.successes, 3);
    assert_eq!(report.failures, 0);
    assert_eq!(report.tensors_inserted, 3);
    assert!(report.attempts >= 3);
    assert!(report.bytes > 0);
    assert_tensor_count(&node, 3);

    let observation = validator_role_work_observation(&node, validator);
    assert_eq!(
        observation.artifact_ready_receipts,
        BTreeSet::from([receipt_id])
    );
    assert!(observation.artifact_missing_receipts.is_empty());
    let submission = submit_validator_role_attestation(&mut node, validator, receipt_id)
        .unwrap()
        .expect("remote-fetched tensors should allow attestation");
    assert_eq!(submission.attestations_submitted, 1);
    assert_eq!(
        node.chain.state().attestations()[&receipt_id][0].result,
        VerificationResult::Valid
    );
}

#[test]
fn chain_profile_labels_drive_runtime_synthetic_jobs() {
    let local = chain_profile_from_label("local_cpu").unwrap();
    let testnet = chain_profile_from_label("public_testnet").unwrap();
    let mainnet = chain_profile_from_label("mainnet").unwrap();

    assert_eq!(local.label(), "local_cpu");
    assert_eq!(testnet.label(), "public_testnet");
    assert_eq!(mainnet.label(), "mainnet");
    assert!(local.synthetic_job_source().is_some());
    assert!(testnet.synthetic_job_source().is_none());
    assert!(mainnet.synthetic_job_source().is_none());
    assert!(chain_profile_from_label("staging").is_err());
}

fn test_rpc_server(chain: Chain) -> RpcHttpServer {
    let node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    let gateway = RpcGateway::new(node, RpcPolicy::default());
    RpcHttpServer::bind("127.0.0.1:0", gateway).unwrap()
}

fn assert_tensor_count(node: &RpcNode, expected: usize) {
    let response = node.handle(&tensor_vm::RpcRequest {
        method: "GET".to_owned(),
        path: "/tensor/latest".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(response.status, 200);
    assert!(
        response
            .body
            .contains(&format!("\"tensor_count\":{expected}")),
        "unexpected tensor latest response: {}",
        response.body
    );
}

fn insert_bundle_tensors(node: &mut RpcNode, bundle: &RoleReceiptBundle) {
    for tensor in bundle.served_tensors() {
        node.insert_tensor(tensor);
    }
}

fn unique_temp_data_dir(name: &str) -> std::path::PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("tensor-vm-{name}-{}-{now}", std::process::id()))
}

fn test_service_runtime_config(data_dir: &Path, auth_token: &str) -> ServiceRuntimeConfig {
    let data_dir_text = data_dir.to_string_lossy().into_owned();
    ServiceRuntimeConfig {
        runtime_command: "service_serve",
        role: RuntimeRole::Service,
        role_wallet_address: None,
        node: runtime_node_config(
            &data_dir_text,
            RuntimeRole::Service,
            "127.0.0.1:0",
            "/ip4/127.0.0.1/tcp/0",
            Some(hash_bytes(b"test", &[data_dir_text.as_bytes()])),
            auth_token,
            0,
        )
        .unwrap(),
    }
}

fn file_modified_at(path: &Path) -> std::time::SystemTime {
    std::fs::metadata(path).unwrap().modified().unwrap()
}

fn send_http_request(addr: std::net::SocketAddr, request: &str) -> String {
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpStream};

    let mut client = TcpStream::connect(addr).unwrap();
    client.write_all(request.as_bytes()).unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    response
}

fn free_tcp_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn wait_for_connected_role_services(
    service_a: &TensorVmLibp2pService,
    service_b: &TensorVmLibp2pService,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline
        && (service_a.connected_peer_count() == 0 || service_b.connected_peer_count() == 0)
    {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(service_a.connected_peer_count(), 1);
    assert_eq!(service_b.connected_peer_count(), 1);
}

fn chain_with_network_participants(
    receipt: &ReceiptState,
    attestation: &ValidatorAttestation,
) -> Chain {
    let mut chain = Chain::new(local_cpu_seed_beacon());
    chain
        .register_miner(receipt.miner(), chain.params().miner_min_stake)
        .unwrap();
    chain
        .register_validator(attestation.validator, chain.params().validator_min_stake)
        .unwrap();
    chain
}

fn chain_with_network_job(
    job: tensor_vm::JobState,
    receipt: &ReceiptState,
    attestation: &ValidatorAttestation,
) -> Chain {
    let mut chain = chain_with_network_participants(receipt, attestation);
    chain.apply_command(ChainCommand::SubmitJob(job)).unwrap();
    chain
}

fn chain_with_network_receipt(
    job: tensor_vm::JobState,
    receipt: ReceiptState,
    attestation: &ValidatorAttestation,
) -> Chain {
    let mut chain = chain_with_network_job(job, &receipt, attestation);
    chain
        .apply_command(ChainCommand::SubmitReceipt(receipt))
        .unwrap();
    chain
}

#[test]
fn network_payload_application_defers_out_of_order_receipts_and_attestations() {
    let mut testnet = LocalTestnet::new(TestnetConfig::default(), local_cpu_seed_beacon());
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    testnet.run_matmul_round(&scheduler);
    let job = testnet
        .chain
        .state()
        .jobs()
        .values()
        .next()
        .expect("local round must produce a job")
        .clone();
    let receipt = testnet
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("local round must produce a receipt")
        .clone();
    let receipt_id = receipt.receipt_id();
    let attestation = testnet
        .chain
        .state()
        .attestations()
        .values()
        .flat_map(|items| items.iter())
        .next()
        .expect("local round must produce an attestation")
        .clone();
    let attestation_id = attestation_announcement_hash(&attestation);

    let missing_job_chain = chain_with_network_participants(&receipt, &attestation);
    let mut missing_job_server = test_rpc_server(missing_job_chain);
    assert_eq!(
        apply_network_receipt_payload(
            &mut missing_job_server.gateway_mut().node.chain,
            receipt_id,
            &encode_receipt_payload(&receipt),
        ),
        NetworkPayloadApply::Pending
    );

    let receipt_chain = chain_with_network_job(job.clone(), &receipt, &attestation);
    let mut receipt_server = test_rpc_server(receipt_chain);
    assert_eq!(
        apply_network_receipt_payload(
            &mut receipt_server.gateway_mut().node.chain,
            receipt_id,
            &encode_receipt_payload(&receipt),
        ),
        NetworkPayloadApply::Applied
    );

    let missing_receipt_chain = chain_with_network_job(job.clone(), &receipt, &attestation);
    let mut missing_receipt_server = test_rpc_server(missing_receipt_chain);
    assert_eq!(
        apply_network_attestation_payload(
            &mut missing_receipt_server.gateway_mut().node.chain,
            attestation_id,
            &encode_attestation_payload(&attestation),
        ),
        NetworkPayloadApply::Pending
    );

    let attestation_chain = chain_with_network_receipt(job, receipt.clone(), &attestation);
    let mut attestation_server = test_rpc_server(attestation_chain);
    assert_eq!(
        apply_network_attestation_payload(
            &mut attestation_server.gateway_mut().node.chain,
            attestation_id,
            &encode_attestation_payload(&attestation),
        ),
        NetworkPayloadApply::Applied
    );
}

#[test]
fn pending_network_payloads_retry_after_dependencies_arrive() {
    let mut testnet = LocalTestnet::new(TestnetConfig::default(), local_cpu_seed_beacon());
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    testnet.run_matmul_round(&scheduler);
    let job = testnet
        .chain
        .state()
        .jobs()
        .values()
        .next()
        .expect("local round must produce a job")
        .clone();
    let job_id = job.job_id();
    let receipt = testnet
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("local round must produce a receipt")
        .clone();
    let receipt_id = receipt.receipt_id();
    let attestation = testnet
        .chain
        .state()
        .attestations()
        .values()
        .flat_map(|items| items.iter())
        .next()
        .expect("local round must produce an attestation")
        .clone();
    let attestation_id = attestation_announcement_hash(&attestation);

    let out_of_order_chain = chain_with_network_participants(&receipt, &attestation);
    let mut server = test_rpc_server(out_of_order_chain);
    let mut pending = PendingNetworkPayloads::default();

    assert_eq!(
        apply_network_receipt_payload(
            &mut server.gateway_mut().node.chain,
            receipt_id,
            &encode_receipt_payload(&receipt)
        ),
        NetworkPayloadApply::Pending
    );
    pending.queue_receipt(receipt_id, encode_receipt_payload(&receipt));
    assert_eq!(
        apply_network_attestation_payload(
            &mut server.gateway_mut().node.chain,
            attestation_id,
            &encode_attestation_payload(&attestation),
        ),
        NetworkPayloadApply::Pending
    );
    pending.queue_attestation(attestation_id, encode_attestation_payload(&attestation));

    apply_network_job_payload(
        &mut server.gateway_mut().node.chain,
        job_id,
        &encode_job_payload(&job),
    )
    .unwrap();
    let mut processor = ChainNetworkPayloadProcessor::new(&mut server.gateway_mut().node.chain);
    let retried = pending.retry_with(&mut processor);

    assert!(retried.has_activity());
    assert_eq!(retried.receipt_payloads_applied, 1);
    assert_eq!(retried.attestation_payloads_applied, 1);
    assert_eq!(retried.invalid_events, 0);
    assert!(pending.is_empty());
    assert_eq!(
        server
            .gateway()
            .node
            .chain
            .state()
            .receipts()
            .get(&receipt_id),
        Some(&receipt)
    );
    assert_eq!(
        server
            .gateway()
            .node
            .chain
            .state()
            .attestations()
            .get(&receipt_id)
            .and_then(|items| items.first()),
        Some(&attestation)
    );
}

#[test]
fn network_ingest_orders_payload_dependencies_before_blocks() {
    let block_hash = hash_bytes(b"test", &[b"announced-block"]);
    let job_id = hash_bytes(b"test", &[b"announced-job"]);
    let receipt_id = hash_bytes(b"test", &[b"announced-receipt"]);
    let messages = network_ingest_order(vec![
        P2pMessage::NewJobPayload {
            job_id,
            payload: vec![1, 2, 3],
        },
        P2pMessage::NewReceipt(receipt_id),
        P2pMessage::NewBlockHeader {
            height: 3,
            block_hash,
        },
        P2pMessage::NewBlockPayload {
            height: 3,
            block_hash,
            payload: vec![4, 5, 6],
        },
        P2pMessage::NewJob(job_id),
        P2pMessage::NewBlock(block_hash),
    ]);

    assert!(matches!(messages[0], P2pMessage::NewJobPayload { .. }));
    assert!(matches!(messages[1], P2pMessage::NewReceipt(_)));
    assert!(matches!(messages[2], P2pMessage::NewJob(_)));
    assert!(matches!(messages[3], P2pMessage::NewBlockPayload { .. }));
    assert!(matches!(messages[4], P2pMessage::NewBlockHeader { .. }));
    assert!(matches!(messages[5], P2pMessage::NewBlock(_)));
}
