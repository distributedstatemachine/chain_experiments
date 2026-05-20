use std::{
    collections::BTreeSet,
    io::ErrorKind,
    path::Path,
    thread,
    time::{Duration, Instant},
};
use tensor_vm::{
    CliCommand, Faucet, JobScheduler, Libp2pControlPlaneConfig, LocalChain, NodeStore, PeerRecord,
    RpcGateway, RpcHttpServer, RpcNode, RpcPolicy, Tensor, TensorVmLibp2pService,
    ValidatorAttestation,
    api::P2pMessage,
    cli::{
        execute_reference_cli_command, validate_public_evidence_manifest,
        validate_public_testnet_preflight_manifest,
    },
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
        "command=service_status\ndata_dir={}\noperator_name={}\noperator_id={}\nrole={}\nruntime_command={}\nrole_runtime_command={}\nrole_loop_ready={}\nrole_loop_role={}\nrole_local_producer={}\nrole_served_requests={}\nrole_produced_blocks={}\nrole_network_applied_blocks={}\nrole_latest_height={}\nrole_p2p_connected_peers={}\nrole_p2p_observed_blocks={}\nrole_p2p_observed_jobs={}\nrole_p2p_observed_receipts={}\nrole_p2p_observed_attestations={}\nrole_p2p_latest_observed_block_height={}\nrole_p2p_latest_observed_block_hash={}\nrole_p2p_observed_block_hashes={}\nnode_multiaddr={}\np2p_peer_id={}\nheight={}\nepoch={}\nblock_count={}\nlatest_block_height={latest_block_height}\nlatest_block_hash={}\nstate_root={}\nblock_log_root={}\nfinalized_block_count={finalized_block_count}\nfirst_live_block_height={first_live_block_height}\nfirst_live_block_hash={}\nregistered_miner_count={}\nregistered_validator_count={}\njob_count={}\nreceipt_count={}\nsettled_receipt_count={}\nattestation_count={attestation_count}\nreward_account_count={reward_account_count}\nmodel_count={}\nbootstrap_peer_count={bootstrap_peer_count}\nnode_store_ready=true\nstatus_source=node_store",
        status.data_dir.display(),
        ready_file_field(data_dir, "operator_name"),
        ready_file_field(data_dir, "operator_id"),
        ready_file_field(data_dir, "role"),
        ready_file_field(data_dir, "runtime_command"),
        role_runtime_status_field(data_dir, "role_runtime_command"),
        role_runtime_status_field(data_dir, "role_loop_ready"),
        role_runtime_status_field(data_dir, "role_loop_role"),
        role_runtime_status_field(data_dir, "role_local_producer"),
        role_runtime_status_field(data_dir, "role_served_requests"),
        role_runtime_status_field(data_dir, "role_produced_blocks"),
        role_runtime_status_field(data_dir, "role_network_applied_blocks"),
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
    Ok(format!(
        "command=service_block\ndata_dir={data_dir}\nheight={height}\nblock_hash={}\nstate_root={}\nepoch={}\nlatest_height={}\nfinalized={}\nstatus_source=node_store",
        hex(&block_hash),
        hex(&block.state_root),
        block.epoch,
        chain.state.height,
        chain.is_block_finalized(&block_hash),
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
        listen: config.listen,
        p2p_listen: config.p2p_listen,
        data_dir: config.data_dir,
        identity_seed: config.identity_seed,
        auth_token: config.auth_token,
        max_requests: config.max_requests,
        runtime_command: "miner_run",
        role: "miner",
    })?;
    let device = config.device.unwrap_or("unknown");
    Ok(format!(
        "command=miner_run\nrole=miner\nwallet={}\ndevice={device}\nnode={}\nrole_runtime_ready=true\n{service_report}",
        config.wallet, config.node
    ))
}

fn run_validator_service(config: RoleServiceConfig<'_>) -> std::result::Result<String, String> {
    let service_report = serve_service_with_runtime(ServiceRuntimeConfig {
        listen: config.listen,
        p2p_listen: config.p2p_listen,
        data_dir: config.data_dir,
        identity_seed: config.identity_seed,
        auth_token: config.auth_token,
        max_requests: config.max_requests,
        runtime_command: "validator_run",
        role: "validator",
    })?;
    Ok(format!(
        "command=validator_run\nrole=validator\nwallet={}\nnode={}\nreference_verifier_ready=true\nrole_runtime_ready=true\n{service_report}",
        config.wallet, config.node
    ))
}

fn run_proposer_service(config: RoleServiceConfig<'_>) -> std::result::Result<String, String> {
    let service_report = serve_service_with_runtime(ServiceRuntimeConfig {
        listen: config.listen,
        p2p_listen: config.p2p_listen,
        data_dir: config.data_dir,
        identity_seed: config.identity_seed,
        auth_token: config.auth_token,
        max_requests: config.max_requests,
        runtime_command: "proposer_run",
        role: "proposer",
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
        listen,
        p2p_listen,
        data_dir,
        identity_seed,
        auth_token,
        max_requests,
        runtime_command: "service_serve",
        role: "service",
    })
}

