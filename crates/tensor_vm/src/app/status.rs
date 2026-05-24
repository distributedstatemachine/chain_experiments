use std::{collections::BTreeMap, path::Path};

use crate::{NodeStore, hash::hex};

pub fn hex_hash_list(hashes: &[[u8; 32]]) -> String {
    if hashes.is_empty() {
        return "none".to_owned();
    }
    hashes
        .iter()
        .map(|hash| hex(hash))
        .collect::<Vec<_>>()
        .join(",")
}

struct StatusFileFields {
    fields: BTreeMap<String, String>,
}

impl StatusFileFields {
    fn from_path(path: impl AsRef<Path>) -> Self {
        let fields = std::fs::read_to_string(path)
            .ok()
            .map(|contents| status_file_fields(&contents))
            .unwrap_or_default();
        Self { fields }
    }

    fn value(&self, key: &str) -> String {
        self.fields
            .get(key)
            .cloned()
            .unwrap_or_else(|| "unknown".to_owned())
    }
}

fn status_file_fields(contents: &str) -> BTreeMap<String, String> {
    let mut fields = BTreeMap::new();
    for line in contents.lines() {
        if let Some((key, value)) = line.split_once('=') {
            fields
                .entry(key.to_owned())
                .or_insert_with(|| value.to_owned());
        }
    }
    fields
}

