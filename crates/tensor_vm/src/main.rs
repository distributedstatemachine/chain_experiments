use std::{
    collections::{BTreeMap, BTreeSet},
    io::ErrorKind,
    path::Path,
    thread,
    time::{Duration, Instant},
};
use tensor_vm::{
    ChainCommand, ChainEngine, ChainProfile, CliCommand, Faucet, JobScheduler,
    Libp2pControlPlaneConfig, LocalChain, NetworkConfig, NetworkEventIngest, NodeConfig, NodeRole,
    NodeRuntimeState, NodeStore, PeerRecord, PendingNetworkPayloads, PrimitiveType, ReceiptState,
    RpcGateway, RpcHttpServer, RpcNode, RpcPolicy, SyntheticLocalJobSource, Tensor,
    TensorVmLibp2pService, VerificationResult,
    api::P2pMessage,
    cli::{
        execute_reference_cli_command, validate_public_evidence_manifest,
        validate_public_testnet_preflight_manifest,
    },
    encode_attestation_payload, encode_job_payload, encode_receipt_payload,
    hash::hex,
    jobs::LinearTrainingStepOutput,
    localnet::produce_synthetic_cpu_round_with_tensors,
    node::{NetworkEventContext, attestation_announcement_hash, ingest_network_messages},
    parse_cli_args,
    roles::{
        CpuReferenceMinerRole, ReferenceValidatorRole, RoleReceiptArtifacts, RoleReceiptBundle,
    },
    spawn_libp2p_service,
    testnet::{LocalTestnet, PublicTestnetCriteria, TestnetConfig},
    types::{Address, Hash, address, hash_bytes},
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_cli_args(&args) {
        Ok(command) => match execute_command(&command) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}

fn execute_command(command: &CliCommand) -> std::result::Result<String, String> {
    match command {
        CliCommand::PublicEvidenceValidate { manifest } => {
            let contents = std::fs::read_to_string(manifest)
                .map_err(|error| format!("failed to read evidence manifest {manifest}: {error}"))?;
            validate_public_evidence_manifest(&contents).map_err(|error| error.to_string())
        }
        CliCommand::PublicTestnetPreflight { manifest } => {
            let contents = std::fs::read_to_string(manifest).map_err(|error| {
                format!("failed to read preflight manifest {manifest}: {error}")
            })?;
            validate_public_testnet_preflight_manifest(&contents).map_err(|error| error.to_string())
        }
        CliCommand::MinerRun {
            wallet,
            device,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            run_miner_service(RoleServiceConfig {
                wallet,
                device: Some(device),
                node,
                listen,
                p2p_listen,
                data_dir,
                identity_seed: *identity_seed,
                auth_token,
                max_requests: *max_requests,
            })
        }
        CliCommand::ValidatorRun {
            wallet,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            run_validator_service(RoleServiceConfig {
                wallet,
                device: None,
                node,
                listen,
                p2p_listen,
                data_dir,
                identity_seed: *identity_seed,
                auth_token,
                max_requests: *max_requests,
            })
        }
        CliCommand::ProposerRun {
            wallet,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            run_proposer_service(RoleServiceConfig {
                wallet,
                device: None,
                node,
                listen,
                p2p_listen,
                data_dir,
                identity_seed: *identity_seed,
                auth_token,
                max_requests: *max_requests,
            })
        }
        CliCommand::ServiceInit { data_dir } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            init_service_store(data_dir)
        }
        CliCommand::ServicePeerAdd {
            data_dir,
            peer_id,
            address,
        } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            add_service_peer(data_dir, peer_id, address)
        }
        CliCommand::ServiceReadiness {
            p2p_listen,
            data_dir,
            identity_seed,
        } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            check_service_readiness(p2p_listen, data_dir, *identity_seed)
        }
        CliCommand::ServiceServe {
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            serve_service(
                listen,
                p2p_listen,
                data_dir,
                *identity_seed,
                auth_token,
                *max_requests,
            )
        }
        CliCommand::ServiceStatus { data_dir } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            service_status(data_dir)
        }
        CliCommand::ServiceBlock { data_dir, height } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            service_block_status(data_dir, *height)
        }
        CliCommand::LocalTestnetSeed { data_dir } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            seed_local_testnet(data_dir)
        }
        _ => execute_reference_cli_command(command).map_err(|error| error.to_string()),
    }
}

fn init_service_store(data_dir: &str) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    if Path::new(data_dir).exists()
        && Path::new(data_dir)
            .read_dir()
            .map_err(|error| format!("failed to inspect data dir {data_dir}: {error}"))?
            .next()
            .is_some()
    {
        match store.load_chain().and_then(|_| store.status()) {
            Ok(status) => {
                return Ok(format!(
                    "command=service_init\ndata_dir={}\nexisting_store=true\nrecovered_store=false\nblock_count={}\nlatest_block_hash={}",
                    status.data_dir.display(),
                    status.block_count,
                    hex(&status.latest_block_hash)
                ));
            }
            Err(error) => {
                let status = store.recover_from_chain_state().map_err(|recovery_error| {
                    format!(
                        "existing node store is invalid: {error}; chain-state recovery failed: {recovery_error}"
                    )
                })?;
                return Ok(format!(
                    "command=service_init\ndata_dir={}\nexisting_store=true\nrecovered_store=true\nrecovery_source=chain_state\nblock_count={}\nlatest_block_hash={}",
                    status.data_dir.display(),
                    status.block_count,
                    hex(&status.latest_block_hash)
                ));
            }
        }
    }

    let chain = LocalChain::new(hash_bytes(
        b"tensor-vm-service-genesis",
        &[data_dir.as_bytes()],
    ));
    let status = store
        .persist_chain(&chain)
        .map_err(|error| format!("failed to initialize node store {data_dir}: {error}"))?;
    Ok(format!(
        "command=service_init\ndata_dir={}\nexisting_store=false\nrecovered_store=false\nblock_count={}\nlatest_block_hash={}",
        status.data_dir.display(),
        status.block_count,
        hex(&status.latest_block_hash)
    ))
}

fn add_service_peer(
    data_dir: &str,
    peer_id: &str,
    address: &str,
) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    let record = PeerRecord::from_strings(peer_id, address)
        .map_err(|error| format!("invalid libp2p bootstrap peer: {error}"))?;
    let bootstrap_address = record
        .bootstrap_multiaddr()
        .map_err(|error| format!("invalid libp2p bootstrap peer: {error}"))?
        .to_string();
    let records = store
        .peer_book_store()
        .upsert_record(record)
        .map_err(|error| format!("failed to update libp2p peer book {data_dir}: {error}"))?;
    Ok(format!(
        "command=service_peer_add\ndata_dir={data_dir}\npeer_id={peer_id}\naddress={address}\nbootstrap_address={bootstrap_address}\nbootstrap_peers={}",
        records.len()
    ))
}

fn check_service_readiness(
    p2p_listen: &str,
    data_dir: &str,
    identity_seed: Option<[u8; 32]>,
) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    store
        .load_chain()
        .map_err(|error| format!("failed to load node store {data_dir}: {error}"))?;
    let bootstrap_addresses = if store.peer_book_store().path().exists() {
        store
            .peer_book_store()
            .load_bootstrap_addresses()
            .map_err(|error| format!("failed to load libp2p peer book {data_dir}: {error}"))?
    } else {
        Vec::new()
    };
    let bootstrap_peer_count = bootstrap_addresses.len();
    let p2p_config = Libp2pControlPlaneConfig {
        listen_addresses: vec![p2p_listen.to_owned()],
        bootstrap_addresses,
        identity_seed,
        ..Libp2pControlPlaneConfig::default()
    };
    let max_transmit_bytes = p2p_config.max_gossipsub_transmit_bytes;
    let request_timeout_seconds = p2p_config.request_timeout_seconds;
    let max_concurrent_streams = p2p_config.max_concurrent_request_streams;
    let idle_timeout_seconds = p2p_config.idle_connection_timeout_seconds;
    let p2p_service = spawn_libp2p_service(p2p_config)
        .map_err(|error| format!("failed to start mandatory libp2p readiness check: {error}"))?;
    let identity = p2p_identity_report(identity_seed);
    Ok(format!(
        "command=service_readiness\np2p_listen={p2p_listen}\np2p_runtime=libp2p\np2p_peer_id={}\np2p_gossipsub_topics={}\np2p_request_response_protocols={}\np2p_bootstrap_peers={bootstrap_peer_count}\n{identity}\np2p_max_transmit_bytes={max_transmit_bytes}\np2p_request_timeout_seconds={request_timeout_seconds}\np2p_max_concurrent_streams={max_concurrent_streams}\np2p_idle_timeout_seconds={idle_timeout_seconds}\ndata_dir={data_dir}\nnode_store_ready=true\nlibp2p_ready=true",
        p2p_service.peer_id(),
        p2p_service.info().subscribed_topics.len(),
        p2p_service.info().request_response_protocols.len()
    ))
}

fn seed_local_testnet(data_dir: &str) -> std::result::Result<String, String> {
    let mut testnet = LocalTestnet::new(TestnetConfig::default(), local_cpu_seed_beacon());
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    testnet.run_matmul_round(&scheduler);
    let matmul_settled_receipts = testnet.chain.state.settled_receipts.len();
    testnet.run_linear_training_round(&scheduler);

    let store = NodeStore::open(data_dir);
    let status = store
        .persist_chain(&testnet.chain)
        .map_err(|error| format!("failed to persist seeded local testnet chain: {error}"))?;
    let telemetry = testnet.telemetry();
    let local_evidence = testnet.public_testnet_evidence(
        &PublicTestnetCriteria {
            duration_days: 0,
            min_finality_rate_bps: 10_000,
            min_data_availability_bps: 9_500,
            ..PublicTestnetCriteria::default()
        },
        true,
    );
    let rewarded_miners = testnet
        .miners
        .iter()
        .filter(|miner| testnet.chain.state.rewards.balance(miner) > 0)
        .count();
    let total_reward_balance: u64 = testnet.chain.state.rewards.balances.values().sum();
    let attestation_count: usize = testnet
        .chain
        .state
        .attestations
        .values()
        .map(Vec::len)
        .sum();
    Ok(format!(
        "command=local_testnet_seed\ndata_dir={data_dir}\nminers={}\nvalidators={}\nheight={}\nblocks={}\nsettled_receipts={}\nmatmul_settled={}\nlinear_training_settled={}\nmodel_states={}\nrewarded_miners={rewarded_miners}\ntotal_reward_balance={total_reward_balance}\nattestation_count={attestation_count}\ntotal_tensor_work={}\nfinality_rate_bps={}\ndata_availability_bps={}\nnode_store_ready=true\npersisted_block_count={}\nlatest_block_hash={}\npublic_evidence_full_spec=false\nindependently_checkable=false",
        testnet.miners.len(),
        testnet.validators.len(),
        testnet.chain.state.height,
        testnet.chain.blocks.len(),
        testnet.chain.state.settled_receipts.len(),
        matmul_settled_receipts > 0,
        !testnet.chain.state.model_states.is_empty(),
        testnet.chain.state.model_states.len(),
        telemetry.total_tensor_work,
        local_evidence.finality_rate_bps,
        local_evidence.data_availability_bps,
        status.block_count,
        hex(&status.latest_block_hash)
    ))
}