struct ServiceRuntimeConfig<'a> {
    listen: &'a str,
    p2p_listen: &'a str,
    data_dir: &'a str,
    identity_seed: Option<[u8; 32]>,
    auth_token: &'a str,
    max_requests: usize,
    runtime_command: &'a str,
    role: &'a str,
}

fn serve_service_with_runtime(
    config: ServiceRuntimeConfig<'_>,
) -> std::result::Result<String, String> {
    let store = NodeStore::open(config.data_dir);
    let chain = store
        .load_chain()
        .map_err(|error| format!("failed to load node store {}: {error}", config.data_dir))?;
    let bootstrap_addresses = if store.peer_book_store().path().exists() {
        store
            .peer_book_store()
            .load_bootstrap_addresses()
            .map_err(|error| {
                format!(
                    "failed to load libp2p peer book {}: {error}",
                    config.data_dir
                )
            })?
    } else {
        Vec::new()
    };
    let bootstrap_peer_count = bootstrap_addresses.len();
    let p2p_config = Libp2pControlPlaneConfig {
        listen_addresses: vec![config.p2p_listen.to_owned()],
        bootstrap_addresses,
        identity_seed: config.identity_seed,
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
    let identity = p2p_identity_report(config.identity_seed);
    let node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    let gateway = RpcGateway::new(
        node,
        RpcPolicy {
            auth_token: Some(config.auth_token.to_owned()),
            ..RpcPolicy::default()
        },
    );
    let mut server = RpcHttpServer::bind(config.listen, gateway)
        .map_err(|error| format!("failed to bind service listener {}: {error}", config.listen))?;
    let mut served_requests = 0usize;
    let block_interval = local_cpu_block_interval();
    let mut next_block_at = block_interval.map(|interval| Instant::now() + interval);
    let local_producer = local_cpu_role_producer();
    let mut produced_blocks = 0usize;
    let mut network_applied_blocks = 0usize;
    write_role_runtime_status(
        &config,
        &role_runtime_status_snapshot(
            &server,
            &p2p_service,
            served_requests,
            produced_blocks,
            network_applied_blocks,
            local_producer,
        ),
    )?;
    if block_interval.is_some() {
        server.set_nonblocking(true).map_err(|error| {
            format!("failed to configure nonblocking service listener: {error}")
        })?;
    }
    loop {
        if config.max_requests != 0 && served_requests >= config.max_requests {
            break;
        }
        if let Some(interval) = block_interval {
            match server.serve_next() {
                Ok(()) => {
                    store
                        .persist_chain(&server.gateway().node.chain)
                        .map_err(|error| format!("failed to persist service state: {error}"))?;
                    served_requests = served_requests.saturating_add(1);
                    write_role_runtime_status(
                        &config,
                        &role_runtime_status_snapshot(
                            &server,
                            &p2p_service,
                            served_requests,
                            produced_blocks,
                            network_applied_blocks,
                            local_producer,
                        ),
                    )?;
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {}
                Err(error) => return Err(format!("service request failed: {error}")),
            }
            if next_block_at.is_some_and(|deadline| Instant::now() >= deadline) {
                if local_producer {
                    if produce_and_publish_synthetic_round(&mut server, &p2p_service)?.is_some() {
                        store
                            .persist_chain(&server.gateway().node.chain)
                            .map_err(|error| {
                                format!("failed to persist produced block: {error}")
                            })?;
                        produced_blocks = produced_blocks.saturating_add(1);
                        write_role_runtime_status(
                            &config,
                            &role_runtime_status_snapshot(
                                &server,
                                &p2p_service,
                                served_requests,
                                produced_blocks,
                                network_applied_blocks,
                                local_producer,
                            ),
                        )?;
                    }
                } else if let applied_blocks @ 1.. =
                    catch_up_to_observed_block(&mut server, &p2p_service)?
                {
                    store
                        .persist_chain(&server.gateway().node.chain)
                        .map_err(|error| {
                            format!("failed to persist network-applied block: {error}")
                        })?;
                    network_applied_blocks = network_applied_blocks.saturating_add(applied_blocks);
                    write_role_runtime_status(
                        &config,
                        &role_runtime_status_snapshot(
                            &server,
                            &p2p_service,
                            served_requests,
                            produced_blocks,
                            network_applied_blocks,
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
            served_requests = served_requests.saturating_add(1);
            write_role_runtime_status(
                &config,
                &role_runtime_status_snapshot(
                    &server,
                    &p2p_service,
                    served_requests,
                    produced_blocks,
                    network_applied_blocks,
                    local_producer,
                ),
            )?;
        }
    }
    Ok(format!(
        "command=service_serve\nruntime_command={}\nrole={}\nrole_loop_ready=true\nlocal_producer={local_producer}\nlisten={}\np2p_listen={}\np2p_runtime=libp2p\np2p_peer_id={p2p_peer_id}\np2p_connected_peers={}\np2p_observed_block_gossip_count={}\np2p_observed_job_gossip_count={}\np2p_observed_receipt_gossip_count={}\np2p_observed_attestation_gossip_count={}\np2p_latest_observed_block_height={}\np2p_latest_observed_block_hash={}\np2p_observed_block_hashes={}\np2p_gossipsub_topics={p2p_topics}\np2p_request_response_protocols={p2p_request_response_protocols}\np2p_bootstrap_peers={bootstrap_peer_count}\n{identity}\np2p_max_transmit_bytes={max_transmit_bytes}\np2p_request_timeout_seconds={request_timeout_seconds}\np2p_max_concurrent_streams={max_concurrent_streams}\np2p_idle_timeout_seconds={idle_timeout_seconds}\ndata_dir={}\nserved_requests={served_requests}\nproduced_blocks={produced_blocks}\nnetwork_applied_blocks={network_applied_blocks}",
        config.runtime_command,
        config.role,
        config.listen,
        config.p2p_listen,
        p2p_service.connected_peer_count(),
        p2p_service.observed_block_gossip_count(),
        p2p_service.observed_job_gossip_count(),
        p2p_service.observed_receipt_gossip_count(),
        p2p_service.observed_attestation_gossip_count(),
        p2p_service.latest_observed_block_height(),
        hex(&p2p_service.latest_observed_block_hash()),
        hex_hash_list(&p2p_service.observed_block_hashes()),
        config.data_dir
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
}

fn role_runtime_status_snapshot(
    server: &RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    served_requests: usize,
    produced_blocks: usize,
    network_applied_blocks: usize,
    local_producer: bool,
) -> RoleRuntimeStatusSnapshot {
    RoleRuntimeStatusSnapshot {
        served_requests,
        produced_blocks,
        network_applied_blocks,
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
    }
}

fn write_role_runtime_status(
    config: &ServiceRuntimeConfig<'_>,
    snapshot: &RoleRuntimeStatusSnapshot,
) -> std::result::Result<(), String> {
    let path = Path::new(config.data_dir).join("role-runtime.status");
    let contents = format!(
        "role_runtime_command={}\nrole_loop_role={}\nrole_loop_ready=true\nrole_local_producer={}\nrole_served_requests={}\nrole_produced_blocks={}\nrole_network_applied_blocks={}\nrole_latest_height={}\nrole_p2p_connected_peers={}\nrole_p2p_observed_blocks={}\nrole_p2p_observed_jobs={}\nrole_p2p_observed_receipts={}\nrole_p2p_observed_attestations={}\nrole_p2p_latest_observed_block_height={}\nrole_p2p_latest_observed_block_hash={}\nrole_p2p_observed_block_hashes={}\n",
        config.runtime_command,
        config.role,
        snapshot.local_producer,
        snapshot.served_requests,
        snapshot.produced_blocks,
        snapshot.network_applied_blocks,
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

fn catch_up_to_observed_block(
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
) -> std::result::Result<usize, String> {
    let target_height = p2p_service.latest_observed_block_height();
    let target_hash = p2p_service.latest_observed_block_hash();
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
    publish_new_chain_announcements(
        p2p_service,
        &announcement_checkpoint,
        &server.gateway().node.chain,
    )?;
    if let Some(block) = block_at_height(&server.gateway().node.chain, target_height) {
        publish_block_announcement(p2p_service, block.height, block.hash())?;
    }
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

fn produce_and_publish_synthetic_round(
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
) -> std::result::Result<Option<Hash>, String> {
    let announcement_checkpoint = chain_announcement_checkpoint(&server.gateway().node.chain);
    if server
        .gateway_mut()
        .node
        .produce_synthetic_cpu_round()
        .map_err(|error| format!("synthetic CPU round failed: {error}"))?
        .is_none()
    {
        return Ok(None);
    }
    publish_new_chain_announcements(
        p2p_service,
        &announcement_checkpoint,
        &server.gateway().node.chain,
    )?;
    let Some(block) = server.gateway().node.chain.blocks.last() else {
        return Ok(None);
    };
    let block_hash = block.hash();
    publish_block_announcement(p2p_service, block.height, block_hash)?;
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
    for job_id in chain.state.jobs.keys().copied() {
        if !before.jobs.contains(&job_id) {
            p2p_service
                .publish_gossip(P2pMessage::NewJob(job_id))
                .map_err(|error| format!("failed to publish job gossip: {error}"))?;
        }
    }
    for receipt_id in chain.state.receipts.keys().copied() {
        if !before.receipts.contains(&receipt_id) {
            p2p_service
                .publish_gossip(P2pMessage::NewReceipt(receipt_id))
                .map_err(|error| format!("failed to publish receipt gossip: {error}"))?;
        }
    }
    for attestation_id in attestation_announcement_hashes(chain) {
        if !before.attestations.contains(&attestation_id) {
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

fn local_cpu_block_interval() -> Option<Duration> {
    std::env::var("TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .map(Duration::from_millis)
}

fn local_cpu_role_producer() -> bool {
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
}