pub fn service_status(data_dir: &str) -> std::result::Result<String, String> {
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
    let ready_status = StatusFileFields::from_path(Path::new(data_dir).join("local-cpu-ready"));
    let role_runtime_status =
        StatusFileFields::from_path(Path::new(data_dir).join("role-runtime.status"));
    let ready = |key| ready_status.value(key);
    let role = |key| role_runtime_status.value(key);
    Ok(format!(
        "command=service_status\ndata_dir={}\noperator_name={}\noperator_id={}\nrole={}\nruntime_command={}\nrole_runtime_command={}\nrole_loop_ready={}\nrole_loop_role={}\nrole_chain_profile={}\nrole_can_produce_blocks={}\nrole_wallet_address={}\nrole_wallet_registration={}\nrole_wallet_registered={}\nrole_miner_work_ready={}\nrole_miner_assigned_jobs_seen={}\nrole_miner_unreceipted_jobs={}\nrole_miner_receipts_submitted={}\nrole_miner_tensors_inserted={}\nrole_validator_work_ready={}\nrole_validator_assigned_receipts_seen={}\nrole_validator_unattested_receipts={}\nrole_validator_artifact_ready_receipts={}\nrole_validator_artifact_missing_receipts={}\nrole_validator_remote_tensor_fetch_attempts={}\nrole_validator_remote_tensor_fetch_successes={}\nrole_validator_remote_tensor_fetch_failures={}\nrole_validator_remote_tensor_fetch_bytes={}\nrole_validator_remote_tensors_inserted={}\nrole_validator_attestations_submitted={}\nrole_validator_block_votes_submitted={}\nrole_local_producer={}\nrole_served_requests={}\nrole_produced_blocks={}\nrole_network_applied_blocks={}\nrole_network_events_ingested={}\nrole_network_block_events_ingested={}\nrole_network_block_headers_ingested={}\nrole_network_block_payloads_ingested={}\nrole_network_block_payloads_applied={}\nrole_network_block_votes_ingested={}\nrole_network_block_votes_applied={}\nrole_network_job_events_ingested={}\nrole_network_job_payloads_ingested={}\nrole_network_job_payloads_applied={}\nrole_network_receipt_events_ingested={}\nrole_network_receipt_payloads_ingested={}\nrole_network_receipt_payloads_applied={}\nrole_network_attestation_events_ingested={}\nrole_network_attestation_payloads_ingested={}\nrole_network_attestation_payloads_applied={}\nrole_network_peer_events_ingested={}\nrole_network_invalid_events={}\nrole_latest_height={}\nrole_p2p_connected_peers={}\nrole_p2p_observed_blocks={}\nrole_p2p_observed_block_payloads={}\nrole_p2p_observed_block_votes={}\nrole_p2p_observed_jobs={}\nrole_p2p_observed_receipts={}\nrole_p2p_observed_attestations={}\nrole_p2p_latest_observed_block_height={}\nrole_p2p_latest_observed_block_hash={}\nrole_p2p_observed_block_hashes={}\nrole_p2p_latest_observed_block_payload_height={}\nrole_p2p_latest_observed_block_payload_hash={}\nrole_p2p_observed_block_payload_hashes={}\nnode_multiaddr={}\np2p_peer_id={}\nheight={}\nepoch={}\nblock_count={}\nlatest_block_height={latest_block_height}\nlatest_block_hash={}\nstate_root={}\nblock_log_root={}\nfinalized_block_count={finalized_block_count}\nfirst_live_block_height={first_live_block_height}\nfirst_live_block_hash={}\nregistered_miner_count={}\nregistered_validator_count={}\njob_count={}\nreceipt_count={}\nsettled_receipt_count={}\nattestation_count={attestation_count}\nreward_account_count={reward_account_count}\nmodel_count={}\nbootstrap_peer_count={bootstrap_peer_count}\nnode_store_ready=true\nstatus_source=node_store",
        status.data_dir.display(),
        ready("operator_name"),
        ready("operator_id"),
        ready("role"),
        ready("runtime_command"),
        role("role_runtime_command"),
        role("role_loop_ready"),
        role("role_loop_role"),
        role("role_chain_profile"),
        role("role_can_produce_blocks"),
        role("role_wallet_address"),
        role("role_wallet_registration"),
        role("role_wallet_registered"),
        role("role_miner_work_ready"),
        role("role_miner_assigned_jobs_seen"),
        role("role_miner_unreceipted_jobs"),
        role("role_miner_receipts_submitted"),
        role("role_miner_tensors_inserted"),
        role("role_validator_work_ready"),
        role("role_validator_assigned_receipts_seen"),
        role("role_validator_unattested_receipts"),
        role("role_validator_artifact_ready_receipts"),
        role("role_validator_artifact_missing_receipts"),
        role("role_validator_remote_tensor_fetch_attempts"),
        role("role_validator_remote_tensor_fetch_successes"),
        role("role_validator_remote_tensor_fetch_failures"),
        role("role_validator_remote_tensor_fetch_bytes"),
        role("role_validator_remote_tensors_inserted"),
        role("role_validator_attestations_submitted"),
        role("role_validator_block_votes_submitted"),
        role("role_local_producer"),
        role("role_served_requests"),
        role("role_produced_blocks"),
        role("role_network_applied_blocks"),
        role("role_network_events_ingested"),
        role("role_network_block_events_ingested"),
        role("role_network_block_headers_ingested"),
        role("role_network_block_payloads_ingested"),
        role("role_network_block_payloads_applied"),
        role("role_network_block_votes_ingested"),
        role("role_network_block_votes_applied"),
        role("role_network_job_events_ingested"),
        role("role_network_job_payloads_ingested"),
        role("role_network_job_payloads_applied"),
        role("role_network_receipt_events_ingested"),
        role("role_network_receipt_payloads_ingested"),
        role("role_network_receipt_payloads_applied"),
        role("role_network_attestation_events_ingested"),
        role("role_network_attestation_payloads_ingested"),
        role("role_network_attestation_payloads_applied"),
        role("role_network_peer_events_ingested"),
        role("role_network_invalid_events"),
        role("role_latest_height"),
        role("role_p2p_connected_peers"),
        role("role_p2p_observed_blocks"),
        role("role_p2p_observed_block_payloads"),
        role("role_p2p_observed_block_votes"),
        role("role_p2p_observed_jobs"),
        role("role_p2p_observed_receipts"),
        role("role_p2p_observed_attestations"),
        role("role_p2p_latest_observed_block_height"),
        role("role_p2p_latest_observed_block_hash"),
        role("role_p2p_observed_block_hashes"),
        role("role_p2p_latest_observed_block_payload_height"),
        role("role_p2p_latest_observed_block_payload_hash"),
        role("role_p2p_observed_block_payload_hashes"),
        ready("node_multiaddr"),
        ready("p2p_peer_id"),
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