fn service_status(data_dir: &str) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    let chain = store
        .load_chain()
        .map_err(|error| format!("failed to load node store {data_dir}: {error}"))?;
    let status = store
        .status()
        .map_err(|error| format!("failed to inspect node store {data_dir}: {error}"))?;
    let latest_block_height = chain
        .blocks
        .last()
        .map(|block| block.height)
        .unwrap_or_default();
    let finalized_block_count = chain
        .blocks
        .iter()
        .filter(|block| chain.is_block_finalized(&block.hash()))
        .count();
    let first_live_block = chain.blocks.iter().find(|block| block.height > 2);
    let first_live_block_height = first_live_block
        .map(|block| block.height)
        .unwrap_or_default();
    let first_live_block_hash = first_live_block
        .map(|block| block.hash())
        .unwrap_or([0; 32]);
    let bootstrap_peer_count = if store.peer_book_store().path().exists() {
        store
            .peer_book_store()
            .load_bootstrap_addresses()
            .map_err(|error| format!("failed to inspect peer book {data_dir}: {error}"))?
            .len()
    } else {
        0
    };
    let attestation_count: usize = chain.state.attestations.values().map(Vec::len).sum();
    let reward_account_count = chain
        .state
        .rewards
        .balances
        .values()
        .filter(|balance| **balance > 0)
        .count();
    Ok(format!(
        "command=service_status\ndata_dir={}\noperator_name={}\noperator_id={}\nrole={}\nruntime_command={}\nrole_runtime_command={}\nrole_loop_ready={}\nrole_loop_role={}\nrole_chain_profile={}\nrole_can_produce_blocks={}\nrole_wallet_address={}\nrole_wallet_registration={}\nrole_wallet_registered={}\nrole_miner_work_ready={}\nrole_miner_assigned_jobs_seen={}\nrole_miner_unreceipted_jobs={}\nrole_miner_receipts_submitted={}\nrole_miner_tensors_inserted={}\nrole_validator_work_ready={}\nrole_validator_assigned_receipts_seen={}\nrole_validator_unattested_receipts={}\nrole_validator_artifact_ready_receipts={}\nrole_validator_artifact_missing_receipts={}\nrole_validator_attestations_submitted={}\nrole_local_producer={}\nrole_served_requests={}\nrole_produced_blocks={}\nrole_network_applied_blocks={}\nrole_network_events_ingested={}\nrole_network_block_events_ingested={}\nrole_network_block_headers_ingested={}\nrole_network_job_events_ingested={}\nrole_network_job_payloads_ingested={}\nrole_network_job_payloads_applied={}\nrole_network_receipt_events_ingested={}\nrole_network_receipt_payloads_ingested={}\nrole_network_receipt_payloads_applied={}\nrole_network_attestation_events_ingested={}\nrole_network_attestation_payloads_ingested={}\nrole_network_attestation_payloads_applied={}\nrole_network_peer_events_ingested={}\nrole_network_invalid_events={}\nrole_latest_height={}\nrole_p2p_connected_peers={}\nrole_p2p_observed_blocks={}\nrole_p2p_observed_jobs={}\nrole_p2p_observed_receipts={}\nrole_p2p_observed_attestations={}\nrole_p2p_latest_observed_block_height={}\nrole_p2p_latest_observed_block_hash={}\nrole_p2p_observed_block_hashes={}\nnode_multiaddr={}\np2p_peer_id={}\nheight={}\nepoch={}\nblock_count={}\nlatest_block_height={latest_block_height}\nlatest_block_hash={}\nstate_root={}\nblock_log_root={}\nfinalized_block_count={finalized_block_count}\nfirst_live_block_height={first_live_block_height}\nfirst_live_block_hash={}\nregistered_miner_count={}\nregistered_validator_count={}\njob_count={}\nreceipt_count={}\nsettled_receipt_count={}\nattestation_count={attestation_count}\nreward_account_count={reward_account_count}\nmodel_count={}\nbootstrap_peer_count={bootstrap_peer_count}\nnode_store_ready=true\nstatus_source=node_store",
        status.data_dir.display(),
        ready_file_field(data_dir, "operator_name"),
        ready_file_field(data_dir, "operator_id"),
        ready_file_field(data_dir, "role"),
        ready_file_field(data_dir, "runtime_command"),
        role_runtime_status_field(data_dir, "role_runtime_command"),
        role_runtime_status_field(data_dir, "role_loop_ready"),
        role_runtime_status_field(data_dir, "role_loop_role"),
        role_runtime_status_field(data_dir, "role_chain_profile"),
        role_runtime_status_field(data_dir, "role_can_produce_blocks"),
        role_runtime_status_field(data_dir, "role_wallet_address"),
        role_runtime_status_field(data_dir, "role_wallet_registration"),
        role_runtime_status_field(data_dir, "role_wallet_registered"),
        role_runtime_status_field(data_dir, "role_miner_work_ready"),
        role_runtime_status_field(data_dir, "role_miner_assigned_jobs_seen"),
        role_runtime_status_field(data_dir, "role_miner_unreceipted_jobs"),
        role_runtime_status_field(data_dir, "role_miner_receipts_submitted"),
        role_runtime_status_field(data_dir, "role_miner_tensors_inserted"),
        role_runtime_status_field(data_dir, "role_validator_work_ready"),
        role_runtime_status_field(data_dir, "role_validator_assigned_receipts_seen"),
        role_runtime_status_field(data_dir, "role_validator_unattested_receipts"),
        role_runtime_status_field(data_dir, "role_validator_artifact_ready_receipts"),
        role_runtime_status_field(data_dir, "role_validator_artifact_missing_receipts"),
        role_runtime_status_field(data_dir, "role_validator_attestations_submitted"),
        role_runtime_status_field(data_dir, "role_local_producer"),
        role_runtime_status_field(data_dir, "role_served_requests"),
        role_runtime_status_field(data_dir, "role_produced_blocks"),
        role_runtime_status_field(data_dir, "role_network_applied_blocks"),
        role_runtime_status_field(data_dir, "role_network_events_ingested"),
        role_runtime_status_field(data_dir, "role_network_block_events_ingested"),
        role_runtime_status_field(data_dir, "role_network_block_headers_ingested"),
        role_runtime_status_field(data_dir, "role_network_job_events_ingested"),
        role_runtime_status_field(data_dir, "role_network_job_payloads_ingested"),
        role_runtime_status_field(data_dir, "role_network_job_payloads_applied"),
        role_runtime_status_field(data_dir, "role_network_receipt_events_ingested"),
        role_runtime_status_field(data_dir, "role_network_receipt_payloads_ingested"),
        role_runtime_status_field(data_dir, "role_network_receipt_payloads_applied"),
        role_runtime_status_field(data_dir, "role_network_attestation_events_ingested"),
        role_runtime_status_field(data_dir, "role_network_attestation_payloads_ingested"),
        role_runtime_status_field(data_dir, "role_network_attestation_payloads_applied"),
        role_runtime_status_field(data_dir, "role_network_peer_events_ingested"),
        role_runtime_status_field(data_dir, "role_network_invalid_events"),
        role_runtime_status_field(data_dir, "role_latest_height"),
        role_runtime_status_field(data_dir, "role_p2p_connected_peers"),
        role_runtime_status_field(data_dir, "role_p2p_observed_blocks"),
        role_runtime_status_field(data_dir, "role_p2p_observed_jobs"),
        role_runtime_status_field(data_dir, "role_p2p_observed_receipts"),
        role_runtime_status_field(data_dir, "role_p2p_observed_attestations"),
        role_runtime_status_field(data_dir, "role_p2p_latest_observed_block_height"),
        role_runtime_status_field(data_dir, "role_p2p_latest_observed_block_hash"),
        role_runtime_status_field(data_dir, "role_p2p_observed_block_hashes"),
        ready_file_field(data_dir, "node_multiaddr"),
        ready_file_field(data_dir, "p2p_peer_id"),
        chain.state.height,
        chain.state.epoch,
        status.block_count,
        hex(&status.latest_block_hash),
        hex(&chain.state_root()),
        hex(&status.block_log_root),
        hex(&first_live_block_hash),
        chain.state.miners.len(),
        chain.state.validators.len(),
        chain.state.jobs.len(),
        chain.state.receipts.len(),
        chain.state.settled_receipts.len(),
        chain.state.model_states.len(),
    ))
}

fn service_block_status(data_dir: &str, height: u64) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    let chain = store
        .load_chain()
        .map_err(|error| format!("failed to load node store {data_dir}: {error}"))?;
    let Some(block) = chain.blocks.iter().find(|block| block.height == height) else {
        return Err(format!(
            "block height {height} is not in node store {data_dir}"
        ));
    };
    let block_hash = block.hash();
    let mut receipt_ids = Vec::new();
    let mut tensor_op_receipt_ids = Vec::new();
    let mut linear_training_receipt_ids = Vec::new();
    let mut settled_receipt_ids = Vec::new();
    for receipt in chain
        .state
        .receipts
        .values()
        .filter(|receipt| receipt.submitted_at_block() == height)
    {
        let receipt_id = receipt.receipt_id();
        receipt_ids.push(receipt_id);
        if chain.state.settled_receipts.contains(&receipt_id) {
            settled_receipt_ids.push(receipt_id);
        }
        match receipt.primitive_type() {
            PrimitiveType::TensorOp => tensor_op_receipt_ids.push(receipt_id),
            PrimitiveType::LinearTrainingStep => linear_training_receipt_ids.push(receipt_id),
        }
    }
    Ok(format!(
        "command=service_block\ndata_dir={data_dir}\nheight={height}\nblock_hash={}\nstate_root={}\nepoch={}\nlatest_height={}\nfinalized={}\nreceipt_count={}\nreceipt_ids={}\ntensor_op_receipt_count={}\ntensor_op_receipt_ids={}\nlinear_training_receipt_count={}\nlinear_training_receipt_ids={}\nsettled_receipt_count={}\nsettled_receipt_ids={}\nstatus_source=node_store",
        hex(&block_hash),
        hex(&block.state_root),
        block.epoch,
        chain.state.height,
        chain.is_block_finalized(&block_hash),
        receipt_ids.len(),
        hex_hash_list(&receipt_ids),
        tensor_op_receipt_ids.len(),
        hex_hash_list(&tensor_op_receipt_ids),
        linear_training_receipt_ids.len(),
        hex_hash_list(&linear_training_receipt_ids),
        settled_receipt_ids.len(),
        hex_hash_list(&settled_receipt_ids),
    ))
}

#[derive(Clone, Copy, Debug)]
struct RoleServiceConfig<'a> {
    wallet: &'a str,
    device: Option<&'a str>,
    node: &'a str,
    listen: &'a str,
    p2p_listen: &'a str,
    data_dir: &'a str,
    identity_seed: Option<[u8; 32]>,
    auth_token: &'a str,
    max_requests: usize,
}

fn run_miner_service(config: RoleServiceConfig<'_>) -> std::result::Result<String, String> {
    RoleRunLoop::miner().run(config)
}

fn run_validator_service(config: RoleServiceConfig<'_>) -> std::result::Result<String, String> {
    RoleRunLoop::validator().run(config)
}

