use std::{
    collections::{BTreeMap, BTreeSet},
    io::ErrorKind,
    path::Path,
    thread,
    time::{Duration, Instant},
};
use tensor_vm::{
    ChainCommand, ChainEngine, ChainProfile, CliCommand, Faucet, JobScheduler,
    Libp2pControlPlaneConfig, LocalChain, NetworkConfig, NetworkEventIngest, NetworkPayloadApply,
    NetworkPayloadProcessor, NodeConfig, NodeRole, NodeRuntimeState, NodeStore, PeerRecord,
    PendingNetworkPayloads, PrimitiveType, RpcGateway, RpcHttpServer, RpcNode, RpcPolicy, Tensor,
    TensorVmLibp2pService, TvmError, ValidatorAttestation, VerificationResult,
    api::P2pMessage,
    cli::{
        execute_reference_cli_command, validate_public_evidence_manifest,
        validate_public_testnet_preflight_manifest,
    },
    decode_attestation_payload, decode_job_payload, decode_receipt_payload,
    encode_attestation_payload, encode_job_payload, encode_receipt_payload,
    hash::hex,
    localnet::produce_synthetic_cpu_round_with_tensors,
    parse_cli_args, spawn_libp2p_service,
    testnet::{LocalTestnet, PublicTestnetCriteria, TestnetConfig},
    types::{Hash, hash_bytes},
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
        "command=service_status\ndata_dir={}\noperator_name={}\noperator_id={}\nrole={}\nruntime_command={}\nrole_runtime_command={}\nrole_loop_ready={}\nrole_loop_role={}\nrole_chain_profile={}\nrole_can_produce_blocks={}\nrole_local_producer={}\nrole_served_requests={}\nrole_produced_blocks={}\nrole_network_applied_blocks={}\nrole_network_events_ingested={}\nrole_network_block_events_ingested={}\nrole_network_block_headers_ingested={}\nrole_network_job_events_ingested={}\nrole_network_job_payloads_ingested={}\nrole_network_job_payloads_applied={}\nrole_network_receipt_events_ingested={}\nrole_network_receipt_payloads_ingested={}\nrole_network_receipt_payloads_applied={}\nrole_network_attestation_events_ingested={}\nrole_network_attestation_payloads_ingested={}\nrole_network_attestation_payloads_applied={}\nrole_network_peer_events_ingested={}\nrole_network_invalid_events={}\nrole_latest_height={}\nrole_p2p_connected_peers={}\nrole_p2p_observed_blocks={}\nrole_p2p_observed_jobs={}\nrole_p2p_observed_receipts={}\nrole_p2p_observed_attestations={}\nrole_p2p_latest_observed_block_height={}\nrole_p2p_latest_observed_block_hash={}\nrole_p2p_observed_block_hashes={}\nnode_multiaddr={}\np2p_peer_id={}\nheight={}\nepoch={}\nblock_count={}\nlatest_block_height={latest_block_height}\nlatest_block_hash={}\nstate_root={}\nblock_log_root={}\nfinalized_block_count={finalized_block_count}\nfirst_live_block_height={first_live_block_height}\nfirst_live_block_hash={}\nregistered_miner_count={}\nregistered_validator_count={}\njob_count={}\nreceipt_count={}\nsettled_receipt_count={}\nattestation_count={attestation_count}\nreward_account_count={reward_account_count}\nmodel_count={}\nbootstrap_peer_count={bootstrap_peer_count}\nnode_store_ready=true\nstatus_source=node_store",
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
    let service_report = serve_service_with_runtime(ServiceRuntimeConfig {
        runtime_command: "miner_run",
        role: RuntimeRole::Miner,
        node: runtime_node_config(
            config.data_dir,
            RuntimeRole::Miner,
            config.listen,
            config.p2p_listen,
            config.identity_seed,
            config.auth_token,
            config.max_requests,
        )?,
    })?;
    let device = config.device.unwrap_or("unknown");
    Ok(format!(
        "command=miner_run\nrole=miner\nwallet={}\ndevice={device}\nnode={}\nrole_runtime_ready=true\n{service_report}",
        config.wallet, config.node
    ))
}

