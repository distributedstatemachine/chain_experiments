use std::{
    io::ErrorKind,
    path::Path,
    thread,
    time::{Duration, Instant},
};
use tensor_vm::{
    Chain, ChainProfile, ChainSnapshot, CliCommand, Faucet, JobScheduler, Libp2pControlPlaneConfig,
    NetworkConfig, NodeConfig, NodeRole, NodeRuntimeState, NodeStore, RpcGateway, RpcHttpServer,
    RpcNode, RpcPolicy, TensorVmLibp2pService,
    cli::{
        execute_reference_cli_command, validate_public_evidence_manifest,
        validate_public_testnet_preflight_manifest,
    },
    hash::hex,
    parse_cli_args, spawn_libp2p_service,
    testnet::{LocalTestnet, PublicTestnetCriteria, TestnetConfig},
    types::{Address, address, hash_bytes},
};

#[path = "main/commands.rs"]
mod commands;

#[path = "main/network.rs"]
mod network;

#[path = "main/roles.rs"]
mod roles;

#[path = "main/status.rs"]
mod status;

use commands::{
    add_service_peer, check_service_readiness, init_service_store, verify_local_cpu_store,
};
use network::{
    chain_announcement_checkpoint, ingest_network_events, produce_and_publish_synthetic_round,
    publish_new_chain_announcements,
};
use roles::{
    fetch_validator_role_missing_tensors, miner_role_work_observation, submit_miner_role_receipt,
    submit_validator_role_attestation, submit_validator_role_block_vote,
    validator_role_work_observation,
};
use status::{
    RuntimeStatusSnapshot, service_block_status, service_status, write_role_runtime_status,
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
        CliCommand::LocalCpuVerify { data_dir, json } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            verify_local_cpu_store(data_dir, *json)
        }
        _ => execute_reference_cli_command(command).map_err(|error| error.to_string()),
    }
}