fn run_proposer_service(config: RoleServiceConfig<'_>) -> std::result::Result<String, String> {
    RoleRunLoop::proposer().run(config)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RoleRunLoopKind {
    Miner,
    Validator,
    Proposer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RoleRunLoop {
    kind: RoleRunLoopKind,
}

impl RoleRunLoop {
    fn miner() -> Self {
        Self {
            kind: RoleRunLoopKind::Miner,
        }
    }

    fn validator() -> Self {
        Self {
            kind: RoleRunLoopKind::Validator,
        }
    }

    fn proposer() -> Self {
        Self {
            kind: RoleRunLoopKind::Proposer,
        }
    }

    fn runtime_command(self) -> &'static str {
        match self.kind {
            RoleRunLoopKind::Miner => "miner_run",
            RoleRunLoopKind::Validator => "validator_run",
            RoleRunLoopKind::Proposer => "proposer_run",
        }
    }

    fn runtime_role(self) -> RuntimeRole {
        match self.kind {
            RoleRunLoopKind::Miner => RuntimeRole::Miner,
            RoleRunLoopKind::Validator => RuntimeRole::Validator,
            RoleRunLoopKind::Proposer => RuntimeRole::Proposer,
        }
    }

    fn service_runtime_config(
        self,
        config: RoleServiceConfig<'_>,
    ) -> std::result::Result<ServiceRuntimeConfig, String> {
        let role = self.runtime_role();
        Ok(ServiceRuntimeConfig {
            runtime_command: self.runtime_command(),
            role,
            role_wallet_address: Some(role_wallet_address(config.wallet)?),
            node: runtime_node_config(
                config.data_dir,
                role,
                config.listen,
                config.p2p_listen,
                config.identity_seed,
                config.auth_token,
                config.max_requests,
            )?,
        })
    }

    fn run(self, config: RoleServiceConfig<'_>) -> std::result::Result<String, String> {
        let service_report = run_role_runtime_loop(self.service_runtime_config(config)?)?;
        Ok(self.format_report(config, &service_report))
    }

    fn format_report(self, config: RoleServiceConfig<'_>, service_report: &str) -> String {
        match self.kind {
            RoleRunLoopKind::Miner => format!(
                "command=miner_run\nrole=miner\nwallet={}\ndevice={}\nnode={}\nrole_runtime_ready=true\n{service_report}",
                config.wallet,
                config.device.unwrap_or("unknown"),
                config.node
            ),
            RoleRunLoopKind::Validator => format!(
                "command=validator_run\nrole=validator\nwallet={}\nnode={}\nreference_verifier_ready=true\nrole_runtime_ready=true\n{service_report}",
                config.wallet, config.node
            ),
            RoleRunLoopKind::Proposer => format!(
                "command=proposer_run\nrole=proposer\nwallet={}\nnode={}\nproposer_ready=true\nrole_runtime_ready=true\n{service_report}",
                config.wallet, config.node
            ),
        }
    }
}

fn serve_service(
    listen: &str,
    p2p_listen: &str,
    data_dir: &str,
    identity_seed: Option<[u8; 32]>,
    auth_token: &str,
    max_requests: usize,
) -> std::result::Result<String, String> {
    run_role_runtime_loop(ServiceRuntimeConfig {
        runtime_command: "service_serve",
        role: RuntimeRole::Service,
        role_wallet_address: None,
        node: runtime_node_config(
            data_dir,
            RuntimeRole::Service,
            listen,
            p2p_listen,
            identity_seed,
            auth_token,
            max_requests,
        )?,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimeRole {
    Service,
    Miner,
    Validator,
    Proposer,
}

impl RuntimeRole {
    fn label(self) -> &'static str {
        match self {
            Self::Service => "service",
            Self::Miner => "miner",
            Self::Validator => "validator",
            Self::Proposer => "proposer",
        }
    }

    fn node_role(self) -> NodeRole {
        match self {
            Self::Service => NodeRole::Gateway,
            Self::Miner => NodeRole::Miner,
            Self::Validator => NodeRole::Validator,
            Self::Proposer => NodeRole::Proposer,
        }
    }
}

fn runtime_chain_profile() -> std::result::Result<ChainProfile, String> {
    let label = std::env::var("TENSORVM_CHAIN_PROFILE").unwrap_or_else(|_| "local_cpu".to_owned());
    chain_profile_from_label(&label)
}

fn chain_profile_from_label(label: &str) -> std::result::Result<ChainProfile, String> {
    ChainProfile::from_label(label).ok_or_else(|| {
        format!(
            "unsupported TENSORVM_CHAIN_PROFILE {label:?}; expected local_cpu, public_testnet, or mainnet"
        )
    })
}

fn runtime_node_config(
    data_dir: &str,
    role: RuntimeRole,
    listen: &str,
    p2p_listen: &str,
    identity_seed: Option<[u8; 32]>,
    auth_token: &str,
    max_requests: usize,
) -> std::result::Result<NodeConfig, String> {
    Ok(
        NodeConfig::new(runtime_chain_profile()?, role.node_role(), data_dir)
            .with_network(
                NetworkConfig::new(listen, p2p_listen)
                    .with_identity_seed(identity_seed)
                    .with_auth_token(auth_token)
                    .with_max_requests(max_requests),
            )
            .with_block_interval(runtime_block_interval())
            .with_local_producer(runtime_local_block_producer()),
    )
}

#[derive(Debug)]
struct ServiceRuntimeConfig {
    runtime_command: &'static str,
    role: RuntimeRole,
    role_wallet_address: Option<Address>,
    node: NodeConfig,
}

fn role_wallet_address(wallet: &str) -> std::result::Result<Address, String> {
    let wallet = wallet.trim();
    if wallet.is_empty() {
        return Err("wallet argument is empty".to_owned());
    }
    Ok(address(wallet.as_bytes()))
}

fn runtime_role_wallet_address_text(address: Option<Address>) -> String {
    address
        .map(|address| hex(&address))
        .unwrap_or_else(|| "none".to_owned())
}

fn runtime_role_wallet_registration(
    role: RuntimeRole,
    address: Option<Address>,
    chain: &LocalChain,
) -> &'static str {
    let Some(address) = address else {
        return "none";
    };
    match role {
        RuntimeRole::Miner => {
            if chain.state.miners.contains_key(&address) {
                "miner"
            } else {
                "unregistered"
            }
        }
        RuntimeRole::Validator => {
            if chain.state.validators.contains_key(&address) {
                "validator"
            } else {
                "unregistered"
            }
        }
        RuntimeRole::Proposer if chain.state.miners.contains_key(&address) => "miner",
        RuntimeRole::Proposer if chain.state.validators.contains_key(&address) => "validator",
        RuntimeRole::Proposer => "unregistered",
        RuntimeRole::Service => "none",
    }
}

fn runtime_role_wallet_registered(
    role: RuntimeRole,
    address: Option<Address>,
    chain: &LocalChain,
) -> bool {
    !matches!(
        runtime_role_wallet_registration(role, address, chain),
        "none" | "unregistered"
    )
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct MinerRoleWorkObservation {
    assigned_jobs: BTreeSet<Hash>,
    unreceipted_jobs: BTreeSet<Hash>,
}

fn miner_role_work_observation(chain: &LocalChain, miner: Address) -> MinerRoleWorkObservation {
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let assignment_seed = chain.state.finalized_randomness;
    let mut observation = MinerRoleWorkObservation::default();
    for job_id in chain.state.jobs.keys() {
        let assignment = scheduler.assign_miners(chain, *job_id, &assignment_seed);
        if !assignment.miners.contains(&miner) {
            continue;
        }
        observation.assigned_jobs.insert(*job_id);
        if !miner_has_receipt_for_job(chain, miner, *job_id) {
            observation.unreceipted_jobs.insert(*job_id);
        }
    }
    observation
}

fn miner_has_receipt_for_job(chain: &LocalChain, miner: Address, job_id: Hash) -> bool {
    chain
        .state
        .receipts
        .values()
        .any(|receipt| receipt.job_id() == job_id && receipt.miner() == miner)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct MinerRoleReceiptSubmission {
    receipts_submitted: usize,
    tensors_inserted: usize,
}

fn submit_miner_role_receipt(
    node: &mut RpcNode,
    miner: Address,
    job_id: Hash,
) -> std::result::Result<Option<MinerRoleReceiptSubmission>, String> {
    if !node.chain.state.miners.contains_key(&miner) {
        return Ok(None);
    }
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let assignment =
        scheduler.assign_miners(&node.chain, job_id, &node.chain.state.finalized_randomness);
    if !assignment.miners.contains(&miner) || miner_has_receipt_for_job(&node.chain, miner, job_id)
    {
        return Ok(None);
    }
    let Some(job) = node.chain.state.jobs.get(&job_id).cloned() else {
        return Ok(None);
    };
    let bundle = CpuReferenceMinerRole::new(miner)
        .execute_job(&job, node.chain.state.height, 1)
        .map_err(|error| format!("miner role failed to execute job {}: {error}", hex(&job_id)))?;
    if bundle.receipt.job_id() != job_id || bundle.receipt.miner() != miner {
        return Err("miner role produced receipt for the wrong job or miner".to_owned());
    }
    let served_tensors = bundle.served_tensors();
    node.chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt))
        .map_err(|error| {
            format!(
                "miner role failed to submit receipt {}: {error}",
                hex(&job_id)
            )
        })?;
    let mut tensors_inserted = 0usize;
    for tensor in served_tensors {
        node.insert_tensor(tensor);
        tensors_inserted = tensors_inserted.saturating_add(1);
    }
    Ok(Some(MinerRoleReceiptSubmission {
        receipts_submitted: 1,
        tensors_inserted,
    }))
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ValidatorRoleWorkObservation {
    assigned_receipts: BTreeSet<Hash>,
    unattested_receipts: BTreeSet<Hash>,
    artifact_ready_receipts: BTreeSet<Hash>,
    artifact_missing_receipts: BTreeSet<Hash>,
}

fn validator_role_work_observation(
    node: &RpcNode,
    validator: Address,
) -> ValidatorRoleWorkObservation {
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let assignment_seed = node.chain.state.finalized_randomness;
    let mut observation = ValidatorRoleWorkObservation::default();
    for (receipt_id, receipt) in &node.chain.state.receipts {
        let assignment = scheduler.assign_validators(&node.chain, *receipt_id, &assignment_seed);
        if !assignment.validators.contains(&validator) {
            continue;
        }
        observation.assigned_receipts.insert(*receipt_id);
        if validator_has_attested_for_receipt(&node.chain, validator, *receipt_id) {
            continue;
        }
        observation.unattested_receipts.insert(*receipt_id);
        if role_receipt_bundle_from_local_tensors(node, receipt).is_some() {
            observation.artifact_ready_receipts.insert(*receipt_id);
        } else {
            observation.artifact_missing_receipts.insert(*receipt_id);
        }
    }
    observation
}

fn validator_has_attested_for_receipt(
    chain: &LocalChain,
    validator: Address,
    receipt_id: Hash,
) -> bool {
    chain
        .state
        .attestations
        .get(&receipt_id)
        .is_some_and(|attestations| {
            attestations
                .iter()
                .any(|attestation| attestation.validator == validator)
        })
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ValidatorRoleAttestationSubmission {
    attestations_submitted: usize,
}

fn submit_validator_role_attestation(
    node: &mut RpcNode,
    validator: Address,
    receipt_id: Hash,
) -> std::result::Result<Option<ValidatorRoleAttestationSubmission>, String> {
    let Some(validator_state) = node.chain.state.validators.get(&validator) else {
        return Ok(None);
    };
    let validator_stake = validator_state.stake;
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let assignment = scheduler.assign_validators(
        &node.chain,
        receipt_id,
        &node.chain.state.finalized_randomness,
    );
    if !assignment.validators.contains(&validator)
        || validator_has_attested_for_receipt(&node.chain, validator, receipt_id)
    {
        return Ok(None);
    }
    let Some(receipt) = node.chain.state.receipts.get(&receipt_id).cloned() else {
        return Ok(None);
    };
    let Some(job) = node.chain.state.jobs.get(&receipt.job_id()).cloned() else {
        return Ok(None);
    };
    let Some(bundle) = role_receipt_bundle_from_local_tensors(node, &receipt) else {
        return Ok(None);
    };
    let validation_seed = node.chain.validation_seed(&receipt_id);
    let attestation = ReferenceValidatorRole::new(validator, validator_stake)
        .verify_receipt(
            &job,
            &bundle,
            &validation_seed,
            &node.chain.params.freivalds,
        )
        .map_err(|error| {
            format!(
                "validator role failed to verify receipt {}: {error}",
                hex(&receipt_id)
            )
        })?;
    if attestation.receipt_id != receipt_id || attestation.validator != validator {
        return Err(
            "validator role produced attestation for the wrong receipt or validator".to_owned(),
        );
    }
    node.chain
        .apply_command(ChainCommand::SubmitAttestation(attestation))
        .map_err(|error| {
            format!(
                "validator role failed to submit attestation {}: {error}",
                hex(&receipt_id)
            )
        })?;
    Ok(Some(ValidatorRoleAttestationSubmission {
        attestations_submitted: 1,
    }))
}

fn role_receipt_bundle_from_local_tensors(
    node: &RpcNode,
    receipt: &ReceiptState,
) -> Option<RoleReceiptBundle> {
    let job = node.chain.state.jobs.get(&receipt.job_id())?;
    match (job, receipt) {
        (tensor_vm::JobState::TensorOp(_), ReceiptState::TensorOp(receipt)) => {
            let a = node
                .tensor_by_commitment_root(receipt.input_roots.first()?)?
                .clone();
            let b = node
                .tensor_by_commitment_root(receipt.input_roots.get(1)?)?
                .clone();
            let c = node
                .tensor_by_commitment_root(receipt.output_roots.first()?)?
                .clone();
            Some(RoleReceiptBundle {
                receipt: ReceiptState::TensorOp(receipt.clone()),
                artifacts: RoleReceiptArtifacts::TensorOp { a, b, c },
            })
        }
        (
            tensor_vm::JobState::LinearTrainingStep(job),
            ReceiptState::LinearTrainingStep(receipt),
        ) => {
            let weights_before = SyntheticLocalJobSource::linear_training_weights();
            if weights_before.commitment_root() != job.weight_root_before
                || receipt.weight_root_before != job.weight_root_before
            {
                return None;
            }
            let (x, target) = job.batch_tensors().ok()?;
            let y = node.tensor_by_commitment_root(&receipt.y_root)?.clone();
            let grad_w = node
                .tensor_by_commitment_root(&receipt.grad_w_root)?
                .clone();
            let weight_after = node
                .tensor_by_commitment_root(&receipt.weight_root_after)?
                .clone();
            let dy = y.sub(&target).ok()?;
            Some(RoleReceiptBundle {
                receipt: ReceiptState::LinearTrainingStep(receipt.clone()),
                artifacts: RoleReceiptArtifacts::LinearTrainingStep {
                    weights_before,
                    output: Box::new(LinearTrainingStepOutput {
                        x,
                        target,
                        y,
                        dy,
                        grad_w,
                        weight_after,
                        loss_commitment: receipt.loss_commitment,
                    }),
                },
            })
        }
        _ => None,
    }
}

fn run_role_runtime_loop(config: ServiceRuntimeConfig) -> std::result::Result<String, String> {
    let mut runtime = RoleRuntimeLoop::start(config)?;
    runtime.run_until_max_requests()?;
    Ok(runtime.report())
}

struct RoleRuntimeLoop {
    config: ServiceRuntimeConfig,
    store: NodeStore,
    server: RpcHttpServer,
    p2p_service: TensorVmLibp2pService,
    local_producer: bool,
    block_interval: Option<Duration>,
    next_block_at: Option<Instant>,
    runtime_state: NodeRuntimeState,
    p2p_peer_id: String,
    p2p_topics: usize,
    p2p_request_response_protocols: usize,
    bootstrap_peer_count: usize,
    identity: String,
    max_transmit_bytes: usize,
    request_timeout_seconds: u64,
    max_concurrent_streams: usize,
    idle_timeout_seconds: u64,
}

impl RoleRuntimeLoop {
    fn start(config: ServiceRuntimeConfig) -> std::result::Result<Self, String> {
        let network = &config.node.network;
        let store = NodeStore::open(config.node.data_dir());
        let chain = store.load_chain().map_err(|error| {
            format!(
                "failed to load node store {}: {error}",
                config.node.data_dir().display()
            )
        })?;
        let bootstrap_addresses = if store.peer_book_store().path().exists() {
            store
                .peer_book_store()
                .load_bootstrap_addresses()
                .map_err(|error| {
                    format!(
                        "failed to load libp2p peer book {}: {error}",
                        config.node.data_dir().display()
                    )
                })?
        } else {
            Vec::new()
        };
        let bootstrap_peer_count = bootstrap_addresses.len();
        let p2p_config = Libp2pControlPlaneConfig {
            listen_addresses: vec![network.p2p_listen.clone()],
            bootstrap_addresses,
            identity_seed: network.identity_seed,
            ..Libp2pControlPlaneConfig::default()
        };
        let max_transmit_bytes = p2p_config.max_gossipsub_transmit_bytes;
        let request_timeout_seconds = p2p_config.request_timeout_seconds;
        let max_concurrent_streams = p2p_config.max_concurrent_request_streams;
        let idle_timeout_seconds = p2p_config.idle_connection_timeout_seconds;
        let p2p_service = spawn_libp2p_service(p2p_config)
            .map_err(|error| format!("failed to start mandatory libp2p service: {error}"))?;
        let p2p_peer_id = p2p_service.peer_id().to_string();
        let p2p_topics = p2p_service.info().subscribed_topics.len();
        let p2p_request_response_protocols = p2p_service.info().request_response_protocols.len();
        let identity = p2p_identity_report(network.identity_seed);
        let node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
        let gateway = RpcGateway::new(
            node,
            RpcPolicy {
                auth_token: Some(network.auth_token.clone()),
                ..RpcPolicy::default()
            },
        );
        let server = RpcHttpServer::bind(&network.rpc_listen, gateway).map_err(|error| {
            format!(
                "failed to bind service listener {}: {error}",
                network.rpc_listen
            )
        })?;
        let block_interval = config.node.synthetic_block_interval();
        let next_block_at = block_interval.map(|interval| Instant::now() + interval);
        let local_producer = config.node.local_synthetic_producer();
        Ok(Self {
            config,
            store,
            server,
            p2p_service,
            local_producer,
            block_interval,
            next_block_at,
            runtime_state: NodeRuntimeState::default(),
            p2p_peer_id,
            p2p_topics,
            p2p_request_response_protocols,
            bootstrap_peer_count,
            identity,
            max_transmit_bytes,
            request_timeout_seconds,
            max_concurrent_streams,
            idle_timeout_seconds,
        })
    }

    fn run_until_max_requests(&mut self) -> std::result::Result<(), String> {
        self.write_status()?;
        if self.block_interval.is_some() {
            self.server.set_nonblocking(true).map_err(|error| {
                format!("failed to configure nonblocking service listener: {error}")
            })?;
        }
        loop {
            if self.max_requests_reached() {
                break;
            }
            self.serve_rpc_once()?;
            if self.block_interval.is_some() {
                self.ingest_network_once()?;
            }
            self.tick_role_work_once()?;
            if self.block_interval.is_some() {
                self.produce_local_round_if_due()?;
                thread::sleep(Duration::from_millis(25));
            }
        }
        Ok(())
    }

    fn max_requests_reached(&self) -> bool {
        let max_requests = self.config.node.network.max_requests;
        max_requests != 0 && self.runtime_state.served_requests() >= max_requests
    }

    fn serve_rpc_once(&mut self) -> std::result::Result<(), String> {
        if self.block_interval.is_some() {
            match self.server.serve_next() {
                Ok(()) => self.record_served_request(),
                Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(()),
                Err(error) => Err(format!("service request failed: {error}")),
            }
        } else {
            self.server
                .serve_next()
                .map_err(|error| format!("service request failed: {error}"))?;
            self.record_served_request()
        }
    }

    fn record_served_request(&mut self) -> std::result::Result<(), String> {
        self.store
            .persist_chain(&self.server.gateway().node.chain)
            .map_err(|error| format!("failed to persist service state: {error}"))?;
        self.runtime_state.record_served_request();
        self.write_status()
    }

    fn ingest_network_once(&mut self) -> std::result::Result<(), String> {
        let ingested = ingest_network_events(
            &mut self.server,
            &self.p2p_service,
            self.local_producer,
            self.runtime_state.pending_payloads_mut(),
        )?;
        if !ingested.has_activity() {
            return Ok(());
        }
        let should_persist = ingested.applied_blocks > 0
            || ingested.job_payloads_applied > 0
            || ingested.receipt_payloads_applied > 0
            || ingested.attestation_payloads_applied > 0;
        self.runtime_state.record_network_ingest(ingested);
        if should_persist {
            self.store
                .persist_chain(&self.server.gateway().node.chain)
                .map_err(|error| format!("failed to persist network-applied state: {error}"))?;
        }
        self.write_status()
    }

    fn tick_role_work_once(&mut self) -> std::result::Result<(), String> {
        match self.config.role {
            RuntimeRole::Miner => self.tick_miner_role_work_once(),
            RuntimeRole::Validator => self.tick_validator_role_work_once(),
            RuntimeRole::Proposer | RuntimeRole::Service => Ok(()),
        }
    }

    fn tick_miner_role_work_once(&mut self) -> std::result::Result<(), String> {
        let Some(miner) = self.config.role_wallet_address else {
            return Ok(());
        };
        if runtime_role_wallet_registration(
            self.config.role,
            self.config.role_wallet_address,
            &self.server.gateway().node.chain,
        ) != "miner"
        {
            return Ok(());
        }
        let observation = miner_role_work_observation(&self.server.gateway().node.chain, miner);
        let job_to_submit = observation.unreceipted_jobs.iter().next().copied();
        let mut status_changed = false;
        if self
            .runtime_state
            .record_miner_work_observation(observation.assigned_jobs, observation.unreceipted_jobs)
        {
            status_changed = true;
        }
        if let Some(job_id) = job_to_submit {
            let announcement_checkpoint =
                chain_announcement_checkpoint(&self.server.gateway().node.chain);
            if let Some(submission) =
                submit_miner_role_receipt(&mut self.server.gateway_mut().node, miner, job_id)?
            {
                publish_new_chain_announcements(
                    &self.p2p_service,
                    &announcement_checkpoint,
                    &self.server.gateway().node.chain,
                )?;
                self.store
                    .persist_chain(&self.server.gateway().node.chain)
                    .map_err(|error| format!("failed to persist miner receipt state: {error}"))?;
                self.runtime_state.record_miner_receipt_submission(
                    submission.receipts_submitted,
                    submission.tensors_inserted,
                );
                let observation =
                    miner_role_work_observation(&self.server.gateway().node.chain, miner);
                self.runtime_state.record_miner_work_observation(
                    observation.assigned_jobs,
                    observation.unreceipted_jobs,
                );
                status_changed = true;
            }
        }
        if status_changed {
            self.write_status()?;
        }
        Ok(())
    }

    fn tick_validator_role_work_once(&mut self) -> std::result::Result<(), String> {
        let Some(validator) = self.config.role_wallet_address else {
            return Ok(());
        };
        if runtime_role_wallet_registration(
            self.config.role,
            self.config.role_wallet_address,
            &self.server.gateway().node.chain,
        ) != "validator"
        {
            return Ok(());
        }
        let observation = validator_role_work_observation(&self.server.gateway().node, validator);
        let receipt_to_submit = observation.artifact_ready_receipts.iter().next().copied();
        let mut status_changed = false;
        if self.runtime_state.record_validator_work_observation(
            observation.assigned_receipts,
            observation.unattested_receipts,
            observation.artifact_ready_receipts,
            observation.artifact_missing_receipts,
        ) {
            status_changed = true;
        }
        if let Some(receipt_id) = receipt_to_submit {
            let announcement_checkpoint =
                chain_announcement_checkpoint(&self.server.gateway().node.chain);
            if let Some(submission) = submit_validator_role_attestation(
                &mut self.server.gateway_mut().node,
                validator,
                receipt_id,
            )? {
                publish_new_chain_announcements(
                    &self.p2p_service,
                    &announcement_checkpoint,
                    &self.server.gateway().node.chain,
                )?;
                self.store
                    .persist_chain(&self.server.gateway().node.chain)
                    .map_err(|error| {
                        format!("failed to persist validator attestation state: {error}")
                    })?;
                self.runtime_state
                    .record_validator_attestation_submission(submission.attestations_submitted);
                let observation =
                    validator_role_work_observation(&self.server.gateway().node, validator);
                self.runtime_state.record_validator_work_observation(
                    observation.assigned_receipts,
                    observation.unattested_receipts,
                    observation.artifact_ready_receipts,
                    observation.artifact_missing_receipts,
                );
                status_changed = true;
            }
        }
        if status_changed {
            self.write_status()?;
        }
        Ok(())
    }

    fn produce_local_round_if_due(&mut self) -> std::result::Result<(), String> {
        let Some(interval) = self.block_interval else {
            return Ok(());
        };
        if !self
            .next_block_at
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Ok(());
        }
        if self.local_producer
            && produce_and_publish_synthetic_round(
                &mut self.server,
                &self.p2p_service,
                &self.config.node.profile,
            )?
            .is_some()
        {
            self.store
                .persist_chain(&self.server.gateway().node.chain)
                .map_err(|error| format!("failed to persist produced block: {error}"))?;
            self.runtime_state.record_produced_block();
            self.write_status()?;
        }
        self.next_block_at = Some(Instant::now() + interval);
        Ok(())
    }

    fn write_status(&self) -> std::result::Result<(), String> {
        write_role_runtime_status(
            &self.config,
            &RoleRuntimeStatusSnapshot::from_runtime_state(
                &self.runtime_state,
                &self.server,
                &self.p2p_service,
                self.local_producer,
                self.config.role,
                self.config.role_wallet_address,
            ),
        )
    }

    fn report(&self) -> String {
        let network = &self.config.node.network;
        let network_events = self.runtime_state.network_events();
        format!(
            "command=service_serve\nruntime_command={}\nrole={}\nchain_profile={}\nrole_loop_ready=true\nrole_can_produce_blocks={}\nrole_wallet_address={}\nrole_wallet_registration={}\nrole_wallet_registered={}\nminer_work_ready={}\nminer_assigned_jobs_seen={}\nminer_unreceipted_jobs={}\nminer_receipts_submitted={}\nminer_tensors_inserted={}\nvalidator_work_ready={}\nvalidator_assigned_receipts_seen={}\nvalidator_unattested_receipts={}\nvalidator_artifact_ready_receipts={}\nvalidator_artifact_missing_receipts={}\nvalidator_attestations_submitted={}\nlocal_producer={local_producer}\nlisten={}\np2p_listen={}\np2p_runtime=libp2p\np2p_peer_id={p2p_peer_id}\np2p_connected_peers={}\np2p_observed_block_gossip_count={}\np2p_observed_job_gossip_count={}\np2p_observed_receipt_gossip_count={}\np2p_observed_attestation_gossip_count={}\np2p_latest_observed_block_height={}\np2p_latest_observed_block_hash={}\np2p_observed_block_hashes={}\np2p_gossipsub_topics={p2p_topics}\np2p_request_response_protocols={p2p_request_response_protocols}\np2p_bootstrap_peers={bootstrap_peer_count}\n{identity}\np2p_max_transmit_bytes={max_transmit_bytes}\np2p_request_timeout_seconds={request_timeout_seconds}\np2p_max_concurrent_streams={max_concurrent_streams}\np2p_idle_timeout_seconds={idle_timeout_seconds}\ndata_dir={}\nserved_requests={served_requests}\nproduced_blocks={produced_blocks}\nnetwork_applied_blocks={network_applied_blocks}\nnetwork_events_ingested={}\nnetwork_block_events_ingested={}\nnetwork_block_headers_ingested={}\nnetwork_job_events_ingested={}\nnetwork_job_payloads_ingested={}\nnetwork_job_payloads_applied={}\nnetwork_receipt_events_ingested={}\nnetwork_receipt_payloads_ingested={}\nnetwork_receipt_payloads_applied={}\nnetwork_attestation_events_ingested={}\nnetwork_attestation_payloads_ingested={}\nnetwork_attestation_payloads_applied={}\nnetwork_peer_events_ingested={}\nnetwork_invalid_events={}",
            self.config.runtime_command,
            self.config.role.label(),
            self.config.node.profile.label(),
            self.config.node.can_produce_local_blocks(),
            runtime_role_wallet_address_text(self.config.role_wallet_address),
            runtime_role_wallet_registration(
                self.config.role,
                self.config.role_wallet_address,
                &self.server.gateway().node.chain
            ),
            runtime_role_wallet_registered(
                self.config.role,
                self.config.role_wallet_address,
                &self.server.gateway().node.chain
            ),
            self.runtime_state.miner_work_ready(),
            self.runtime_state.miner_assigned_jobs_seen(),
            self.runtime_state.miner_unreceipted_jobs(),
            self.runtime_state.miner_receipts_submitted(),
            self.runtime_state.miner_tensors_inserted(),
            self.runtime_state.validator_work_ready(),
            self.runtime_state.validator_assigned_receipts_seen(),
            self.runtime_state.validator_unattested_receipts(),
            self.runtime_state.validator_artifact_ready_receipts(),
            self.runtime_state.validator_artifact_missing_receipts(),
            self.runtime_state.validator_attestations_submitted(),
            network.rpc_listen,
            network.p2p_listen,
            self.p2p_service.connected_peer_count(),
            self.p2p_service.observed_block_gossip_count(),
            self.p2p_service.observed_job_gossip_count(),
            self.p2p_service.observed_receipt_gossip_count(),
            self.p2p_service.observed_attestation_gossip_count(),
            self.p2p_service.latest_observed_block_height(),
            hex(&self.p2p_service.latest_observed_block_hash()),
            hex_hash_list(&self.p2p_service.observed_block_hashes()),
            self.config.node.data_dir().display(),
            network_events.events,
            network_events.block_announcements,
            network_events.block_headers,
            network_events.jobs,
            network_events.job_payloads,
            network_events.job_payloads_applied,
            network_events.receipts,
            network_events.receipt_payloads,
            network_events.receipt_payloads_applied,
            network_events.attestations,
            network_events.attestation_payloads,
            network_events.attestation_payloads_applied,
            network_events.peers,
            network_events.invalid_events,
            local_producer = self.local_producer,
            p2p_peer_id = self.p2p_peer_id,
            p2p_topics = self.p2p_topics,
            p2p_request_response_protocols = self.p2p_request_response_protocols,
            bootstrap_peer_count = self.bootstrap_peer_count,
            identity = self.identity,
            max_transmit_bytes = self.max_transmit_bytes,
            request_timeout_seconds = self.request_timeout_seconds,
            max_concurrent_streams = self.max_concurrent_streams,
            idle_timeout_seconds = self.idle_timeout_seconds,
            served_requests = self.runtime_state.served_requests(),
            produced_blocks = self.runtime_state.produced_blocks(),
            network_applied_blocks = self.runtime_state.network_applied_blocks()
        )
    }
}

struct RoleRuntimeStatusSnapshot {
    served_requests: usize,
    produced_blocks: usize,
    network_applied_blocks: usize,
    local_producer: bool,
    latest_height: u64,
    p2p_connected_peers: usize,
    p2p_observed_blocks: usize,
    p2p_observed_jobs: usize,
    p2p_observed_receipts: usize,
    p2p_observed_attestations: usize,
    p2p_latest_observed_block_height: u64,
    p2p_latest_observed_block_hash: [u8; 32],
    p2p_observed_block_hashes: Vec<[u8; 32]>,
    network_events: NetworkEventIngest,
    role_wallet_address: Option<Address>,
    role_wallet_registration: &'static str,
    role_wallet_registered: bool,
    miner_work_ready: bool,
    miner_assigned_jobs_seen: usize,
    miner_unreceipted_jobs: usize,
    miner_receipts_submitted: usize,
    miner_tensors_inserted: usize,
    validator_work_ready: bool,
    validator_assigned_receipts_seen: usize,
    validator_unattested_receipts: usize,
    validator_artifact_ready_receipts: usize,
    validator_artifact_missing_receipts: usize,
    validator_attestations_submitted: usize,
}

impl RoleRuntimeStatusSnapshot {
    fn from_runtime_state(
        state: &NodeRuntimeState,
        server: &RpcHttpServer,
        p2p_service: &TensorVmLibp2pService,
        local_producer: bool,
        role: RuntimeRole,
        role_wallet_address: Option<Address>,
    ) -> Self {
        let chain = &server.gateway().node.chain;
        Self {
            served_requests: state.served_requests(),
            produced_blocks: state.produced_blocks(),
            network_applied_blocks: state.network_applied_blocks(),
            local_producer,
            latest_height: server.gateway().node.chain.state.height,
            p2p_connected_peers: p2p_service.connected_peer_count(),
            p2p_observed_blocks: p2p_service.observed_block_gossip_count(),
            p2p_observed_jobs: p2p_service.observed_job_gossip_count(),
            p2p_observed_receipts: p2p_service.observed_receipt_gossip_count(),
            p2p_observed_attestations: p2p_service.observed_attestation_gossip_count(),
            p2p_latest_observed_block_height: p2p_service.latest_observed_block_height(),
            p2p_latest_observed_block_hash: p2p_service.latest_observed_block_hash(),
            p2p_observed_block_hashes: p2p_service.observed_block_hashes(),
            network_events: state.network_events(),
            role_wallet_address,
            role_wallet_registration: runtime_role_wallet_registration(
                role,
                role_wallet_address,
                chain,
            ),
            role_wallet_registered: runtime_role_wallet_registered(
                role,
                role_wallet_address,
                chain,
            ),
            miner_work_ready: state.miner_work_ready(),
            miner_assigned_jobs_seen: state.miner_assigned_jobs_seen(),
            miner_unreceipted_jobs: state.miner_unreceipted_jobs(),
            miner_receipts_submitted: state.miner_receipts_submitted(),
            miner_tensors_inserted: state.miner_tensors_inserted(),
            validator_work_ready: state.validator_work_ready(),
            validator_assigned_receipts_seen: state.validator_assigned_receipts_seen(),
            validator_unattested_receipts: state.validator_unattested_receipts(),
            validator_artifact_ready_receipts: state.validator_artifact_ready_receipts(),
            validator_artifact_missing_receipts: state.validator_artifact_missing_receipts(),
            validator_attestations_submitted: state.validator_attestations_submitted(),
        }
    }
}

fn write_role_runtime_status(
    config: &ServiceRuntimeConfig,
    snapshot: &RoleRuntimeStatusSnapshot,
) -> std::result::Result<(), String> {
    let path = config.node.data_dir().join("role-runtime.status");
    let contents = format!(
        "role_runtime_command={}\nrole_loop_role={}\nrole_loop_ready=true\nrole_chain_profile={}\nrole_can_produce_blocks={}\nrole_wallet_address={}\nrole_wallet_registration={}\nrole_wallet_registered={}\nrole_miner_work_ready={}\nrole_miner_assigned_jobs_seen={}\nrole_miner_unreceipted_jobs={}\nrole_miner_receipts_submitted={}\nrole_miner_tensors_inserted={}\nrole_validator_work_ready={}\nrole_validator_assigned_receipts_seen={}\nrole_validator_unattested_receipts={}\nrole_validator_artifact_ready_receipts={}\nrole_validator_artifact_missing_receipts={}\nrole_validator_attestations_submitted={}\nrole_local_producer={}\nrole_served_requests={}\nrole_produced_blocks={}\nrole_network_applied_blocks={}\nrole_network_events_ingested={}\nrole_network_block_events_ingested={}\nrole_network_block_headers_ingested={}\nrole_network_job_events_ingested={}\nrole_network_job_payloads_ingested={}\nrole_network_job_payloads_applied={}\nrole_network_receipt_events_ingested={}\nrole_network_receipt_payloads_ingested={}\nrole_network_receipt_payloads_applied={}\nrole_network_attestation_events_ingested={}\nrole_network_attestation_payloads_ingested={}\nrole_network_attestation_payloads_applied={}\nrole_network_peer_events_ingested={}\nrole_network_invalid_events={}\nrole_latest_height={}\nrole_p2p_connected_peers={}\nrole_p2p_observed_blocks={}\nrole_p2p_observed_jobs={}\nrole_p2p_observed_receipts={}\nrole_p2p_observed_attestations={}\nrole_p2p_latest_observed_block_height={}\nrole_p2p_latest_observed_block_hash={}\nrole_p2p_observed_block_hashes={}\n",
        config.runtime_command,
        config.role.label(),
        config.node.profile.label(),
        config.node.can_produce_local_blocks(),
        runtime_role_wallet_address_text(snapshot.role_wallet_address),
        snapshot.role_wallet_registration,
        snapshot.role_wallet_registered,
        snapshot.miner_work_ready,
        snapshot.miner_assigned_jobs_seen,
        snapshot.miner_unreceipted_jobs,
        snapshot.miner_receipts_submitted,
        snapshot.miner_tensors_inserted,
        snapshot.validator_work_ready,
        snapshot.validator_assigned_receipts_seen,
        snapshot.validator_unattested_receipts,
        snapshot.validator_artifact_ready_receipts,
        snapshot.validator_artifact_missing_receipts,
        snapshot.validator_attestations_submitted,
        snapshot.local_producer,
        snapshot.served_requests,
        snapshot.produced_blocks,
        snapshot.network_applied_blocks,
        snapshot.network_events.events,
        snapshot.network_events.block_announcements,
        snapshot.network_events.block_headers,
        snapshot.network_events.jobs,
        snapshot.network_events.job_payloads,
        snapshot.network_events.job_payloads_applied,
        snapshot.network_events.receipts,
        snapshot.network_events.receipt_payloads,
        snapshot.network_events.receipt_payloads_applied,
        snapshot.network_events.attestations,
        snapshot.network_events.attestation_payloads,
        snapshot.network_events.attestation_payloads_applied,
        snapshot.network_events.peers,
        snapshot.network_events.invalid_events,
        snapshot.latest_height,
        snapshot.p2p_connected_peers,
        snapshot.p2p_observed_blocks,
        snapshot.p2p_observed_jobs,
        snapshot.p2p_observed_receipts,
        snapshot.p2p_observed_attestations,
        snapshot.p2p_latest_observed_block_height,
        hex(&snapshot.p2p_latest_observed_block_hash),
        hex_hash_list(&snapshot.p2p_observed_block_hashes)
    );
    std::fs::write(&path, contents).map_err(|error| {
        format!(
            "failed to write role runtime status {}: {error}",
            path.display()
        )
    })
}

struct NetworkCatchup {
    chain: LocalChain,
    tensors: Vec<Tensor>,
    applied_blocks: usize,
}

fn ingest_network_events(
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    local_producer: bool,
    pending_payloads: &mut PendingNetworkPayloads,
) -> std::result::Result<NetworkEventIngest, String> {
    let messages = p2p_service.drain_observed_messages();
    let mut context = RuntimeNetworkEventContext {
        server,
        p2p_service,
    };
    ingest_network_messages(&mut context, messages, local_producer, pending_payloads)
}

struct RuntimeNetworkEventContext<'a> {
    server: &'a mut RpcHttpServer,
    p2p_service: &'a TensorVmLibp2pService,
}

impl NetworkEventContext for RuntimeNetworkEventContext<'_> {
    fn chain(&mut self) -> &mut LocalChain {
        &mut self.server.gateway_mut().node.chain
    }

    fn apply_block_header(
        &mut self,
        height: u64,
        block_hash: Hash,
    ) -> std::result::Result<usize, String> {
        catch_up_to_announced_block(self.server, self.p2p_service, height, block_hash)
    }
}

fn catch_up_to_announced_block(
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    target_height: u64,
    target_hash: Hash,
) -> std::result::Result<usize, String> {
    if target_height == 0
        || target_hash == [0; 32]
        || chain_contains_block_hash(&server.gateway().node.chain, &target_hash)
    {
        return Ok(0);
    }
    let Some(catchup) = replay_synthetic_rounds_to_observed_block(
        &server.gateway().node.chain,
        target_height,
        target_hash,
    )?
    else {
        return Ok(0);
    };
    let announcement_checkpoint = chain_announcement_checkpoint(&server.gateway().node.chain);
    server.gateway_mut().node.chain = catchup.chain;
    for tensor in catchup.tensors {
        server.gateway_mut().node.insert_tensor(tensor);
    }
    if let Some(block) = block_at_height(&server.gateway().node.chain, target_height) {
        publish_block_announcement(p2p_service, block.height, block.hash())?;
    }
    publish_new_chain_announcements(
        p2p_service,
        &announcement_checkpoint,
        &server.gateway().node.chain,
    )?;
    Ok(catchup.applied_blocks)
}

fn replay_synthetic_rounds_to_observed_block(
    chain: &LocalChain,
    target_height: u64,
    target_hash: Hash,
) -> std::result::Result<Option<NetworkCatchup>, String> {
    let latest_height = chain
        .blocks
        .last()
        .map(|block| block.height)
        .unwrap_or_default();
    if target_height <= latest_height {
        return Ok(None);
    }
    let mut candidate = chain.clone();
    prune_future_synthetic_state_for_replay(&mut candidate, latest_height);
    let mut tensors = Vec::new();
    let mut applied_blocks = 0usize;
    let max_replay_blocks = target_height.saturating_sub(latest_height).min(128) as usize;
    for _ in 0..max_replay_blocks {
        let Some(round) = produce_synthetic_cpu_round_with_tensors(&mut candidate)
            .map_err(|error| format!("synthetic CPU catch-up failed: {error}"))?
        else {
            break;
        };
        tensors.extend(round.tensors);
        applied_blocks = applied_blocks.saturating_add(1);
        if block_at_height(&candidate, target_height).is_some() {
            break;
        }
    }
    let Some(block) = block_at_height(&candidate, target_height) else {
        return Ok(None);
    };
    if block.hash() != target_hash {
        return Ok(None);
    }
    Ok(Some(NetworkCatchup {
        chain: candidate,
        tensors,
        applied_blocks,
    }))
}

fn prune_future_synthetic_state_for_replay(chain: &mut LocalChain, latest_height: u64) {
    let receipt_window = chain.params.receipt_submission_window;
    let future_job_ids = chain
        .state
        .jobs
        .iter()
        .filter_map(|(job_id, job)| {
            let is_future_synthetic_job = synthetic_job_submission_height(job, receipt_window)
                .map(|height| height > latest_height)
                .unwrap_or(false);
            is_future_synthetic_job.then_some(*job_id)
        })
        .collect::<BTreeSet<_>>();
    let future_receipt_ids = chain
        .state
        .receipts
        .iter()
        .filter_map(|(receipt_id, receipt)| {
            future_job_ids
                .contains(&receipt.job_id())
                .then_some(*receipt_id)
        })
        .collect::<BTreeSet<_>>();
    let mut future_valid_attestations_by_validator = BTreeMap::<_, u64>::new();
    for receipt_id in &future_receipt_ids {
        if let Some(attestations) = chain.state.attestations.get(receipt_id) {
            for attestation in attestations {
                if attestation.result == VerificationResult::Valid
                    && attestation.data_availability_passed
                {
                    *future_valid_attestations_by_validator
                        .entry(attestation.validator)
                        .or_default() += 1;
                }
            }
        }
    }
    chain.state.jobs.retain(|_, job| {
        synthetic_job_submission_height(job, receipt_window)
            .map(|height| height <= latest_height)
            .unwrap_or(true)
    });
    chain
        .state
        .receipts
        .retain(|receipt_id, _| !future_receipt_ids.contains(receipt_id));
    for receipt_id in future_receipt_ids {
        chain.state.attestations.remove(&receipt_id);
        chain.state.settled_receipts.remove(&receipt_id);
    }
    for (validator, count) in future_valid_attestations_by_validator {
        if let Some(validator) = chain.state.validators.get_mut(&validator) {
            validator.valid_attestations = validator.valid_attestations.saturating_sub(count);
        }
    }
}

fn synthetic_job_submission_height(job: &tensor_vm::JobState, receipt_window: u64) -> Option<u64> {
    match job {
        tensor_vm::JobState::TensorOp(job) => job.deadline_block.checked_sub(receipt_window),
        tensor_vm::JobState::LinearTrainingStep(job) => {
            job.deadline_block.checked_sub(receipt_window)
        }
    }
}

fn produce_and_publish_synthetic_round(
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    profile: &ChainProfile,
) -> std::result::Result<Option<Hash>, String> {
    let announcement_checkpoint = chain_announcement_checkpoint(&server.gateway().node.chain);
    if server
        .gateway_mut()
        .node
        .produce_synthetic_cpu_round_with_profile(profile)
        .map_err(|error| format!("synthetic CPU round failed: {error}"))?
        .is_none()
    {
        return Ok(None);
    }
    let Some(block) = server.gateway().node.chain.blocks.last() else {
        return Ok(None);
    };
    let block_hash = block.hash();
    publish_block_announcement(p2p_service, block.height, block_hash)?;
    publish_new_chain_announcements(
        p2p_service,
        &announcement_checkpoint,
        &server.gateway().node.chain,
    )?;
    Ok(Some(block_hash))
}

fn publish_block_announcement(
    p2p_service: &TensorVmLibp2pService,
    height: u64,
    block_hash: Hash,
) -> std::result::Result<(), String> {
    p2p_service
        .publish_gossip(P2pMessage::NewBlockHeader { height, block_hash })
        .map_err(|error| format!("failed to publish block gossip: {error}"))
}

fn block_at_height(chain: &LocalChain, height: u64) -> Option<&tensor_vm::chain::TensorBlock> {
    chain.blocks.iter().find(|block| block.height == height)
}

fn chain_contains_block_hash(chain: &LocalChain, block_hash: &Hash) -> bool {
    chain.blocks.iter().any(|block| block.hash() == *block_hash)
}

struct ChainAnnouncementCheckpoint {
    jobs: BTreeSet<Hash>,
    receipts: BTreeSet<Hash>,
    attestations: BTreeSet<Hash>,
}

fn chain_announcement_checkpoint(chain: &LocalChain) -> ChainAnnouncementCheckpoint {
    ChainAnnouncementCheckpoint {
        jobs: chain.state.jobs.keys().copied().collect(),
        receipts: chain.state.receipts.keys().copied().collect(),
        attestations: attestation_announcement_hashes(chain).collect(),
    }
}

fn publish_new_chain_announcements(
    p2p_service: &TensorVmLibp2pService,
    before: &ChainAnnouncementCheckpoint,
    chain: &LocalChain,
) -> std::result::Result<(), String> {
    for (job_id, job) in &chain.state.jobs {
        if !before.jobs.contains(job_id) {
            p2p_service
                .publish_gossip(P2pMessage::NewJobPayload {
                    job_id: *job_id,
                    payload: encode_job_payload(job),
                })
                .map_err(|error| format!("failed to publish job payload gossip: {error}"))?;
            p2p_service
                .publish_gossip(P2pMessage::NewJob(*job_id))
                .map_err(|error| format!("failed to publish job gossip: {error}"))?;
        }
    }
    for (receipt_id, receipt) in &chain.state.receipts {
        if !before.receipts.contains(receipt_id) {
            p2p_service
                .publish_gossip(P2pMessage::NewReceiptPayload {
                    receipt_id: *receipt_id,
                    payload: encode_receipt_payload(receipt),
                })
                .map_err(|error| format!("failed to publish receipt payload gossip: {error}"))?;
            p2p_service
                .publish_gossip(P2pMessage::NewReceipt(*receipt_id))
                .map_err(|error| format!("failed to publish receipt gossip: {error}"))?;
        }
    }
    for attestation in chain
        .state
        .attestations
        .values()
        .flat_map(|attestations| attestations.iter())
    {
        let attestation_id = attestation_announcement_hash(attestation);
        if !before.attestations.contains(&attestation_id) {
            p2p_service
                .publish_gossip(P2pMessage::NewAttestationPayload {
                    attestation_id,
                    payload: encode_attestation_payload(attestation),
                })
                .map_err(|error| {
                    format!("failed to publish attestation payload gossip: {error}")
                })?;
            p2p_service
                .publish_gossip(P2pMessage::NewAttestation(attestation_id))
                .map_err(|error| format!("failed to publish attestation gossip: {error}"))?;
        }
    }
    Ok(())
}

fn attestation_announcement_hashes(chain: &LocalChain) -> impl Iterator<Item = Hash> + '_ {
    chain
        .state
        .attestations
        .values()
        .flat_map(|attestations| attestations.iter().map(attestation_announcement_hash))
}

