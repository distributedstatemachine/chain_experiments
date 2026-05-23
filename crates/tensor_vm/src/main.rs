use std::{
    collections::BTreeSet,
    io::ErrorKind,
    path::Path,
    thread,
    time::{Duration, Instant},
};
use tensor_vm::{
    BlockVote, Chain, ChainCommand, ChainEngine, ChainProfile, ChainSnapshot, CliCommand, Faucet,
    JobScheduler, Libp2pControlPlaneConfig, NetworkConfig, NetworkEventIngest, NodeConfig,
    NodeRole, NodeRuntimeState, NodeStore, PeerRecord, PendingNetworkPayloads, ReceiptState,
    RpcGateway, RpcHttpServer, RpcNode, RpcPolicy, SyntheticLocalJobSource, Tensor,
    TensorVmLibp2pService,
    api::P2pMessage,
    cli::{
        execute_reference_cli_command, validate_public_evidence_manifest,
        validate_public_testnet_preflight_manifest,
    },
    decode_tensor_payload, encode_attestation_payload, encode_block_payload,
    encode_block_vote_payload, encode_job_payload, encode_receipt_payload,
    hash::hex,
    jobs::LinearTrainingStepOutput,
    localnet::produce_synthetic_cpu_round_with_profile,
    node::{
        NetworkBlockPayloadApply, NetworkEventContext, apply_network_block_payload,
        attestation_announcement_hash, ingest_network_messages,
    },
    parse_cli_args,
    roles::{
        CpuReferenceMinerRole, ReferenceValidatorRole, RoleReceiptArtifacts, RoleReceiptBundle,
    },
    spawn_libp2p_service,
    testnet::{LocalTestnet, PublicTestnetCriteria, TestnetConfig},
    types::{Address, Hash, address, hash_bytes},
};

#[path = "main/status.rs"]
mod status;

