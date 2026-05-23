use super::runtime_config::{
    RuntimeRole, ServiceRuntimeConfig, runtime_role_wallet_address_text,
    runtime_role_wallet_registered, runtime_role_wallet_registration,
};
use std::path::Path;
use tensor_vm::{
    NetworkEventIngest, NodeRuntimeState, NodeStore, RpcHttpServer, TensorVmLibp2pService,
    hash::hex, types::Address,
};

pub(super) fn hex_hash_list(hashes: &[[u8; 32]]) -> String {
    if hashes.is_empty() {
        return "none".to_owned();
    }
    hashes
        .iter()
        .map(|hash| hex(hash))
        .collect::<Vec<_>>()
        .join(",")
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

pub(super) fn service_status(data_dir: &str) -> std::result::Result<String, String> {
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
    let first_live_block = chain.blocks().iter().find(|block| block.height > 2);
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
    let attestation_count: usize = chain.state().attestations().values().map(Vec::len).sum();
    let reward_account_count = chain
        .state()
        .rewards()
        .balances()
        .values()
        .filter(|balance| **balance > 0)
        .count();
    Ok(format!(
        "command=service_status\ndata_dir={}\noperator_name={}\noperator_id={}\nrole={}\nruntime_command={}\nrole_runtime_command={}\nrole_loop_ready={}\nrole_loop_role={}\nrole_chain_profile={}\nrole_can_produce_blocks={}\nrole_wallet_address={}\nrole_wallet_registration={}\nrole_wallet_registered={}\nrole_miner_work_ready={}\nrole_miner_assigned_jobs_seen={}\nrole_miner_unreceipted_jobs={}\nrole_miner_receipts_submitted={}\nrole_miner_tensors_inserted={}\nrole_validator_work_ready={}\nrole_validator_assigned_receipts_seen={}\nrole_validator_unattested_receipts={}\nrole_validator_artifact_ready_receipts={}\nrole_validator_artifact_missing_receipts={}\nrole_validator_remote_tensor_fetch_attempts={}\nrole_validator_remote_tensor_fetch_successes={}\nrole_validator_remote_tensor_fetch_failures={}\nrole_validator_remote_tensor_fetch_bytes={}\nrole_validator_remote_tensors_inserted={}\nrole_validator_attestations_submitted={}\nrole_validator_block_votes_submitted={}\nrole_local_producer={}\nrole_served_requests={}\nrole_produced_blocks={}\nrole_network_applied_blocks={}\nrole_network_events_ingested={}\nrole_network_block_events_ingested={}\nrole_network_block_headers_ingested={}\nrole_network_block_payloads_ingested={}\nrole_network_block_payloads_applied={}\nrole_network_block_votes_ingested={}\nrole_network_block_votes_applied={}\nrole_network_job_events_ingested={}\nrole_network_job_payloads_ingested={}\nrole_network_job_payloads_applied={}\nrole_network_receipt_events_ingested={}\nrole_network_receipt_payloads_ingested={}\nrole_network_receipt_payloads_applied={}\nrole_network_attestation_events_ingested={}\nrole_network_attestation_payloads_ingested={}\nrole_network_attestation_payloads_applied={}\nrole_network_peer_events_ingested={}\nrole_network_invalid_events={}\nrole_latest_height={}\nrole_p2p_connected_peers={}\nrole_p2p_observed_blocks={}\nrole_p2p_observed_block_payloads={}\nrole_p2p_observed_block_votes={}\nrole_p2p_observed_jobs={}\nrole_p2p_observed_receipts={}\nrole_p2p_observed_attestations={}\nrole_p2p_latest_observed_block_height={}\nrole_p2p_latest_observed_block_hash={}\nrole_p2p_observed_block_hashes={}\nrole_p2p_latest_observed_block_payload_height={}\nrole_p2p_latest_observed_block_payload_hash={}\nrole_p2p_observed_block_payload_hashes={}\nnode_multiaddr={}\np2p_peer_id={}\nheight={}\nepoch={}\nblock_count={}\nlatest_block_height={latest_block_height}\nlatest_block_hash={}\nstate_root={}\nblock_log_root={}\nfinalized_block_count={finalized_block_count}\nfirst_live_block_height={first_live_block_height}\nfirst_live_block_hash={}\nregistered_miner_count={}\nregistered_validator_count={}\njob_count={}\nreceipt_count={}\nsettled_receipt_count={}\nattestation_count={attestation_count}\nreward_account_count={reward_account_count}\nmodel_count={}\nbootstrap_peer_count={bootstrap_peer_count}\nnode_store_ready=true\nstatus_source=node_store",
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
        role_runtime_status_field(data_dir, "role_validator_remote_tensor_fetch_attempts"),
        role_runtime_status_field(data_dir, "role_validator_remote_tensor_fetch_successes"),
        role_runtime_status_field(data_dir, "role_validator_remote_tensor_fetch_failures"),
        role_runtime_status_field(data_dir, "role_validator_remote_tensor_fetch_bytes"),
        role_runtime_status_field(data_dir, "role_validator_remote_tensors_inserted"),
        role_runtime_status_field(data_dir, "role_validator_attestations_submitted"),
        role_runtime_status_field(data_dir, "role_validator_block_votes_submitted"),
        role_runtime_status_field(data_dir, "role_local_producer"),
        role_runtime_status_field(data_dir, "role_served_requests"),
        role_runtime_status_field(data_dir, "role_produced_blocks"),
        role_runtime_status_field(data_dir, "role_network_applied_blocks"),
        role_runtime_status_field(data_dir, "role_network_events_ingested"),
        role_runtime_status_field(data_dir, "role_network_block_events_ingested"),
        role_runtime_status_field(data_dir, "role_network_block_headers_ingested"),
        role_runtime_status_field(data_dir, "role_network_block_payloads_ingested"),
        role_runtime_status_field(data_dir, "role_network_block_payloads_applied"),
        role_runtime_status_field(data_dir, "role_network_block_votes_ingested"),
        role_runtime_status_field(data_dir, "role_network_block_votes_applied"),
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
        role_runtime_status_field(data_dir, "role_p2p_observed_block_payloads"),
        role_runtime_status_field(data_dir, "role_p2p_observed_block_votes"),
        role_runtime_status_field(data_dir, "role_p2p_observed_jobs"),
        role_runtime_status_field(data_dir, "role_p2p_observed_receipts"),
        role_runtime_status_field(data_dir, "role_p2p_observed_attestations"),
        role_runtime_status_field(data_dir, "role_p2p_latest_observed_block_height"),
        role_runtime_status_field(data_dir, "role_p2p_latest_observed_block_hash"),
        role_runtime_status_field(data_dir, "role_p2p_observed_block_hashes"),
        role_runtime_status_field(data_dir, "role_p2p_latest_observed_block_payload_height"),
        role_runtime_status_field(data_dir, "role_p2p_latest_observed_block_payload_hash"),
        role_runtime_status_field(data_dir, "role_p2p_observed_block_payload_hashes"),
        ready_file_field(data_dir, "node_multiaddr"),
        ready_file_field(data_dir, "p2p_peer_id"),
        chain.state().height(),
        chain.state().epoch(),
        status.block_count,
        hex(&status.latest_block_hash),
        hex(&chain.state_root()),
        hex(&status.block_log_root),
        hex(&first_live_block_hash),
        chain.state().miners().len(),
        chain.state().validators().len(),
        chain.state().jobs().len(),
        chain.state().receipts().len(),
        chain.state().settled_receipts().len(),
        chain.state().model_states().len(),
    ))
}

pub(super) struct RuntimeStatusSnapshot {
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
    pub(super) fn from_runtime_state(
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

pub(super) struct RuntimeP2pReport<'a> {
    pub(super) peer_id: &'a str,
    pub(super) topics: usize,
    pub(super) request_response_protocols: usize,
    pub(super) bootstrap_peer_count: usize,
    pub(super) identity: &'a str,
    pub(super) max_transmit_bytes: usize,
    pub(super) request_timeout_seconds: u64,
    pub(super) max_concurrent_streams: usize,
    pub(super) idle_timeout_seconds: u64,
}

pub(super) fn format_role_runtime_report(
    config: &ServiceRuntimeConfig,
    snapshot: &RuntimeStatusSnapshot,
    p2p: &RuntimeP2pReport<'_>,
) -> String {
    let network = &config.node.network;
    let network_events = snapshot.network_events;
    format!(
        "command=service_serve\nruntime_command={}\nrole={}\nchain_profile={}\nrole_loop_ready=true\nrole_can_produce_blocks={}\nrole_wallet_address={}\nrole_wallet_registration={}\nrole_wallet_registered={}\nminer_work_ready={}\nminer_assigned_jobs_seen={}\nminer_unreceipted_jobs={}\nminer_receipts_submitted={}\nminer_tensors_inserted={}\nvalidator_work_ready={}\nvalidator_assigned_receipts_seen={}\nvalidator_unattested_receipts={}\nvalidator_artifact_ready_receipts={}\nvalidator_artifact_missing_receipts={}\nvalidator_remote_tensor_fetch_attempts={}\nvalidator_remote_tensor_fetch_successes={}\nvalidator_remote_tensor_fetch_failures={}\nvalidator_remote_tensor_fetch_bytes={}\nvalidator_remote_tensors_inserted={}\nvalidator_attestations_submitted={}\nvalidator_block_votes_submitted={}\nlocal_producer={local_producer}\nlisten={}\np2p_listen={}\np2p_runtime=libp2p\np2p_peer_id={p2p_peer_id}\np2p_connected_peers={}\np2p_observed_block_gossip_count={}\np2p_observed_block_payload_gossip_count={}\np2p_observed_block_vote_gossip_count={}\np2p_observed_job_gossip_count={}\np2p_observed_receipt_gossip_count={}\np2p_observed_attestation_gossip_count={}\np2p_latest_observed_block_height={}\np2p_latest_observed_block_hash={}\np2p_observed_block_hashes={}\np2p_latest_observed_block_payload_height={}\np2p_latest_observed_block_payload_hash={}\np2p_observed_block_payload_hashes={}\np2p_gossipsub_topics={p2p_topics}\np2p_request_response_protocols={p2p_request_response_protocols}\np2p_bootstrap_peers={bootstrap_peer_count}\n{identity}\np2p_max_transmit_bytes={max_transmit_bytes}\np2p_request_timeout_seconds={request_timeout_seconds}\np2p_max_concurrent_streams={max_concurrent_streams}\np2p_idle_timeout_seconds={idle_timeout_seconds}\ndata_dir={}\nserved_requests={served_requests}\nproduced_blocks={produced_blocks}\nnetwork_applied_blocks={network_applied_blocks}\nnetwork_events_ingested={}\nnetwork_block_events_ingested={}\nnetwork_block_headers_ingested={}\nnetwork_block_payloads_ingested={}\nnetwork_block_payloads_applied={}\nnetwork_block_votes_ingested={}\nnetwork_block_votes_applied={}\nnetwork_job_events_ingested={}\nnetwork_job_payloads_ingested={}\nnetwork_job_payloads_applied={}\nnetwork_receipt_events_ingested={}\nnetwork_receipt_payloads_ingested={}\nnetwork_receipt_payloads_applied={}\nnetwork_attestation_events_ingested={}\nnetwork_attestation_payloads_ingested={}\nnetwork_attestation_payloads_applied={}\nnetwork_peer_events_ingested={}\nnetwork_invalid_events={}",
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
        network.rpc_listen,
        network.p2p_listen,
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
        hex_hash_list(&snapshot.p2p_observed_block_payload_hashes),
        config.node.data_dir().display(),
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
        local_producer = snapshot.local_producer,
        p2p_peer_id = p2p.peer_id,
        p2p_topics = p2p.topics,
        p2p_request_response_protocols = p2p.request_response_protocols,
        bootstrap_peer_count = p2p.bootstrap_peer_count,
        identity = p2p.identity,
        max_transmit_bytes = p2p.max_transmit_bytes,
        request_timeout_seconds = p2p.request_timeout_seconds,
        max_concurrent_streams = p2p.max_concurrent_streams,
        idle_timeout_seconds = p2p.idle_timeout_seconds,
        served_requests = snapshot.served_requests,
        produced_blocks = snapshot.produced_blocks,
        network_applied_blocks = snapshot.network_applied_blocks
    )
}

pub(super) fn write_role_runtime_status(
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
