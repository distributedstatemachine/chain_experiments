use std::path::Path;
use tensor_vm::{NodeStore, hash::hex};

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