use status::{service_block_status, service_status};

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

    let chain = Chain::new(hash_bytes(
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

fn verify_local_cpu_store(data_dir: &str, json: bool) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    let chain = store
        .load_chain()
        .map_err(|error| format!("failed to load node store {data_dir}: {error}"))?;
    let status = store
        .status()
        .map_err(|error| format!("failed to inspect node store {data_dir}: {error}"))?;
    let latest_block_height = chain
        .blocks()
        .last()
        .map(|block| block.height)
        .unwrap_or_default();
    let finalized_block_count = chain
        .blocks()
        .iter()
        .filter(|block| chain.is_block_finalized(&block.hash()))
        .count();
    let ready = status.block_count == chain.blocks().len()
        && status.block_count > 0
        && chain.state().height() == latest_block_height.saturating_add(1)
        && finalized_block_count <= status.block_count;
    if json {
        Ok(format!(
            "{{\"command\":\"local_cpu_verify\",\"data_dir\":\"{}\",\"structured_verifier_ready\":true,\"ready\":{},\"height\":{},\"latest_block_height\":{},\"block_count\":{},\"finalized_block_count\":{},\"node_store_ready\":true}}",
            json_escape(data_dir),
            ready,
            chain.state().height(),
            latest_block_height,
            status.block_count,
            finalized_block_count
        ))
    } else {
        Ok(format!(
            "command=local_cpu_verify\ndata_dir={data_dir}\nstructured_verifier_ready=true\nready={ready}\nheight={}\nlatest_block_height={latest_block_height}\nblock_count={}\nfinalized_block_count={finalized_block_count}\nnode_store_ready=true",
            chain.state().height(),
            status.block_count
        ))
    }
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct MinerRoleWorkObservation {
    assigned_jobs: BTreeSet<Hash>,
    unreceipted_jobs: BTreeSet<Hash>,
}

fn miner_role_work_observation(chain: &Chain, miner: Address) -> MinerRoleWorkObservation {
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let assignment_seed = chain.state().finalized_randomness();
    let mut observation = MinerRoleWorkObservation::default();
    for job_id in chain.state().jobs().keys() {
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

fn miner_has_receipt_for_job(chain: &Chain, miner: Address, job_id: Hash) -> bool {
    chain
        .state()
        .receipts()
        .values()
        .any(|receipt| receipt.job_id() == job_id && receipt.miner() == miner)
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct MinerRoleReceiptSubmission {
    receipts_submitted: usize,
    tensors_inserted: usize,
    served_tensors: Vec<Tensor>,
}

fn submit_miner_role_receipt(
    node: &mut RpcNode,
    miner: Address,
    job_id: Hash,
) -> std::result::Result<Option<MinerRoleReceiptSubmission>, String> {
    if !node.chain.state().miners().contains_key(&miner) {
        return Ok(None);
    }
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let assignment = scheduler.assign_miners(
        &node.chain,
        job_id,
        &node.chain.state().finalized_randomness(),
    );
    if !assignment.miners.contains(&miner) || miner_has_receipt_for_job(&node.chain, miner, job_id)
    {
        return Ok(None);
    }
    let Some(job) = node.chain.state().jobs().get(&job_id).cloned() else {
        return Ok(None);
    };
    let bundle = CpuReferenceMinerRole::new(miner)
        .execute_job(&job, node.chain.state().height(), 1)
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
    for tensor in &served_tensors {
        node.insert_tensor(tensor.clone());
        tensors_inserted = tensors_inserted.saturating_add(1);
    }
    Ok(Some(MinerRoleReceiptSubmission {
        receipts_submitted: 1,
        tensors_inserted,
        served_tensors,
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
    let assignment_seed = node.chain.state().finalized_randomness();
    let mut observation = ValidatorRoleWorkObservation::default();
    for (receipt_id, receipt) in node.chain.state().receipts() {
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

fn validator_has_attested_for_receipt(chain: &Chain, validator: Address, receipt_id: Hash) -> bool {
    chain
        .state()
        .attestations()
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ValidatorRoleBlockVoteSubmission {
    block_votes_submitted: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ValidatorRemoteTensorFetchReport {
    attempts: usize,
    successes: usize,
    failures: usize,
    bytes: usize,
    tensors_inserted: usize,
}

fn fetch_validator_role_missing_tensors(
    node: &mut RpcNode,
    p2p_service: &TensorVmLibp2pService,
    receipt_id: Hash,
) -> std::result::Result<ValidatorRemoteTensorFetchReport, String> {
    let Some(receipt) = node.chain.state().receipts().get(&receipt_id).cloned() else {
        return Ok(ValidatorRemoteTensorFetchReport::default());
    };
    let missing_roots = validator_receipt_required_remote_roots(node, &receipt);
    if missing_roots.is_empty() {
        return Ok(ValidatorRemoteTensorFetchReport::default());
    }
    let peers = p2p_service.connected_peer_ids();
    let mut report = ValidatorRemoteTensorFetchReport::default();
    if peers.is_empty() {
        report.failures = missing_roots.len();
        return Ok(report);
    }
    for root in missing_roots {
        let mut fetched = false;
        let mut failed_response_recorded = false;
        for peer in &peers {
            report.attempts = report.attempts.saturating_add(1);
            let response = p2p_service.request_response(
                *peer,
                P2pMessage::RequestTensorByCommitmentRoot {
                    commitment_root: root,
                },
                Duration::from_secs(2),
            );
            let Ok(response) = response else {
                continue;
            };
            match validator_remote_tensor_response(root, response) {
                ValidatorRemoteTensorResponse::Found { tensor, bytes } => {
                    node.insert_tensor(tensor.clone());
                    p2p_service.register_tensor(tensor);
                    report.bytes = report.bytes.saturating_add(bytes);
                    report.successes = report.successes.saturating_add(1);
                    report.tensors_inserted = report.tensors_inserted.saturating_add(1);
                    fetched = true;
                    break;
                }
                ValidatorRemoteTensorResponse::Missing => {}
                ValidatorRemoteTensorResponse::Invalid => {
                    record_validator_remote_fetch_failure(
                        &mut report,
                        &mut failed_response_recorded,
                    );
                }
            }
        }
        if !fetched && !failed_response_recorded {
            report.failures = report.failures.saturating_add(1);
        }
    }
    Ok(report)
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ValidatorRemoteTensorResponse {
    Found { tensor: Tensor, bytes: usize },
    Missing,
    Invalid,
}

fn validator_remote_tensor_response(
    requested_root: Hash,
    response: P2pMessage,
) -> ValidatorRemoteTensorResponse {
    let P2pMessage::TensorByCommitmentRootResponse {
        commitment_root,
        payload,
    } = response
    else {
        return ValidatorRemoteTensorResponse::Missing;
    };
    if commitment_root != requested_root {
        return ValidatorRemoteTensorResponse::Invalid;
    }
    let Some(payload) = payload else {
        return ValidatorRemoteTensorResponse::Missing;
    };
    let bytes = payload.len();
    let Ok(tensor) = decode_tensor_payload(&payload) else {
        return ValidatorRemoteTensorResponse::Invalid;
    };
    if tensor.commitment_root() != requested_root {
        return ValidatorRemoteTensorResponse::Invalid;
    }
    ValidatorRemoteTensorResponse::Found { tensor, bytes }
}

fn record_validator_remote_fetch_failure(
    report: &mut ValidatorRemoteTensorFetchReport,
    recorded_for_root: &mut bool,
) {
    if !*recorded_for_root {
        report.failures = report.failures.saturating_add(1);
        *recorded_for_root = true;
    }
}

fn validator_receipt_required_remote_roots(node: &RpcNode, receipt: &ReceiptState) -> Vec<Hash> {
    let mut roots = Vec::new();
    match receipt {
        ReceiptState::TensorOp(receipt) => {
            roots.extend(receipt.input_roots.iter().copied());
            roots.extend(receipt.output_roots.iter().copied());
        }
        ReceiptState::LinearTrainingStep(receipt) => {
            roots.push(receipt.y_root);
            roots.push(receipt.grad_w_root);
            roots.push(receipt.weight_root_after);
        }
    }
    roots.sort();
    roots.dedup();
    roots
        .into_iter()
        .filter(|root| !node.contains_tensor_commitment_root(root))
        .collect()
}

fn submit_validator_role_attestation(
    node: &mut RpcNode,
    validator: Address,
    receipt_id: Hash,
) -> std::result::Result<Option<ValidatorRoleAttestationSubmission>, String> {
    let Some(validator_state) = node.chain.state().validators().get(&validator) else {
        return Ok(None);
    };
    let validator_stake = validator_state.stake;
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let assignment = scheduler.assign_validators(
        &node.chain,
        receipt_id,
        &node.chain.state().finalized_randomness(),
    );
    if !assignment.validators.contains(&validator)
        || validator_has_attested_for_receipt(&node.chain, validator, receipt_id)
    {
        return Ok(None);
    }
    let Some(receipt) = node.chain.state().receipts().get(&receipt_id).cloned() else {
        return Ok(None);
    };
    let Some(job) = node.chain.state().jobs().get(&receipt.job_id()).cloned() else {
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
            &node.chain.params().freivalds,
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

fn submit_validator_role_block_vote(
    node: &mut RpcNode,
    validator: Address,
) -> std::result::Result<Option<ValidatorRoleBlockVoteSubmission>, String> {
    let Some(validator_state) = node.chain.state().validators().get(&validator) else {
        return Ok(None);
    };
    let validator_stake = validator_state.stake;
    let Some(block) = node
        .chain
        .blocks()
        .iter()
        .rev()
        .find(|block| {
            let block_hash = block.hash();
            !node.chain.is_block_finalized(&block_hash)
                && !validator_has_block_vote(&node.chain, validator, block_hash)
                && node.chain.validate_block(block).is_ok()
        })
        .cloned()
    else {
        return Ok(None);
    };
    let vote = BlockVote::new(validator, validator_stake, &block);
    node.chain
        .apply_command(ChainCommand::SubmitBlockVote(vote))
        .map_err(|error| {
            format!(
                "validator role failed to submit block vote {}: {error}",
                hex(&block.hash())
            )
        })?;
    Ok(Some(ValidatorRoleBlockVoteSubmission {
        block_votes_submitted: 1,
    }))
}

fn validator_has_block_vote(chain: &Chain, validator: Address, block_hash: Hash) -> bool {
    chain
        .state()
        .block_votes()
        .get(&block_hash)
        .is_some_and(|votes| votes.iter().any(|vote| vote.validator == validator))
}

fn role_receipt_bundle_from_local_tensors(
    node: &RpcNode,
    receipt: &ReceiptState,
) -> Option<RoleReceiptBundle> {
    let job = node.chain.state().jobs().get(&receipt.job_id())?;
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

struct RuntimeStatusSnapshot {
    served_requests: usize,
    produced_blocks: usize,
    network_applied_blocks: usize,
    local_producer: bool,
    latest_height: u64,
    p2p_connected_peers: usize,
    p2p_observed_blocks: usize,
    p2p_observed_block_payloads: usize,
    p2p_observed_block_votes: usize,
    p2p_observed_jobs: usize,
    p2p_observed_receipts: usize,
    p2p_observed_attestations: usize,
    p2p_latest_observed_block_height: u64,
    p2p_latest_observed_block_hash: [u8; 32],
    p2p_observed_block_hashes: Vec<[u8; 32]>,
    p2p_latest_observed_block_payload_height: u64,
    p2p_latest_observed_block_payload_hash: [u8; 32],
    p2p_observed_block_payload_hashes: Vec<[u8; 32]>,
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
    validator_remote_tensor_fetch_attempts: usize,
    validator_remote_tensor_fetch_successes: usize,
    validator_remote_tensor_fetch_failures: usize,
    validator_remote_tensor_fetch_bytes: usize,
    validator_remote_tensors_inserted: usize,
    validator_attestations_submitted: usize,
    validator_block_votes_submitted: usize,
}

impl RuntimeStatusSnapshot {
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
            latest_height: server.gateway().node.chain.state().height(),
            p2p_connected_peers: p2p_service.connected_peer_count(),
            p2p_observed_blocks: p2p_service.observed_block_gossip_count(),
            p2p_observed_block_payloads: p2p_service.observed_block_payload_gossip_count(),
            p2p_observed_block_votes: p2p_service.observed_block_vote_gossip_count(),
            p2p_observed_jobs: p2p_service.observed_job_gossip_count(),
            p2p_observed_receipts: p2p_service.observed_receipt_gossip_count(),
            p2p_observed_attestations: p2p_service.observed_attestation_gossip_count(),
            p2p_latest_observed_block_height: p2p_service.latest_observed_block_height(),
            p2p_latest_observed_block_hash: p2p_service.latest_observed_block_hash(),
            p2p_observed_block_hashes: p2p_service.observed_block_hashes(),
            p2p_latest_observed_block_payload_height: p2p_service
                .latest_observed_block_payload_height(),
            p2p_latest_observed_block_payload_hash: p2p_service
                .latest_observed_block_payload_hash(),
            p2p_observed_block_payload_hashes: p2p_service.observed_block_payload_hashes(),
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
            validator_remote_tensor_fetch_attempts: state.validator_remote_tensor_fetch_attempts(),
            validator_remote_tensor_fetch_successes: state
                .validator_remote_tensor_fetch_successes(),
            validator_remote_tensor_fetch_failures: state.validator_remote_tensor_fetch_failures(),
            validator_remote_tensor_fetch_bytes: state.validator_remote_tensor_fetch_bytes(),
            validator_remote_tensors_inserted: state.validator_remote_tensors_inserted(),
            validator_attestations_submitted: state.validator_attestations_submitted(),
            validator_block_votes_submitted: state.validator_block_votes_submitted(),
        }
    }
}

fn write_role_runtime_status(
    config: &ServiceRuntimeConfig,
    snapshot: &RuntimeStatusSnapshot,
) -> std::result::Result<(), String> {
    let path = config.node.data_dir().join("role-runtime.status");
    let contents = format!(
        "role_runtime_command={}\nrole_loop_role={}\nrole_loop_ready=true\nrole_chain_profile={}\nrole_can_produce_blocks={}\nrole_wallet_address={}\nrole_wallet_registration={}\nrole_wallet_registered={}\nrole_miner_work_ready={}\nrole_miner_assigned_jobs_seen={}\nrole_miner_unreceipted_jobs={}\nrole_miner_receipts_submitted={}\nrole_miner_tensors_inserted={}\nrole_validator_work_ready={}\nrole_validator_assigned_receipts_seen={}\nrole_validator_unattested_receipts={}\nrole_validator_artifact_ready_receipts={}\nrole_validator_artifact_missing_receipts={}\nrole_validator_remote_tensor_fetch_attempts={}\nrole_validator_remote_tensor_fetch_successes={}\nrole_validator_remote_tensor_fetch_failures={}\nrole_validator_remote_tensor_fetch_bytes={}\nrole_validator_remote_tensors_inserted={}\nrole_validator_attestations_submitted={}\nrole_validator_block_votes_submitted={}\nrole_local_producer={}\nrole_served_requests={}\nrole_produced_blocks={}\nrole_network_applied_blocks={}\nrole_network_events_ingested={}\nrole_network_block_events_ingested={}\nrole_network_block_headers_ingested={}\nrole_network_block_payloads_ingested={}\nrole_network_block_payloads_applied={}\nrole_network_block_votes_ingested={}\nrole_network_block_votes_applied={}\nrole_network_job_events_ingested={}\nrole_network_job_payloads_ingested={}\nrole_network_job_payloads_applied={}\nrole_network_receipt_events_ingested={}\nrole_network_receipt_payloads_ingested={}\nrole_network_receipt_payloads_applied={}\nrole_network_attestation_events_ingested={}\nrole_network_attestation_payloads_ingested={}\nrole_network_attestation_payloads_applied={}\nrole_network_peer_events_ingested={}\nrole_network_invalid_events={}\nrole_latest_height={}\nrole_p2p_connected_peers={}\nrole_p2p_observed_blocks={}\nrole_p2p_observed_block_payloads={}\nrole_p2p_observed_block_votes={}\nrole_p2p_observed_jobs={}\nrole_p2p_observed_receipts={}\nrole_p2p_observed_attestations={}\nrole_p2p_latest_observed_block_height={}\nrole_p2p_latest_observed_block_hash={}\nrole_p2p_observed_block_hashes={}\nrole_p2p_latest_observed_block_payload_height={}\nrole_p2p_latest_observed_block_payload_hash={}\nrole_p2p_observed_block_payload_hashes={}\n",
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
        snapshot.validator_remote_tensor_fetch_attempts,
        snapshot.validator_remote_tensor_fetch_successes,
        snapshot.validator_remote_tensor_fetch_failures,
        snapshot.validator_remote_tensor_fetch_bytes,
        snapshot.validator_remote_tensors_inserted,
        snapshot.validator_attestations_submitted,
        snapshot.validator_block_votes_submitted,
        snapshot.local_producer,
        snapshot.served_requests,
        snapshot.produced_blocks,
        snapshot.network_applied_blocks,
        snapshot.network_events.events,
        snapshot.network_events.block_announcements,
        snapshot.network_events.block_headers,
        snapshot.network_events.block_payloads,
        snapshot.network_events.block_payloads_applied,
        snapshot.network_events.block_votes,
        snapshot.network_events.block_votes_applied,
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
        snapshot.p2p_observed_block_payloads,
        snapshot.p2p_observed_block_votes,
        snapshot.p2p_observed_jobs,
        snapshot.p2p_observed_receipts,
        snapshot.p2p_observed_attestations,
        snapshot.p2p_latest_observed_block_height,
        hex(&snapshot.p2p_latest_observed_block_hash),
        hex_hash_list(&snapshot.p2p_observed_block_hashes),
        snapshot.p2p_latest_observed_block_payload_height,
        hex(&snapshot.p2p_latest_observed_block_payload_hash),
        hex_hash_list(&snapshot.p2p_observed_block_payload_hashes)
    );
    std::fs::write(&path, contents).map_err(|error| {
        format!(
            "failed to write role runtime status {}: {error}",
            path.display()
        )
    })
}

fn ingest_network_events(
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    local_producer: bool,
    pending_payloads: &mut PendingNetworkPayloads,
) -> std::result::Result<NetworkEventIngest, String> {
    let messages = p2p_service.drain_observed_messages();
    let mut context = RuntimeNetworkEventContext { server };
    ingest_network_messages(&mut context, messages, local_producer, pending_payloads)
}

struct RuntimeNetworkEventContext<'a> {
    server: &'a mut RpcHttpServer,
}

impl NetworkEventContext for RuntimeNetworkEventContext<'_> {
    fn chain(&mut self) -> &mut Chain {
        &mut self.server.gateway_mut().node.chain
    }

    fn apply_block_payload(
        &mut self,
        height: u64,
        block_hash: Hash,
        payload: &[u8],
    ) -> NetworkBlockPayloadApply {
        apply_network_block_payload(
            &mut self.server.gateway_mut().node.chain,
            height,
            block_hash,
            payload,
        )
    }
}

fn produce_and_publish_synthetic_round(
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    profile: &ChainProfile,
) -> std::result::Result<Option<Hash>, String> {
    let announcement_checkpoint = chain_announcement_checkpoint(&server.gateway().node.chain);
    let Some(round) =
        produce_synthetic_cpu_round_with_profile(&mut server.gateway_mut().node.chain, profile)
            .map_err(|error| format!("synthetic CPU round failed: {error}"))?
    else {
        return Ok(None);
    };
    for tensor in round.tensors {
        p2p_service.register_tensor(tensor.clone());
        server.gateway_mut().node.insert_tensor(tensor);
    }
    let Some(block) = server.gateway().node.chain.blocks().last() else {
        return Ok(None);
    };
    let block_hash = block.hash();
    publish_new_chain_announcements(
        p2p_service,
        &announcement_checkpoint,
        &server.gateway().node.chain,
    )?;
    publish_block_announcements(p2p_service, block)?;
    Ok(Some(block_hash))
}

fn publish_block_announcements(
    p2p_service: &TensorVmLibp2pService,
    block: &tensor_vm::chain::TensorBlock,
) -> std::result::Result<(), String> {
    let block_hash = block.hash();
    p2p_service
        .publish_gossip(P2pMessage::NewBlockPayload {
            height: block.height,
            block_hash,
            payload: encode_block_payload(block),
        })
        .map_err(|error| format!("failed to publish block payload gossip: {error}"))?;
    p2p_service
        .publish_gossip(P2pMessage::NewBlockHeader {
            height: block.height,
            block_hash,
        })
        .map_err(|error| format!("failed to publish block header gossip: {error}"))?;
    p2p_service
        .publish_gossip(P2pMessage::NewBlock(block_hash))
        .map_err(|error| format!("failed to publish block hash gossip: {error}"))
}

struct ChainAnnouncementCheckpoint {
    jobs: BTreeSet<Hash>,
    receipts: BTreeSet<Hash>,
    attestations: BTreeSet<Hash>,
    block_votes: BTreeSet<(Hash, Address)>,
}

fn chain_announcement_checkpoint(chain: &Chain) -> ChainAnnouncementCheckpoint {
    ChainAnnouncementCheckpoint {
        jobs: chain.state().jobs().keys().copied().collect(),
        receipts: chain.state().receipts().keys().copied().collect(),
        attestations: attestation_announcement_hashes(chain).collect(),
        block_votes: block_vote_announcement_keys(chain).collect(),
    }
}

fn publish_new_chain_announcements(
    p2p_service: &TensorVmLibp2pService,
    before: &ChainAnnouncementCheckpoint,
    chain: &Chain,
) -> std::result::Result<(), String> {
    for (job_id, job) in chain.state().jobs() {
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
    for (receipt_id, receipt) in chain.state().receipts() {
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
        .state()
        .attestations()
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
    for (block_hash, votes) in chain.state().block_votes() {
        for vote in votes {
            let key = (*block_hash, vote.validator);
            if !before.block_votes.contains(&key) {
                p2p_service
                    .publish_gossip(P2pMessage::NewBlockVotePayload {
                        block_hash: *block_hash,
                        validator: vote.validator,
                        payload: encode_block_vote_payload(vote),
                    })
                    .map_err(|error| {
                        format!("failed to publish block vote payload gossip: {error}")
                    })?;
            }
        }
    }
    Ok(())
}

fn attestation_announcement_hashes(chain: &Chain) -> impl Iterator<Item = Hash> + '_ {
    chain
        .state()
        .attestations()
        .values()
        .flat_map(|attestations| attestations.iter().map(attestation_announcement_hash))
}

fn block_vote_announcement_keys(chain: &Chain) -> impl Iterator<Item = (Hash, Address)> + '_ {
    chain
        .state()
        .block_votes()
        .iter()
        .flat_map(|(block_hash, votes)| votes.iter().map(move |vote| (*block_hash, vote.validator)))
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