fn run_validator_service(config: RoleServiceConfig<'_>) -> std::result::Result<String, String> {
    let service_report = serve_service_with_runtime(ServiceRuntimeConfig {
        runtime_command: "validator_run",
        role: RuntimeRole::Validator,
        node: runtime_node_config(
            config.data_dir,
            RuntimeRole::Validator,
            config.listen,
            config.p2p_listen,
            config.identity_seed,
            config.auth_token,
            config.max_requests,
        )?,
    })?;
    Ok(format!(
        "command=validator_run\nrole=validator\nwallet={}\nnode={}\nreference_verifier_ready=true\nrole_runtime_ready=true\n{service_report}",
        config.wallet, config.node
    ))
}

fn run_proposer_service(config: RoleServiceConfig<'_>) -> std::result::Result<String, String> {
    let service_report = serve_service_with_runtime(ServiceRuntimeConfig {
        runtime_command: "proposer_run",
        role: RuntimeRole::Proposer,
        node: runtime_node_config(
            config.data_dir,
            RuntimeRole::Proposer,
            config.listen,
            config.p2p_listen,
            config.identity_seed,
            config.auth_token,
            config.max_requests,
        )?,
    })?;
    Ok(format!(
        "command=proposer_run\nrole=proposer\nwallet={}\nnode={}\nproposer_ready=true\nrole_runtime_ready=true\n{service_report}",
        config.wallet, config.node
    ))
}