fn seed_local_testnet(data_dir: &str) -> std::result::Result<String, String> {
    let mut testnet = LocalTestnet::new(TestnetConfig::default(), local_cpu_seed_beacon());
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    testnet.run_matmul_round(&scheduler);
    let matmul_settled_receipts = testnet.chain.state().settled_receipts().len();
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
        .filter(|miner| testnet.chain.state().rewards().balance(miner) > 0)
        .count();
    let total_reward_balance = testnet.chain.state().rewards().total_balance();
    let attestation_count: usize = testnet
        .chain
        .state()
        .attestations()
        .values()
        .map(Vec::len)
        .sum();
    Ok(format!(
        "command=local_testnet_seed\ndata_dir={data_dir}\nminers={}\nvalidators={}\nheight={}\nblocks={}\nsettled_receipts={}\nmatmul_settled={}\nlinear_training_settled={}\nmodel_states={}\nrewarded_miners={rewarded_miners}\ntotal_reward_balance={total_reward_balance}\nattestation_count={attestation_count}\ntotal_tensor_work={}\nfinality_rate_bps={}\ndata_availability_bps={}\nnode_store_ready=true\npersisted_block_count={}\nlatest_block_hash={}\npublic_evidence_full_spec=false\nindependently_checkable=false",
        testnet.miners.len(),
        testnet.validators.len(),
        testnet.chain.state().height(),
        testnet.chain.blocks().len(),
        testnet.chain.state().settled_receipts().len(),
        matmul_settled_receipts > 0,
        !testnet.chain.state().model_states().is_empty(),
        testnet.chain.state().model_states().len(),
        telemetry.total_tensor_work,
        local_evidence.finality_rate_bps,
        local_evidence.data_availability_bps,
        status.block_count,
        hex(&status.latest_block_hash)
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
    chain: &Chain,
) -> &'static str {
    let Some(address) = address else {
        return "none";
    };
    match role {
        RuntimeRole::Miner => {
            if chain.state().miners().contains_key(&address) {
                "miner"
            } else {
                "unregistered"
            }
        }
        RuntimeRole::Validator => {
            if chain.state().validators().contains_key(&address) {
                "validator"
            } else {
                "unregistered"
            }
        }
        RuntimeRole::Proposer if chain.state().validators().contains_key(&address) => "validator",
        RuntimeRole::Proposer => "unregistered",
        RuntimeRole::Service => "none",
    }
}

fn runtime_role_wallet_registered(
    role: RuntimeRole,
    address: Option<Address>,
    chain: &Chain,
) -> bool {
    !matches!(
        runtime_role_wallet_registration(role, address, chain),
        "none" | "unregistered"
    )
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
        self.server.set_nonblocking(true).map_err(|error| {
            format!("failed to configure nonblocking service listener: {error}")
        })?;
        loop {
            if self.max_requests_reached() {
                break;
            }
            self.serve_rpc_once()?;
            self.ingest_network_once()?;
            self.tick_role_work_once()?;
            self.produce_local_round_if_due()?;
            thread::sleep(Duration::from_millis(25));
        }
        Ok(())
    }

    fn max_requests_reached(&self) -> bool {
        let max_requests = self.config.node.network.max_requests;
        max_requests != 0 && self.runtime_state.served_requests() >= max_requests
    }

    fn serve_rpc_once(&mut self) -> std::result::Result<(), String> {
        let chain_snapshot_before = ChainSnapshot::from_chain(&self.server.gateway().node.chain);
        match self.server.serve_next() {
            Ok(()) => {
                let chain_changed = ChainSnapshot::from_chain(&self.server.gateway().node.chain)
                    != chain_snapshot_before;
                self.record_served_request(chain_changed)
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(()),
            Err(error) => Err(format!("service request failed: {error}")),
        }
    }

    fn record_served_request(&mut self, chain_changed: bool) -> std::result::Result<(), String> {
        if chain_changed {
            self.store
                .persist_chain(&self.server.gateway().node.chain)
                .map_err(|error| format!("failed to persist service state: {error}"))?;
        }
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
            || ingested.attestation_payloads_applied > 0
            || ingested.block_votes_applied > 0;
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
                for tensor in submission.served_tensors {
                    self.p2p_service.register_tensor(tensor);
                }
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
        let receipt_to_fetch = observation.artifact_missing_receipts.iter().next().copied();
        let mut receipt_to_submit = observation.artifact_ready_receipts.iter().next().copied();
        let mut status_changed = false;
        if self.runtime_state.record_validator_work_observation(
            observation.assigned_receipts,
            observation.unattested_receipts,
            observation.artifact_ready_receipts,
            observation.artifact_missing_receipts,
        ) {
            status_changed = true;
        }
        if receipt_to_submit.is_none()
            && let Some(receipt_id) = receipt_to_fetch
        {
            let fetch_report = fetch_validator_role_missing_tensors(
                &mut self.server.gateway_mut().node,
                &self.p2p_service,
                receipt_id,
            )?;
            if fetch_report.attempts > 0
                || fetch_report.successes > 0
                || fetch_report.failures > 0
                || fetch_report.tensors_inserted > 0
            {
                self.runtime_state.record_validator_remote_tensor_fetch(
                    fetch_report.attempts,
                    fetch_report.successes,
                    fetch_report.failures,
                    fetch_report.bytes,
                    fetch_report.tensors_inserted,
                );
                let observation =
                    validator_role_work_observation(&self.server.gateway().node, validator);
                receipt_to_submit = observation.artifact_ready_receipts.iter().next().copied();
                self.runtime_state.record_validator_work_observation(
                    observation.assigned_receipts,
                    observation.unattested_receipts,
                    observation.artifact_ready_receipts,
                    observation.artifact_missing_receipts,
                );
                status_changed = true;
            }
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
        let announcement_checkpoint =
            chain_announcement_checkpoint(&self.server.gateway().node.chain);
        if let Some(submission) =
            submit_validator_role_block_vote(&mut self.server.gateway_mut().node, validator)?
        {
            publish_new_chain_announcements(
                &self.p2p_service,
                &announcement_checkpoint,
                &self.server.gateway().node.chain,
            )?;
            self.store
                .persist_chain(&self.server.gateway().node.chain)
                .map_err(|error| {
                    format!("failed to persist validator block vote state: {error}")
                })?;
            self.runtime_state
                .record_validator_block_vote_submission(submission.block_votes_submitted);
            status_changed = true;
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
        if self
            .next_block_at
            .is_none_or(|deadline| Instant::now() < deadline)
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
            &RuntimeStatusSnapshot::from_runtime_state(
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
            "command=service_serve\nruntime_command={}\nrole={}\nchain_profile={}\nrole_loop_ready=true\nrole_can_produce_blocks={}\nrole_wallet_address={}\nrole_wallet_registration={}\nrole_wallet_registered={}\nminer_work_ready={}\nminer_assigned_jobs_seen={}\nminer_unreceipted_jobs={}\nminer_receipts_submitted={}\nminer_tensors_inserted={}\nvalidator_work_ready={}\nvalidator_assigned_receipts_seen={}\nvalidator_unattested_receipts={}\nvalidator_artifact_ready_receipts={}\nvalidator_artifact_missing_receipts={}\nvalidator_remote_tensor_fetch_attempts={}\nvalidator_remote_tensor_fetch_successes={}\nvalidator_remote_tensor_fetch_failures={}\nvalidator_remote_tensor_fetch_bytes={}\nvalidator_remote_tensors_inserted={}\nvalidator_attestations_submitted={}\nvalidator_block_votes_submitted={}\nlocal_producer={local_producer}\nlisten={}\np2p_listen={}\np2p_runtime=libp2p\np2p_peer_id={p2p_peer_id}\np2p_connected_peers={}\np2p_observed_block_gossip_count={}\np2p_observed_block_payload_gossip_count={}\np2p_observed_block_vote_gossip_count={}\np2p_observed_job_gossip_count={}\np2p_observed_receipt_gossip_count={}\np2p_observed_attestation_gossip_count={}\np2p_latest_observed_block_height={}\np2p_latest_observed_block_hash={}\np2p_observed_block_hashes={}\np2p_latest_observed_block_payload_height={}\np2p_latest_observed_block_payload_hash={}\np2p_observed_block_payload_hashes={}\np2p_gossipsub_topics={p2p_topics}\np2p_request_response_protocols={p2p_request_response_protocols}\np2p_bootstrap_peers={bootstrap_peer_count}\n{identity}\np2p_max_transmit_bytes={max_transmit_bytes}\np2p_request_timeout_seconds={request_timeout_seconds}\np2p_max_concurrent_streams={max_concurrent_streams}\np2p_idle_timeout_seconds={idle_timeout_seconds}\ndata_dir={}\nserved_requests={served_requests}\nproduced_blocks={produced_blocks}\nnetwork_applied_blocks={network_applied_blocks}\nnetwork_events_ingested={}\nnetwork_block_events_ingested={}\nnetwork_block_headers_ingested={}\nnetwork_block_payloads_ingested={}\nnetwork_block_payloads_applied={}\nnetwork_block_votes_ingested={}\nnetwork_block_votes_applied={}\nnetwork_job_events_ingested={}\nnetwork_job_payloads_ingested={}\nnetwork_job_payloads_applied={}\nnetwork_receipt_events_ingested={}\nnetwork_receipt_payloads_ingested={}\nnetwork_receipt_payloads_applied={}\nnetwork_attestation_events_ingested={}\nnetwork_attestation_payloads_ingested={}\nnetwork_attestation_payloads_applied={}\nnetwork_peer_events_ingested={}\nnetwork_invalid_events={}",
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
            self.runtime_state.validator_remote_tensor_fetch_attempts(),
            self.runtime_state.validator_remote_tensor_fetch_successes(),
            self.runtime_state.validator_remote_tensor_fetch_failures(),
            self.runtime_state.validator_remote_tensor_fetch_bytes(),
            self.runtime_state.validator_remote_tensors_inserted(),
            self.runtime_state.validator_attestations_submitted(),
            self.runtime_state.validator_block_votes_submitted(),
            network.rpc_listen,
            network.p2p_listen,
            self.p2p_service.connected_peer_count(),
            self.p2p_service.observed_block_gossip_count(),
            self.p2p_service.observed_block_payload_gossip_count(),
            self.p2p_service.observed_block_vote_gossip_count(),
            self.p2p_service.observed_job_gossip_count(),
            self.p2p_service.observed_receipt_gossip_count(),
            self.p2p_service.observed_attestation_gossip_count(),
            self.p2p_service.latest_observed_block_height(),
            hex(&self.p2p_service.latest_observed_block_hash()),
            hex_hash_list(&self.p2p_service.observed_block_hashes()),
            self.p2p_service.latest_observed_block_payload_height(),
            hex(&self.p2p_service.latest_observed_block_payload_hash()),
            hex_hash_list(&self.p2p_service.observed_block_payload_hashes()),
            self.config.node.data_dir().display(),
            network_events.events,
            network_events.block_announcements,
            network_events.block_headers,
            network_events.block_payloads,
            network_events.block_payloads_applied,
            network_events.block_votes,
            network_events.block_votes_applied,
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
        Err(_) => false,
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
#[path = "main_tests.rs"]
mod tests;
