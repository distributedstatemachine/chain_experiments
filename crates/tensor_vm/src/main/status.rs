use super::runtime::{
    RuntimeRole, ServiceRuntimeConfig, runtime_role_wallet_address_text,
    runtime_role_wallet_registered, runtime_role_wallet_registration,
};
use std::{collections::BTreeSet, path::Path};
use tensor_vm::{
    Chain, NetworkEventIngest, NodeRuntimeState, NodeStore, PrimitiveType, RpcHttpServer,
    TensorVmLibp2pService, hash::hex, types::Address,
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

pub(super) fn service_block_status(
    data_dir: &str,
    height: u64,
) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    let chain = store
        .load_chain()
        .map_err(|error| format!("failed to load node store {data_dir}: {error}"))?;
    let Some(block) = chain.blocks().iter().find(|block| block.height == height) else {
        return Err(format!(
            "block height {height} is not in node store {data_dir}"
        ));
    };
    let block_hash = block.hash();
    let selected_receipt_ids = chain.selected_receipts_for_block(block);
    let blockspace_caps = chain.blockspace_caps();
    let selected_receipt_twu = selected_receipt_ids
        .iter()
        .filter_map(|receipt_id| chain.state().receipts().get(receipt_id))
        .map(|receipt| receipt.tensor_work_units())
        .sum::<u64>();
    let selected_receipt_bytes = selected_receipt_ids
        .iter()
        .filter_map(|receipt_id| chain.state().receipts().get(receipt_id))
        .map(|receipt| receipt.estimated_block_bytes())
        .sum::<u64>();
    let block_valid = chain.validate_block(block).is_ok();
    let proposer_registered = chain.state().validators().contains_key(&block.proposer);
    let pow_hash = block.pow_hash();
    let pow_header_hash = block.pow_header_hash();
    let block_votes = chain
        .state()
        .block_votes()
        .get(&block_hash)
        .cloned()
        .unwrap_or_default();
    let total_validator_stake = chain
        .state()
        .validators()
        .values()
        .map(|validator| validator.stake)
        .sum::<u64>();
    let finality_threshold_stake = finality_threshold_stake(&chain, total_validator_stake);
    let mut seen_vote_validators = BTreeSet::new();
    let mut valid_vote_validators = Vec::new();
    let mut valid_vote_stake = 0_u64;
    for vote in &block_votes {
        let Some(validator) = chain.state().validators().get(&vote.validator) else {
            continue;
        };
        if validator.stake != vote.stake || !vote.verify_signature() {
            continue;
        }
        if seen_vote_validators.insert(vote.validator) {
            valid_vote_validators.push(vote.validator);
            valid_vote_stake = valid_vote_stake.saturating_add(vote.stake);
        }
    }
    let mut receipt_ids = Vec::new();
    let mut tensor_op_receipt_ids = Vec::new();
    let mut linear_training_receipt_ids = Vec::new();
    let mut settled_receipt_ids = Vec::new();
    for receipt in chain
        .state()
        .receipts()
        .values()
        .filter(|receipt| receipt.submitted_at_block() == height)
    {
        let receipt_id = receipt.receipt_id();
        receipt_ids.push(receipt_id);
        if chain.state().settled_receipts().contains(&receipt_id) {
            settled_receipt_ids.push(receipt_id);
        }
        match receipt.primitive_type() {
            PrimitiveType::TensorOp => tensor_op_receipt_ids.push(receipt_id),
            PrimitiveType::LinearTrainingStep => linear_training_receipt_ids.push(receipt_id),
        }
    }
    Ok(format!(
        "command=service_block\ndata_dir={data_dir}\nheight={height}\nblock_hash={}\nblock_validation=useful_verification_pow\nparent_hash={}\nproposer={}\nproposer_role=validator\nproposer_registered={}\ntensorwork_proposer_selection=false\nstate_root={}\nepoch={}\nlatest_height={}\nfinalized={}\nsettled_receipt_set_root={}\nselected_receipt_ids={}\nselected_receipt_count={}\nselected_receipt_twu={}\nselected_receipt_bytes={}\nblock_twu_cap={}\nblock_byte_cap={}\nblock_receipt_cap={}\nchecks_root={}\ncheck_leaf_count={}\nchecks_root_recomputed={}\ndifficulty_target={}\nnonce={}\npow_header_hash={}\npow_hash={}\npow_valid={}\ncanonical_blockspace_valid={}\nblock_vote_count={}\nblock_vote_validators={}\nblock_vote_stake={}\nfinality_threshold_stake={}\nfinality_validated_block={}\nreceipt_count={}\nreceipt_ids={}\ntensor_op_receipt_count={}\ntensor_op_receipt_ids={}\nlinear_training_receipt_count={}\nlinear_training_receipt_ids={}\nsettled_receipt_count={}\nsettled_receipt_ids={}\nstatus_source=node_store",
        hex(&block_hash),
        hex(&block.parent_hash),
        hex(&block.proposer),
        proposer_registered,
        hex(&block.state_root),
        block.epoch,
        chain.state().height(),
        chain.is_block_finalized(&block_hash),
        hex(&block.settled_receipt_set_root),
        hex_hash_list(&selected_receipt_ids),
        selected_receipt_ids.len(),
        selected_receipt_twu,
        selected_receipt_bytes,
        blockspace_caps.max_tensor_work_units,
        blockspace_caps.max_bytes,
        blockspace_caps.max_receipts,
        hex(&block.checks_root),
        selected_receipt_ids.len(),
        block_valid,
        hex(&block.difficulty_target),
        block.nonce,
        hex(&pow_header_hash),
        hex(&pow_hash),
        block.pow_valid(),
        block_valid,
        valid_vote_validators.len(),
        hex_hash_list(&valid_vote_validators),
        valid_vote_stake,
        finality_threshold_stake,
        chain.is_block_finalized(&block_hash) && block_valid,
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

fn finality_threshold_stake(chain: &Chain, total_validator_stake: u64) -> u64 {
    let numerator = chain.params().finality_stake_numerator;
    let denominator = chain.params().finality_stake_denominator.max(1);
    total_validator_stake
        .saturating_mul(numerator)
        .saturating_add(denominator.saturating_sub(1))
        / denominator
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