fn serve_service(
    listen: &str,
    p2p_listen: &str,
    data_dir: &str,
    identity_seed: Option<[u8; 32]>,
    auth_token: &str,
    max_requests: usize,
) -> std::result::Result<String, String> {
    serve_service_with_runtime(ServiceRuntimeConfig {
        runtime_command: "service_serve",
        role: RuntimeRole::Service,
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

struct ServiceRuntimeConfig<'a> {
    runtime_command: &'a str,
    role: RuntimeRole,
    node: NodeConfig,
}

fn serve_service_with_runtime(
    config: ServiceRuntimeConfig<'_>,
) -> std::result::Result<String, String> {
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
    let mut server = RpcHttpServer::bind(&network.rpc_listen, gateway).map_err(|error| {
        format!(
            "failed to bind service listener {}: {error}",
            network.rpc_listen
        )
    })?;
    let block_interval = config.node.synthetic_block_interval();
    let mut next_block_at = block_interval.map(|interval| Instant::now() + interval);
    let local_producer = config.node.local_synthetic_producer();
    let mut runtime_state = NodeRuntimeState::default();
    write_role_runtime_status(
        &config,
        &RoleRuntimeStatusSnapshot::from_runtime_state(
            &runtime_state,
            &server,
            &p2p_service,
            local_producer,
        ),
    )?;
    if block_interval.is_some() {
        server.set_nonblocking(true).map_err(|error| {
            format!("failed to configure nonblocking service listener: {error}")
        })?;
    }
    loop {
        if network.max_requests != 0 && runtime_state.served_requests() >= network.max_requests {
            break;
        }
        if let Some(interval) = block_interval {
            match server.serve_next() {
                Ok(()) => {
                    store
                        .persist_chain(&server.gateway().node.chain)
                        .map_err(|error| format!("failed to persist service state: {error}"))?;
                    runtime_state.record_served_request();
                    write_role_runtime_status(
                        &config,
                        &RoleRuntimeStatusSnapshot::from_runtime_state(
                            &runtime_state,
                            &server,
                            &p2p_service,
                            local_producer,
                        ),
                    )?;
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {}
                Err(error) => return Err(format!("service request failed: {error}")),
            }
            let ingested = ingest_network_events(
                &mut server,
                &p2p_service,
                local_producer,
                runtime_state.pending_payloads_mut(),
            )?;
            if ingested.has_activity() {
                runtime_state.record_network_ingest(ingested);
                if ingested.applied_blocks > 0
                    || ingested.job_payloads_applied > 0
                    || ingested.receipt_payloads_applied > 0
                    || ingested.attestation_payloads_applied > 0
                {
                    store
                        .persist_chain(&server.gateway().node.chain)
                        .map_err(|error| {
                            format!("failed to persist network-applied state: {error}")
                        })?;
                }
                write_role_runtime_status(
                    &config,
                    &RoleRuntimeStatusSnapshot::from_runtime_state(
                        &runtime_state,
                        &server,
                        &p2p_service,
                        local_producer,
                    ),
                )?;
            }
            if next_block_at.is_some_and(|deadline| Instant::now() >= deadline) {
                if local_producer
                    && produce_and_publish_synthetic_round(
                        &mut server,
                        &p2p_service,
                        &config.node.profile,
                    )?
                    .is_some()
                {
                    store
                        .persist_chain(&server.gateway().node.chain)
                        .map_err(|error| format!("failed to persist produced block: {error}"))?;
                    runtime_state.record_produced_block();
                    write_role_runtime_status(
                        &config,
                        &RoleRuntimeStatusSnapshot::from_runtime_state(
                            &runtime_state,
                            &server,
                            &p2p_service,
                            local_producer,
                        ),
                    )?;
                }
                next_block_at = Some(Instant::now() + interval);
            }
            thread::sleep(Duration::from_millis(25));
        } else {
            server
                .serve_next()
                .map_err(|error| format!("service request failed: {error}"))?;
            store
                .persist_chain(&server.gateway().node.chain)
                .map_err(|error| format!("failed to persist service state: {error}"))?;
            runtime_state.record_served_request();
            write_role_runtime_status(
                &config,
                &RoleRuntimeStatusSnapshot::from_runtime_state(
                    &runtime_state,
                    &server,
                    &p2p_service,
                    local_producer,
                ),
            )?;
        }
    }
    let network_events = runtime_state.network_events();
    Ok(format!(
        "command=service_serve\nruntime_command={}\nrole={}\nchain_profile={}\nrole_loop_ready=true\nrole_can_produce_blocks={}\nlocal_producer={local_producer}\nlisten={}\np2p_listen={}\np2p_runtime=libp2p\np2p_peer_id={p2p_peer_id}\np2p_connected_peers={}\np2p_observed_block_gossip_count={}\np2p_observed_job_gossip_count={}\np2p_observed_receipt_gossip_count={}\np2p_observed_attestation_gossip_count={}\np2p_latest_observed_block_height={}\np2p_latest_observed_block_hash={}\np2p_observed_block_hashes={}\np2p_gossipsub_topics={p2p_topics}\np2p_request_response_protocols={p2p_request_response_protocols}\np2p_bootstrap_peers={bootstrap_peer_count}\n{identity}\np2p_max_transmit_bytes={max_transmit_bytes}\np2p_request_timeout_seconds={request_timeout_seconds}\np2p_max_concurrent_streams={max_concurrent_streams}\np2p_idle_timeout_seconds={idle_timeout_seconds}\ndata_dir={}\nserved_requests={served_requests}\nproduced_blocks={produced_blocks}\nnetwork_applied_blocks={network_applied_blocks}\nnetwork_events_ingested={}\nnetwork_block_events_ingested={}\nnetwork_block_headers_ingested={}\nnetwork_job_events_ingested={}\nnetwork_job_payloads_ingested={}\nnetwork_job_payloads_applied={}\nnetwork_receipt_events_ingested={}\nnetwork_receipt_payloads_ingested={}\nnetwork_receipt_payloads_applied={}\nnetwork_attestation_events_ingested={}\nnetwork_attestation_payloads_ingested={}\nnetwork_attestation_payloads_applied={}\nnetwork_peer_events_ingested={}\nnetwork_invalid_events={}",
        config.runtime_command,
        config.role.label(),
        config.node.profile.label(),
        config.node.can_produce_local_blocks(),
        network.rpc_listen,
        network.p2p_listen,
        p2p_service.connected_peer_count(),
        p2p_service.observed_block_gossip_count(),
        p2p_service.observed_job_gossip_count(),
        p2p_service.observed_receipt_gossip_count(),
        p2p_service.observed_attestation_gossip_count(),
        p2p_service.latest_observed_block_height(),
        hex(&p2p_service.latest_observed_block_hash()),
        hex_hash_list(&p2p_service.observed_block_hashes()),
        config.node.data_dir().display(),
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
        served_requests = runtime_state.served_requests(),
        produced_blocks = runtime_state.produced_blocks(),
        network_applied_blocks = runtime_state.network_applied_blocks()
    ))
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
}

impl RoleRuntimeStatusSnapshot {
    fn from_runtime_state(
        state: &NodeRuntimeState,
        server: &RpcHttpServer,
        p2p_service: &TensorVmLibp2pService,
        local_producer: bool,
    ) -> Self {
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
        }
    }
}

fn write_role_runtime_status(
    config: &ServiceRuntimeConfig<'_>,
    snapshot: &RoleRuntimeStatusSnapshot,
) -> std::result::Result<(), String> {
    let path = config.node.data_dir().join("role-runtime.status");
    let contents = format!(
        "role_runtime_command={}\nrole_loop_role={}\nrole_loop_ready=true\nrole_chain_profile={}\nrole_can_produce_blocks={}\nrole_local_producer={}\nrole_served_requests={}\nrole_produced_blocks={}\nrole_network_applied_blocks={}\nrole_network_events_ingested={}\nrole_network_block_events_ingested={}\nrole_network_block_headers_ingested={}\nrole_network_job_events_ingested={}\nrole_network_job_payloads_ingested={}\nrole_network_job_payloads_applied={}\nrole_network_receipt_events_ingested={}\nrole_network_receipt_payloads_ingested={}\nrole_network_receipt_payloads_applied={}\nrole_network_attestation_events_ingested={}\nrole_network_attestation_payloads_ingested={}\nrole_network_attestation_payloads_applied={}\nrole_network_peer_events_ingested={}\nrole_network_invalid_events={}\nrole_latest_height={}\nrole_p2p_connected_peers={}\nrole_p2p_observed_blocks={}\nrole_p2p_observed_jobs={}\nrole_p2p_observed_receipts={}\nrole_p2p_observed_attestations={}\nrole_p2p_latest_observed_block_height={}\nrole_p2p_latest_observed_block_hash={}\nrole_p2p_observed_block_hashes={}\n",
        config.runtime_command,
        config.role.label(),
        config.node.profile.label(),
        config.node.can_produce_local_blocks(),
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
    let mut ingested = NetworkEventIngest::default();
    for message in network_ingest_order(p2p_service.drain_observed_messages()) {
        ingested.events = ingested.events.saturating_add(1);
        match message {
            P2pMessage::NewBlock(block_hash) => {
                ingested.block_announcements = ingested.block_announcements.saturating_add(1);
                if block_hash == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::NewBlockHeader { height, block_hash } => {
                ingested.block_announcements = ingested.block_announcements.saturating_add(1);
                ingested.block_headers = ingested.block_headers.saturating_add(1);
                if height == 0 || block_hash == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    continue;
                }
                if !local_producer {
                    ingested.applied_blocks =
                        ingested
                            .applied_blocks
                            .saturating_add(catch_up_to_announced_block(
                                server,
                                p2p_service,
                                height,
                                block_hash,
                            )?);
                }
            }
            P2pMessage::NewJob(job_id) => {
                ingested.jobs = ingested.jobs.saturating_add(1);
                if job_id == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::NewJobPayload { job_id, payload } => {
                ingested.jobs = ingested.jobs.saturating_add(1);
                ingested.job_payloads = ingested.job_payloads.saturating_add(1);
                match apply_network_job_payload(server, job_id, &payload) {
                    Ok(()) => {
                        ingested.job_payloads_applied =
                            ingested.job_payloads_applied.saturating_add(1);
                    }
                    Err(()) => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::NewReceipt(receipt_id) => {
                ingested.receipts = ingested.receipts.saturating_add(1);
                if receipt_id == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::NewReceiptPayload {
                receipt_id,
                payload,
            } => {
                ingested.receipts = ingested.receipts.saturating_add(1);
                ingested.receipt_payloads = ingested.receipt_payloads.saturating_add(1);
                match apply_network_receipt_payload(server, receipt_id, &payload) {
                    NetworkPayloadApply::Applied => {
                        ingested.receipt_payloads_applied =
                            ingested.receipt_payloads_applied.saturating_add(1);
                    }
                    NetworkPayloadApply::Pending => {
                        pending_payloads.queue_receipt(receipt_id, payload);
                    }
                    NetworkPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::NewAttestation(attestation_id) => {
                ingested.attestations = ingested.attestations.saturating_add(1);
                if attestation_id == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::NewAttestationPayload {
                attestation_id,
                payload,
            } => {
                ingested.attestations = ingested.attestations.saturating_add(1);
                ingested.attestation_payloads = ingested.attestation_payloads.saturating_add(1);
                match apply_network_attestation_payload(server, attestation_id, &payload) {
                    NetworkPayloadApply::Applied => {
                        ingested.attestation_payloads_applied =
                            ingested.attestation_payloads_applied.saturating_add(1);
                    }
                    NetworkPayloadApply::Pending => {
                        pending_payloads.queue_attestation(attestation_id, payload);
                    }
                    NetworkPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::PeerInfo { address } => {
                ingested.peers = ingested.peers.saturating_add(1);
                if address == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::RequestTensorChunk { .. }
            | P2pMessage::TensorChunkResponse { .. }
            | P2pMessage::RequestTensorRow { .. }
            | P2pMessage::TensorRowResponse { .. }
            | P2pMessage::RequestProgram(_)
            | P2pMessage::ProgramResponse { .. } => {
                ingested.invalid_events = ingested.invalid_events.saturating_add(1);
            }
        }
    }
    let mut processor = RuntimeNetworkPayloadProcessor { server };
    ingested.accumulate(pending_payloads.retry_with(&mut processor));
    Ok(ingested)
}

fn network_ingest_order(messages: Vec<P2pMessage>) -> Vec<P2pMessage> {
    let (mut block_messages, mut other_messages): (Vec<_>, Vec<_>) =
        messages.into_iter().partition(is_block_announcement);
    block_messages.append(&mut other_messages);
    block_messages
}

fn is_block_announcement(message: &P2pMessage) -> bool {
    matches!(
        message,
        P2pMessage::NewBlock(_) | P2pMessage::NewBlockHeader { .. }
    )
}

fn apply_network_job_payload(
    server: &mut RpcHttpServer,
    job_id: Hash,
    payload: &[u8],
) -> std::result::Result<(), ()> {
    if job_id == [0; 32] {
        return Err(());
    }
    let job = decode_job_payload(payload).map_err(|_| ())?;
    if job.job_id() != job_id {
        return Err(());
    }
    if let Some(existing) = server.gateway().node.chain.state.jobs.get(&job_id) {
        if existing == &job {
            return Ok(());
        }
        return Err(());
    }
    server
        .gateway_mut()
        .node
        .chain
        .apply_command(ChainCommand::SubmitJob(job))
        .map_err(|_| ())?;
    Ok(())
}

fn apply_network_receipt_payload(
    server: &mut RpcHttpServer,
    receipt_id: Hash,
    payload: &[u8],
) -> NetworkPayloadApply {
    if receipt_id == [0; 32] {
        return NetworkPayloadApply::Invalid;
    }
    let Ok(receipt) = decode_receipt_payload(payload) else {
        return NetworkPayloadApply::Invalid;
    };
    if receipt.receipt_id() != receipt_id {
        return NetworkPayloadApply::Invalid;
    }
    let chain = &server.gateway().node.chain;
    if let Some(existing) = chain.state.receipts.get(&receipt_id) {
        if existing == &receipt {
            return NetworkPayloadApply::Applied;
        }
        return NetworkPayloadApply::Invalid;
    }
    if !chain.state.jobs.contains_key(&receipt.job_id())
        || !chain.state.miners.contains_key(&receipt.miner())
    {
        return NetworkPayloadApply::Pending;
    }
    match server
        .gateway_mut()
        .node
        .chain
        .apply_command(ChainCommand::SubmitReceipt(receipt))
    {
        Ok(_) => NetworkPayloadApply::Applied,
        Err(TvmError::InvalidReceipt("unknown job") | TvmError::UnknownMiner) => {
            NetworkPayloadApply::Pending
        }
        Err(_) => NetworkPayloadApply::Invalid,
    }
}

fn apply_network_attestation_payload(
    server: &mut RpcHttpServer,
    attestation_id: Hash,
    payload: &[u8],
) -> NetworkPayloadApply {
    if attestation_id == [0; 32] {
        return NetworkPayloadApply::Invalid;
    }
    let Ok(attestation) = decode_attestation_payload(payload) else {
        return NetworkPayloadApply::Invalid;
    };
    if attestation_announcement_hash(&attestation) != attestation_id {
        return NetworkPayloadApply::Invalid;
    }
    let chain = &server.gateway().node.chain;
    if let Some(existing) = chain
        .state
        .attestations
        .get(&attestation.receipt_id)
        .and_then(|items| {
            items
                .iter()
                .find(|existing| existing.validator == attestation.validator)
        })
    {
        if existing == &attestation {
            return NetworkPayloadApply::Applied;
        }
        return NetworkPayloadApply::Invalid;
    }
    if !chain.state.validators.contains_key(&attestation.validator)
        || !chain.state.receipts.contains_key(&attestation.receipt_id)
    {
        return NetworkPayloadApply::Pending;
    }
    match server
        .gateway_mut()
        .node
        .chain
        .apply_command(ChainCommand::SubmitAttestation(attestation))
    {
        Ok(_) => NetworkPayloadApply::Applied,
        Err(TvmError::UnknownValidator | TvmError::UnknownReceipt) => NetworkPayloadApply::Pending,
        Err(_) => NetworkPayloadApply::Invalid,
    }
}

struct RuntimeNetworkPayloadProcessor<'a> {
    server: &'a mut RpcHttpServer,
}

impl NetworkPayloadProcessor for RuntimeNetworkPayloadProcessor<'_> {
    fn apply_receipt(&mut self, receipt_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_receipt_payload(self.server, receipt_id, payload)
    }

    fn apply_attestation(&mut self, attestation_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_attestation_payload(self.server, attestation_id, payload)
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

fn attestation_announcement_hash(attestation: &ValidatorAttestation) -> Hash {
    hash_bytes(
        b"tensor-vm-attestation-announcement-v1",
        &[
            &attestation.validator,
            &attestation.receipt_id,
            &attestation.job_id,
            &attestation.checks_root,
            &attestation.signature,
        ],
    )
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
                &mut missing_job_server,
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
                &mut receipt_server,
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
                &mut missing_receipt_server,
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
                &mut attestation_server,
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
                &mut server,
                receipt_id,
                &encode_receipt_payload(&receipt)
            ),
            NetworkPayloadApply::Pending
        );
        pending.queue_receipt(receipt_id, encode_receipt_payload(&receipt));
        assert_eq!(
            apply_network_attestation_payload(
                &mut server,
                attestation_id,
                &encode_attestation_payload(&attestation),
            ),
            NetworkPayloadApply::Pending
        );
        pending.queue_attestation(attestation_id, encode_attestation_payload(&attestation));

        apply_network_job_payload(&mut server, job_id, &encode_job_payload(&job)).unwrap();
        let mut processor = RuntimeNetworkPayloadProcessor {
            server: &mut server,
        };
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