fn hex_hash_list(hashes: &[[u8; 32]]) -> String {
    if hashes.is_empty() {
        return "none".to_owned();
    }
    hashes
        .iter()
        .map(|hash| hex(hash))
        .collect::<Vec<_>>()
        .join(",")
}

fn runtime_block_interval() -> Option<Duration> {
    std::env::var("TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .map(Duration::from_millis)
}

fn runtime_local_block_producer() -> bool {
    match std::env::var("TENSORVM_LOCAL_CPU_ROLE_PRODUCER") {
        Ok(value) => matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
        Err(_) => std::env::var("TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS").is_ok(),
    }
}

fn local_cpu_seed_beacon() -> [u8; 32] {
    hash_bytes(b"tensor-vm-local-cpu-compose-seed", &[b"shared-chain-base"])
}

fn ready_file_field(data_dir: &str, key: &str) -> String {
    let path = Path::new(data_dir).join("local-cpu-ready");
    status_file_field(&path, key)
}

fn role_runtime_status_field(data_dir: &str, key: &str) -> String {
    let path = Path::new(data_dir).join("role-runtime.status");
    status_file_field(&path, key)
}

fn status_file_field(path: &Path, key: &str) -> String {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| {
            contents.lines().find_map(|line| {
                let value = line.strip_prefix(key)?.strip_prefix('=')?;
                Some(value.to_owned())
            })
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

fn p2p_identity_report(identity_seed: Option<[u8; 32]>) -> String {
    match identity_seed {
        Some(seed) => format!("p2p_identity_seeded=true\np2p_identity_seed={}", hex(&seed)),
        None => "p2p_identity_seeded=false".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tensor_vm::{
        ChainCommand, ChainEngine, ChainNetworkPayloadProcessor, NetworkPayloadApply,
        network_ingest_order,
        node::{
            apply_network_attestation_payload, apply_network_job_payload,
            apply_network_receipt_payload,
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
        let mut chain = LocalChain::new(hash_bytes(b"test", &[b"service-init-recovery"]));
        let miner = address(b"service-init-recovery-miner");
        chain
            .register_miner(miner, chain.params.miner_min_stake)
            .unwrap();
        chain.produce_block(miner, 1_000);
        chain.produce_block(miner, 1_006);
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
    fn network_catchup_replays_only_matching_observed_heads() {
        let mut testnet = LocalTestnet::new(TestnetConfig::default(), local_cpu_seed_beacon());
        let scheduler = JobScheduler::with_small_shape((8, 8, 8));
        testnet.run_matmul_round(&scheduler);
        testnet.run_linear_training_round(&scheduler);
        let seed_chain = testnet.chain.clone();

        let mut announced_chain = seed_chain.clone();
        produce_synthetic_cpu_round_with_tensors(&mut announced_chain)
            .unwrap()
            .expect("local synthetic round must advance");
        let announced_block = announced_chain
            .blocks
            .last()
            .expect("announced chain must contain a head block");
        let catchup = replay_synthetic_rounds_to_observed_block(
            &seed_chain,
            announced_block.height,
            announced_block.hash(),
        )
        .unwrap()
        .expect("matching observed head must replay");

        assert_eq!(catchup.applied_blocks, 1);
        assert_eq!(catchup.chain.blocks, announced_chain.blocks);
        assert!(!catchup.tensors.is_empty());
        assert!(
            replay_synthetic_rounds_to_observed_block(
                &seed_chain,
                announced_block.height,
                hash_bytes(b"test", &[b"wrong-network-head"]),
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn network_catchup_replays_after_future_job_payloads_were_applied() {
        let mut testnet = LocalTestnet::new(TestnetConfig::default(), local_cpu_seed_beacon());
        let scheduler = JobScheduler::with_small_shape((8, 8, 8));
        testnet.run_matmul_round(&scheduler);
        testnet.run_linear_training_round(&scheduler);
        let seed_chain = testnet.chain.clone();

        let mut announced_chain = seed_chain.clone();
        produce_synthetic_cpu_round_with_tensors(&mut announced_chain)
            .unwrap()
            .expect("first local synthetic round must advance");
        produce_synthetic_cpu_round_with_tensors(&mut announced_chain)
            .unwrap()
            .expect("second local synthetic round must advance");

        let mut polluted_chain = seed_chain.clone();
        for job in announced_chain
            .state
            .jobs
            .iter()
            .filter(|(job_id, _)| !seed_chain.state.jobs.contains_key(*job_id))
            .map(|(_, job)| job.clone())
        {
            polluted_chain
                .apply_command(ChainCommand::SubmitJob(job))
                .unwrap();
        }
        assert!(polluted_chain.state.jobs.len() > seed_chain.state.jobs.len());

        let announced_block = announced_chain
            .blocks
            .last()
            .expect("announced chain must contain a head block");
        let catchup = replay_synthetic_rounds_to_observed_block(
            &polluted_chain,
            announced_block.height,
            announced_block.hash(),
        )
        .unwrap()
        .expect("matching observed head must replay after pruning future payloads");

        assert_eq!(catchup.applied_blocks, 2);
        assert_eq!(catchup.chain.blocks, announced_chain.blocks);
        assert_eq!(catchup.chain.state.jobs, announced_chain.state.jobs);
    }

    #[test]
    fn network_catchup_prunes_future_receipt_payloads_before_replay() {
        let mut testnet = LocalTestnet::new(TestnetConfig::default(), local_cpu_seed_beacon());
        let scheduler = JobScheduler::with_small_shape((8, 8, 8));
        testnet.run_matmul_round(&scheduler);
        testnet.run_linear_training_round(&scheduler);
        let seed_chain = testnet.chain.clone();

        let mut announced_chain = seed_chain.clone();
        produce_synthetic_cpu_round_with_tensors(&mut announced_chain)
            .unwrap()
            .expect("first local synthetic round must advance");
        produce_synthetic_cpu_round_with_tensors(&mut announced_chain)
            .unwrap()
            .expect("second local synthetic round must advance");

        let mut polluted_chain = seed_chain.clone();
        for job in announced_chain
            .state
            .jobs
            .iter()
            .filter(|(job_id, _)| !seed_chain.state.jobs.contains_key(*job_id))
            .map(|(_, job)| job.clone())
        {
            polluted_chain
                .apply_command(ChainCommand::SubmitJob(job))
                .unwrap();
        }
        for receipt in announced_chain
            .state
            .receipts
            .iter()
            .filter(|(receipt_id, _)| !seed_chain.state.receipts.contains_key(*receipt_id))
            .map(|(_, receipt)| receipt.clone())
        {
            polluted_chain
                .apply_command(ChainCommand::SubmitReceipt(receipt))
                .unwrap();
        }
        for attestation in announced_chain
            .state
            .attestations
            .iter()
            .filter(|(receipt_id, _)| !seed_chain.state.attestations.contains_key(*receipt_id))
            .flat_map(|(_, attestations)| attestations.iter().cloned())
        {
            polluted_chain
                .apply_command(ChainCommand::SubmitAttestation(attestation))
                .unwrap();
        }
        assert!(polluted_chain.state.receipts.len() > seed_chain.state.receipts.len());

        let announced_block = announced_chain
            .blocks
            .last()
            .expect("announced chain must contain a head block");
        let catchup = replay_synthetic_rounds_to_observed_block(
            &polluted_chain,
            announced_block.height,
            announced_block.hash(),
        )
        .unwrap()
        .expect("matching observed head must replay after pruning future role payloads");

        assert_eq!(catchup.applied_blocks, 2);
        assert_eq!(catchup.chain.blocks, announced_chain.blocks);
        assert_eq!(catchup.chain.state.receipts, announced_chain.state.receipts);
        assert_eq!(
            catchup.chain.state.attestations,
            announced_chain.state.attestations
        );
    }

    #[test]
    fn network_event_ingest_accumulates_runtime_counters() {
        let mut cumulative = NetworkEventIngest {
            events: 2,
            block_announcements: 1,
            block_headers: 1,
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
    }

    #[test]
    fn runtime_role_policy_blocks_miner_and_validator_local_production() {
        let profile = ChainProfile::local_cpu();
        assert!(
            NodeConfig::new(profile.clone(), RuntimeRole::Service.node_role(), "service")
                .can_produce_local_blocks()
        );
        assert!(
            NodeConfig::new(
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
            !NodeConfig::new(profile, RuntimeRole::Validator.node_role(), "validator")
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
                matches!(role, RuntimeRole::Proposer)
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

        let validator_report =
            RoleRunLoop::validator().format_report(config, "service_report=true");
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
        let mut chain = LocalChain::new(hash_bytes(b"test", &[b"role-wallet-registration"]));
        let miner = address(b"runtime-wallet-miner");
        let validator = address(b"runtime-wallet-validator");
        let unknown = address(b"runtime-wallet-unknown");
        chain
            .register_miner(miner, chain.params.miner_min_stake)
            .unwrap();
        chain
            .register_validator(validator, chain.params.validator_min_stake)
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
            "miner"
        );
        assert!(runtime_role_wallet_registered(
            RuntimeRole::Proposer,
            Some(miner),
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
        let mut chain = LocalChain::new(hash_bytes(b"test", &[b"miner-work-observation"]));
        let miner = address(b"miner-work-observation-miner");
        chain
            .register_miner(miner, chain.params.miner_min_stake)
            .unwrap();
        let scheduler = JobScheduler::with_small_shape((2, 2, 2));
        let job = scheduler.generate_small_matmul(
            chain.state.epoch,
            chain.state.height,
            &chain.state.finalized_randomness,
            chain
                .state
                .height
                .saturating_add(chain.params.receipt_submission_window),
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
            .execute_job(&job_state, chain.state.height, 1)
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
        let mut chain = LocalChain::new(hash_bytes(b"test", &[b"miner-work-unassigned"]));
        let miner = address(b"miner-work-assigned");
        let unassigned = address(b"miner-work-unassigned");
        chain
            .register_miner(miner, chain.params.miner_min_stake)
            .unwrap();
        let scheduler = JobScheduler::with_small_shape((2, 2, 2));
        let job = scheduler.generate_small_matmul(
            chain.state.epoch,
            chain.state.height,
            &chain.state.finalized_randomness,
            chain
                .state
                .height
                .saturating_add(chain.params.receipt_submission_window),
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
        let mut chain = LocalChain::new(hash_bytes(b"test", &[b"miner-receipt-submit"]));
        let miner = address(b"miner-receipt-submit-miner");
        chain
            .register_miner(miner, chain.params.miner_min_stake)
            .unwrap();
        let scheduler = JobScheduler::with_small_shape((2, 2, 2));
        let job = scheduler.generate_small_matmul(
            chain.state.epoch,
            chain.state.height,
            &chain.state.finalized_randomness,
            chain
                .state
                .height
                .saturating_add(chain.params.receipt_submission_window),
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
        assert_eq!(node.chain.state.receipts.len(), 1);
        let receipt = node
            .chain
            .state
            .receipts
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
        let mut chain = LocalChain::new(hash_bytes(b"test", &[b"miner-receipt-skip"]));
        chain.params.replication_factor = 1;
        let miner_a = address(b"miner-receipt-skip-a");
        let miner_b = address(b"miner-receipt-skip-b");
        let unknown = address(b"miner-receipt-skip-unknown");
        chain
            .register_miner(miner_a, chain.params.miner_min_stake)
            .unwrap();
        chain
            .register_miner(miner_b, chain.params.miner_min_stake)
            .unwrap();
        let scheduler = JobScheduler::with_small_shape((2, 2, 2));
        let job = scheduler.generate_small_matmul(
            chain.state.epoch,
            chain.state.height,
            &chain.state.finalized_randomness,
            chain
                .state
                .height
                .saturating_add(chain.params.receipt_submission_window),
        );
        let job_id = job.job_id;
        chain
            .apply_command(ChainCommand::SubmitJob(tensor_vm::JobState::TensorOp(job)))
            .unwrap();
        let assignment = JobScheduler::with_small_shape((8, 8, 8)).assign_miners(
            &chain,
            job_id,
            &chain.state.finalized_randomness,
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
        assert_eq!(node.chain.state.receipts.len(), 0);

        assert!(
            submit_miner_role_receipt(&mut node, assigned, job_id)
                .unwrap()
                .is_some()
        );
        assert_eq!(node.chain.state.receipts.len(), 1);
        assert_tensor_count(&node, 3);
        assert!(
            submit_miner_role_receipt(&mut node, assigned, job_id)
                .unwrap()
                .is_none()
        );
        assert_eq!(node.chain.state.receipts.len(), 1);
        assert_tensor_count(&node, 3);
    }

    #[test]
    fn validator_role_work_observation_tracks_assigned_unattested_receipts() {
        let mut chain = LocalChain::new(hash_bytes(b"test", &[b"validator-work-observation"]));
        let miner = address(b"validator-work-miner");
        let validator = address(b"validator-work-validator");
        chain
            .register_miner(miner, chain.params.miner_min_stake)
            .unwrap();
        chain
            .register_validator(validator, chain.params.validator_min_stake)
            .unwrap();
        let scheduler = JobScheduler::with_small_shape((2, 2, 2));
        let job = scheduler.generate_small_matmul(
            chain.state.epoch,
            chain.state.height,
            &chain.state.finalized_randomness,
            chain
                .state
                .height
                .saturating_add(chain.params.receipt_submission_window),
        );
        let job_state = tensor_vm::JobState::TensorOp(job);
        chain
            .apply_command(ChainCommand::SubmitJob(job_state.clone()))
            .unwrap();
        let bundle = CpuReferenceMinerRole::new(miner)
            .execute_job(&job_state, chain.state.height, 1)
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
    fn validator_role_attestation_submission_skips_missing_unregistered_unassigned_and_duplicates()
    {
        let mut chain = LocalChain::new(hash_bytes(b"test", &[b"validator-attestation-skip"]));
        chain.params.freivalds.validators_per_job = 1;
        let miner = address(b"validator-attestation-miner");
        let validator_a = address(b"validator-attestation-a");
        let validator_b = address(b"validator-attestation-b");
        let unknown = address(b"validator-attestation-unknown");
        chain
            .register_miner(miner, chain.params.miner_min_stake)
            .unwrap();
        chain
            .register_validator(validator_a, chain.params.validator_min_stake)
            .unwrap();
        chain
            .register_validator(validator_b, chain.params.validator_min_stake)
            .unwrap();
        let scheduler = JobScheduler::with_small_shape((2, 2, 2));
        let job = scheduler.generate_small_matmul(
            chain.state.epoch,
            chain.state.height,
            &chain.state.finalized_randomness,
            chain
                .state
                .height
                .saturating_add(chain.params.receipt_submission_window),
        );
        let job_state = tensor_vm::JobState::TensorOp(job);
        chain
            .apply_command(ChainCommand::SubmitJob(job_state.clone()))
            .unwrap();
        let bundle = CpuReferenceMinerRole::new(miner)
            .execute_job(&job_state, chain.state.height, 1)
            .unwrap();
        let receipt_id = bundle.receipt_id();
        chain
            .apply_command(ChainCommand::SubmitReceipt(bundle.receipt.clone()))
            .unwrap();
        let assignment = JobScheduler::with_small_shape((8, 8, 8)).assign_validators(
            &chain,
            receipt_id,
            &chain.state.finalized_randomness,
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
        assert!(!node.chain.state.attestations.contains_key(&receipt_id));

        insert_bundle_tensors(&mut node, &bundle);
        let submission = submit_validator_role_attestation(&mut node, assigned, receipt_id)
            .unwrap()
            .expect("assigned validator with local tensors should submit attestation");
        assert_eq!(submission.attestations_submitted, 1);
        let attestations = node
            .chain
            .state
            .attestations
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
        assert_eq!(node.chain.state.attestations[&receipt_id].len(), 1);
        let observation = validator_role_work_observation(&node, assigned);
        assert_eq!(observation.assigned_receipts, BTreeSet::from([receipt_id]));
        assert!(observation.unattested_receipts.is_empty());
        assert!(observation.artifact_ready_receipts.is_empty());
        assert!(observation.artifact_missing_receipts.is_empty());
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

    fn test_rpc_server(chain: LocalChain) -> RpcHttpServer {
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

    #[test]
    fn network_payload_application_defers_out_of_order_receipts_and_attestations() {
        let mut testnet = LocalTestnet::new(TestnetConfig::default(), local_cpu_seed_beacon());
        let scheduler = JobScheduler::with_small_shape((8, 8, 8));
        testnet.run_matmul_round(&scheduler);
        let receipt = testnet
            .chain
            .state
            .receipts
            .values()
            .next()
            .expect("local round must produce a receipt")
            .clone();
        let receipt_id = receipt.receipt_id();
        let attestation = testnet
            .chain
            .state
            .attestations
            .values()
            .flat_map(|items| items.iter())
            .next()
            .expect("local round must produce an attestation")
            .clone();
        let attestation_id = attestation_announcement_hash(&attestation);

        let mut missing_job_chain = testnet.chain.clone();
        missing_job_chain.state.jobs.remove(&receipt.job_id());
        missing_job_chain.state.receipts.remove(&receipt_id);
        let mut missing_job_server = test_rpc_server(missing_job_chain);
        assert_eq!(
            apply_network_receipt_payload(
                &mut missing_job_server.gateway_mut().node.chain,
                receipt_id,
                &encode_receipt_payload(&receipt),
            ),
            NetworkPayloadApply::Pending
        );

        let mut receipt_chain = testnet.chain.clone();
        receipt_chain.state.receipts.remove(&receipt_id);
        receipt_chain.state.attestations.remove(&receipt_id);
        let mut receipt_server = test_rpc_server(receipt_chain);
        assert_eq!(
            apply_network_receipt_payload(
                &mut receipt_server.gateway_mut().node.chain,
                receipt_id,
                &encode_receipt_payload(&receipt),
            ),
            NetworkPayloadApply::Applied
        );

        let mut missing_receipt_chain = testnet.chain.clone();
        missing_receipt_chain
            .state
            .receipts
            .remove(&attestation.receipt_id);
        missing_receipt_chain
            .state
            .attestations
            .remove(&attestation.receipt_id);
        let mut missing_receipt_server = test_rpc_server(missing_receipt_chain);
        assert_eq!(
            apply_network_attestation_payload(
                &mut missing_receipt_server.gateway_mut().node.chain,
                attestation_id,
                &encode_attestation_payload(&attestation),
            ),
            NetworkPayloadApply::Pending
        );

        let mut attestation_chain = testnet.chain.clone();
        attestation_chain
            .state
            .attestations
            .remove(&attestation.receipt_id);
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
            .state
            .jobs
            .values()
            .next()
            .expect("local round must produce a job")
            .clone();
        let job_id = job.job_id();
        let receipt = testnet
            .chain
            .state
            .receipts
            .values()
            .next()
            .expect("local round must produce a receipt")
            .clone();
        let receipt_id = receipt.receipt_id();
        let attestation = testnet
            .chain
            .state
            .attestations
            .values()
            .flat_map(|items| items.iter())
            .next()
            .expect("local round must produce an attestation")
            .clone();
        let attestation_id = attestation_announcement_hash(&attestation);

        let mut out_of_order_chain = testnet.chain.clone();
        out_of_order_chain.state.jobs.remove(&job_id);
        out_of_order_chain.state.receipts.remove(&receipt_id);
        out_of_order_chain.state.attestations.remove(&receipt_id);
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
            server.gateway().node.chain.state.receipts.get(&receipt_id),
            Some(&receipt)
        );
        assert_eq!(
            server
                .gateway()
                .node
                .chain
                .state
                .attestations
                .get(&receipt_id)
                .and_then(|items| items.first()),
            Some(&attestation)
        );
    }

    #[test]
    fn network_ingest_orders_block_announcements_before_payloads() {
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
            P2pMessage::NewJob(job_id),
            P2pMessage::NewBlock(block_hash),
        ]);

        assert!(matches!(messages[0], P2pMessage::NewBlockHeader { .. }));
        assert!(matches!(messages[1], P2pMessage::NewBlock(_)));
        assert!(matches!(messages[2], P2pMessage::NewJobPayload { .. }));
        assert!(matches!(messages[3], P2pMessage::NewReceipt(_)));
        assert!(matches!(messages[4], P2pMessage::NewJob(_)));
    }
}
