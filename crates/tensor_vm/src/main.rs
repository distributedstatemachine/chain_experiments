use std::{
    io::ErrorKind,
    path::Path,
    thread,
    time::{Duration, Instant},
};
use tensor_vm::{
    BlockVote, CliCommand, CpuReferenceBackend, Faucet, JobScheduler, JobState,
    Libp2pControlPlaneConfig, LocalChain, MatmulVerificationInput, MinerNode, NodeStore,
    PeerRecord, RpcGateway, RpcHttpServer, RpcNode, RpcPolicy, ValidatorNode,
    chain::TensorBlock,
    cli::{
        execute_reference_cli_command, validate_public_evidence_manifest,
        validate_public_testnet_preflight_manifest,
    },
    hash::hex,
    parse_cli_args, spawn_libp2p_service,
    testnet::{LocalTestnet, PublicTestnetCriteria, TestnetConfig},
    types::hash_bytes,
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
        let status = store
            .status()
            .map_err(|error| format!("existing node store is invalid: {error}"))?;
        return Ok(format!(
            "command=service_init\ndata_dir={}\nexisting_store=true\nblock_count={}\nlatest_block_hash={}",
            status.data_dir.display(),
            status.block_count,
            hex(&status.latest_block_hash)
        ));
    }

    let chain = LocalChain::new(hash_bytes(
        b"tensor-vm-service-genesis",
        &[data_dir.as_bytes()],
    ));
    let status = store
        .persist_chain(&chain)
        .map_err(|error| format!("failed to initialize node store {data_dir}: {error}"))?;
    Ok(format!(
        "command=service_init\ndata_dir={}\nexisting_store=false\nblock_count={}\nlatest_block_hash={}",
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
    let mut testnet = LocalTestnet::new(
        TestnetConfig::default(),
        hash_bytes(b"tensor-vm-local-cpu-compose-seed", &[data_dir.as_bytes()]),
    );
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
    Ok(format!(
        "command=local_testnet_seed\ndata_dir={data_dir}\nminers={}\nvalidators={}\nheight={}\nblocks={}\nsettled_receipts={}\nmatmul_settled={}\nlinear_training_settled={}\nmodel_states={}\nrewarded_miners={rewarded_miners}\ntotal_tensor_work={}\nfinality_rate_bps={}\ndata_availability_bps={}\nnode_store_ready=true\npersisted_block_count={}\nlatest_block_hash={}\npublic_evidence_full_spec=false\nindependently_checkable=false",
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

fn serve_service(
    listen: &str,
    p2p_listen: &str,
    data_dir: &str,
    identity_seed: Option<[u8; 32]>,
    auth_token: &str,
    max_requests: usize,
) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    let chain = store
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
        .map_err(|error| format!("failed to start mandatory libp2p service: {error}"))?;
    let p2p_peer_id = p2p_service.peer_id().to_string();
    let p2p_topics = p2p_service.info().subscribed_topics.len();
    let p2p_request_response_protocols = p2p_service.info().request_response_protocols.len();
    let identity = p2p_identity_report(identity_seed);
    let node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    let gateway = RpcGateway::new(
        node,
        RpcPolicy {
            auth_token: Some(auth_token.to_owned()),
            ..RpcPolicy::default()
        },
    );
    let mut server = RpcHttpServer::bind(listen, gateway)
        .map_err(|error| format!("failed to bind service listener {listen}: {error}"))?;
    let mut served_requests = 0usize;
    let block_interval = local_cpu_block_interval();
    let mut next_block_at = block_interval.map(|interval| Instant::now() + interval);
    let mut produced_blocks = 0usize;
    if block_interval.is_some() {
        server.set_nonblocking(true).map_err(|error| {
            format!("failed to configure nonblocking service listener: {error}")
        })?;
    }
    loop {
        if max_requests != 0 && served_requests >= max_requests {
            break;
        }
        if let Some(interval) = block_interval {
            match server.serve_next() {
                Ok(()) => {
                    store
                        .persist_chain(&server.gateway().node.chain)
                        .map_err(|error| format!("failed to persist service state: {error}"))?;
                    served_requests = served_requests.saturating_add(1);
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {}
                Err(error) => return Err(format!("service request failed: {error}")),
            }
            if next_block_at.is_some_and(|deadline| Instant::now() >= deadline) {
                if produce_synthetic_local_cpu_round(&mut server.gateway_mut().node.chain)?
                    .is_some()
                {
                    store
                        .persist_chain(&server.gateway().node.chain)
                        .map_err(|error| format!("failed to persist produced block: {error}"))?;
                    produced_blocks = produced_blocks.saturating_add(1);
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
        }
    }
    Ok(format!(
        "command=service_serve\nlisten={listen}\np2p_listen={p2p_listen}\np2p_runtime=libp2p\np2p_peer_id={p2p_peer_id}\np2p_gossipsub_topics={p2p_topics}\np2p_request_response_protocols={p2p_request_response_protocols}\np2p_bootstrap_peers={bootstrap_peer_count}\n{identity}\np2p_max_transmit_bytes={max_transmit_bytes}\np2p_request_timeout_seconds={request_timeout_seconds}\np2p_max_concurrent_streams={max_concurrent_streams}\np2p_idle_timeout_seconds={idle_timeout_seconds}\ndata_dir={data_dir}\nserved_requests={served_requests}\nproduced_blocks={produced_blocks}"
    ))
}

fn local_cpu_block_interval() -> Option<Duration> {
    std::env::var("TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .map(Duration::from_millis)
}

fn produce_synthetic_local_cpu_round(
    chain: &mut LocalChain,
) -> std::result::Result<Option<u64>, String> {
    if chain.state.miners.is_empty() || chain.state.validators.is_empty() {
        return Ok(None);
    }
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let beacon = chain.state.finalized_randomness;
    let job = scheduler.generate_small_matmul(
        chain.state.epoch,
        chain.state.height,
        &beacon,
        chain
            .state
            .height
            .saturating_add(chain.params.receipt_submission_window),
    );
    chain.submit_job(JobState::TensorOp(job.clone()));
    let miner_assignment = scheduler.assign_miners(chain, job.job_id, &beacon);
    let mut receipts = Vec::new();
    for (index, miner_address) in miner_assignment.miners.iter().copied().enumerate() {
        let mut miner = MinerNode::new(miner_address, CpuReferenceBackend);
        let (receipt, a, b, c) = miner
            .solve_matmul_job(&job, chain.state.height, 1 + index as u64)
            .map_err(|error| format!("synthetic CPU miner failed: {error}"))?;
        chain
            .submit_tensor_op_receipt(receipt.clone())
            .map_err(|error| format!("synthetic receipt rejected: {error}"))?;
        receipts.push((receipt, a, b, c));
    }
    for (receipt, a, b, c) in &receipts {
        let validation_seed = chain.validation_seed(&receipt.receipt_id);
        let validator_assignment = scheduler.assign_validators(chain, receipt.receipt_id, &beacon);
        for validator_address in validator_assignment.validators {
            let stake = chain
                .state
                .validators
                .get(&validator_address)
                .map(|validator| validator.stake)
                .unwrap_or_default();
            let validator = ValidatorNode::new(validator_address, stake);
            let attestation = validator
                .verify_matmul(MatmulVerificationInput {
                    job: &job,
                    receipt,
                    a,
                    b,
                    c,
                    validation_seed: &validation_seed,
                    params: &chain.params.freivalds,
                })
                .map_err(|error| format!("synthetic validator failed: {error}"))?;
            chain
                .submit_attestation(attestation)
                .map_err(|error| format!("synthetic attestation rejected: {error}"))?;
        }
    }
    let Some(canonical_receipt) = receipts.first().map(|(receipt, _, _, _)| receipt) else {
        return Ok(None);
    };
    if !chain.has_attestation_quorum(&canonical_receipt.receipt_id)
        || !chain.has_redundant_agreement(&canonical_receipt.receipt_id)
    {
        return Ok(None);
    }
    let settled_before = chain.state.settled_receipts.len();
    chain.settle_epoch(1_000, 500);
    if chain.state.settled_receipts.len() == settled_before {
        return Ok(None);
    }
    let Some(proposer) = chain
        .proposer_for_next_epoch(&beacon)
        .or_else(|| chain.state.miners.keys().next().copied())
    else {
        return Ok(None);
    };
    let timestamp = chain
        .blocks
        .last()
        .map(|block| {
            block
                .timestamp
                .saturating_add(chain.params.block_time_seconds)
        })
        .unwrap_or(0);
    let block = chain.produce_block(proposer, timestamp);
    finalize_local_cpu_block(chain, &block)?;
    Ok(Some(chain.state.height))
}

fn finalize_local_cpu_block(
    chain: &mut LocalChain,
    block: &TensorBlock,
) -> std::result::Result<(), String> {
    for validator_address in chain.state.validators.keys().copied().collect::<Vec<_>>() {
        let stake = chain
            .state
            .validators
            .get(&validator_address)
            .map(|validator| validator.stake)
            .unwrap_or_default();
        chain
            .submit_block_vote(BlockVote::new(validator_address, stake, block))
            .map_err(|error| format!("synthetic block vote rejected: {error}"))?;
        if chain.is_block_finalized(&block.hash()) {
            break;
        }
    }
    Ok(())
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
}
