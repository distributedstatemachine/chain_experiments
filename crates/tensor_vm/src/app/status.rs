use std::{collections::BTreeMap, path::Path};

use super::{KeyValueReport, KeyValueReportWriter};
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
            .map(|contents| KeyValueReport::parse_lenient(&contents).into_owned())
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
    let mut report = KeyValueReportWriter::new();
    report.field("command", "service_status");
    report.field("data_dir", status.data_dir.display());
    report.field("operator_name", ready("operator_name"));
    report.field("operator_id", ready("operator_id"));
    report.field("role", ready("role"));
    report.field("runtime_command", ready("runtime_command"));
    report.field("role_runtime_command", role("role_runtime_command"));
    report.field("role_loop_ready", role("role_loop_ready"));
    report.field("role_loop_role", role("role_loop_role"));
    report.field("role_chain_profile", role("role_chain_profile"));
    report.field("role_can_produce_blocks", role("role_can_produce_blocks"));
    report.field("role_wallet_address", role("role_wallet_address"));
    report.field("role_wallet_registration", role("role_wallet_registration"));
    report.field("role_wallet_registered", role("role_wallet_registered"));
    report.field("role_miner_work_ready", role("role_miner_work_ready"));
    report.field(
        "role_miner_assigned_jobs_seen",
        role("role_miner_assigned_jobs_seen"),
    );
    report.field(
        "role_miner_unreceipted_jobs",
        role("role_miner_unreceipted_jobs"),
    );
    report.field(
        "role_miner_receipts_submitted",
        role("role_miner_receipts_submitted"),
    );
    report.field(
        "role_miner_tensors_inserted",
        role("role_miner_tensors_inserted"),
    );
    report.field(
        "role_validator_work_ready",
        role("role_validator_work_ready"),
    );
    report.field(
        "role_validator_assigned_receipts_seen",
        role("role_validator_assigned_receipts_seen"),
    );
    report.field(
        "role_validator_unattested_receipts",
        role("role_validator_unattested_receipts"),
    );
    report.field(
        "role_validator_artifact_ready_receipts",
        role("role_validator_artifact_ready_receipts"),
    );
    report.field(
        "role_validator_artifact_missing_receipts",
        role("role_validator_artifact_missing_receipts"),
    );
    report.field(
        "role_validator_remote_tensor_fetch_attempts",
        role("role_validator_remote_tensor_fetch_attempts"),
    );
    report.field(
        "role_validator_remote_tensor_fetch_successes",
        role("role_validator_remote_tensor_fetch_successes"),
    );
    report.field(
        "role_validator_remote_tensor_fetch_failures",
        role("role_validator_remote_tensor_fetch_failures"),
    );
    report.field(
        "role_validator_remote_tensor_fetch_bytes",
        role("role_validator_remote_tensor_fetch_bytes"),
    );
    report.field(
        "role_validator_remote_tensors_inserted",
        role("role_validator_remote_tensors_inserted"),
    );
    report.field(
        "role_validator_attestations_submitted",
        role("role_validator_attestations_submitted"),
    );
    report.field(
        "role_validator_block_votes_submitted",
        role("role_validator_block_votes_submitted"),
    );
    report.field("role_local_producer", role("role_local_producer"));
    report.field("role_served_requests", role("role_served_requests"));
    report.field("role_produced_blocks", role("role_produced_blocks"));
    report.field(
        "role_network_applied_blocks",
        role("role_network_applied_blocks"),
    );
    report.field(
        "role_network_events_ingested",
        role("role_network_events_ingested"),
    );
    report.field(
        "role_network_block_events_ingested",
        role("role_network_block_events_ingested"),
    );
    report.field(
        "role_network_block_headers_ingested",
        role("role_network_block_headers_ingested"),
    );
    report.field(
        "role_network_block_payloads_ingested",
        role("role_network_block_payloads_ingested"),
    );
    report.field(
        "role_network_block_payloads_applied",
        role("role_network_block_payloads_applied"),
    );
    report.field(
        "role_network_block_votes_ingested",
        role("role_network_block_votes_ingested"),
    );
    report.field(
        "role_network_block_votes_applied",
        role("role_network_block_votes_applied"),
    );
    report.field(
        "role_network_job_events_ingested",
        role("role_network_job_events_ingested"),
    );
    report.field(
        "role_network_job_payloads_ingested",
        role("role_network_job_payloads_ingested"),
    );
    report.field(
        "role_network_job_payloads_applied",
        role("role_network_job_payloads_applied"),
    );
    report.field(
        "role_network_receipt_events_ingested",
        role("role_network_receipt_events_ingested"),
    );
    report.field(
        "role_network_receipt_payloads_ingested",
        role("role_network_receipt_payloads_ingested"),
    );
    report.field(
        "role_network_receipt_payloads_applied",
        role("role_network_receipt_payloads_applied"),
    );
    report.field(
        "role_network_attestation_events_ingested",
        role("role_network_attestation_events_ingested"),
    );
    report.field(
        "role_network_attestation_payloads_ingested",
        role("role_network_attestation_payloads_ingested"),
    );
    report.field(
        "role_network_attestation_payloads_applied",
        role("role_network_attestation_payloads_applied"),
    );
    report.field(
        "role_network_peer_events_ingested",
        role("role_network_peer_events_ingested"),
    );
    report.field(
        "role_network_invalid_events",
        role("role_network_invalid_events"),
    );
    report.field("role_latest_height", role("role_latest_height"));
    report.field("role_p2p_connected_peers", role("role_p2p_connected_peers"));
    report.field("role_p2p_observed_blocks", role("role_p2p_observed_blocks"));
    report.field(
        "role_p2p_observed_block_payloads",
        role("role_p2p_observed_block_payloads"),
    );
    report.field(
        "role_p2p_observed_block_votes",
        role("role_p2p_observed_block_votes"),
    );
    report.field("role_p2p_observed_jobs", role("role_p2p_observed_jobs"));
    report.field(
        "role_p2p_observed_receipts",
        role("role_p2p_observed_receipts"),
    );
    report.field(
        "role_p2p_observed_attestations",
        role("role_p2p_observed_attestations"),
    );
    report.field(
        "role_p2p_latest_observed_block_height",
        role("role_p2p_latest_observed_block_height"),
    );
    report.field(
        "role_p2p_latest_observed_block_hash",
        role("role_p2p_latest_observed_block_hash"),
    );
    report.field(
        "role_p2p_observed_block_hashes",
        role("role_p2p_observed_block_hashes"),
    );
    report.field(
        "role_p2p_latest_observed_block_payload_height",
        role("role_p2p_latest_observed_block_payload_height"),
    );
    report.field(
        "role_p2p_latest_observed_block_payload_hash",
        role("role_p2p_latest_observed_block_payload_hash"),
    );
    report.field(
        "role_p2p_observed_block_payload_hashes",
        role("role_p2p_observed_block_payload_hashes"),
    );
    report.field("node_multiaddr", ready("node_multiaddr"));
    report.field("p2p_peer_id", ready("p2p_peer_id"));
    report.field("height", chain.state().height());
    report.field("epoch", chain.state().epoch());
    report.field("block_count", status.block_count);
    report.field("latest_block_height", latest_block_height);
    report.field("latest_block_hash", hex(&status.latest_block_hash));
    report.field("state_root", hex(&chain.state_root()));
    report.field("block_log_root", hex(&status.block_log_root));
    report.field("finalized_block_count", finalized_block_count);
    report.field("first_live_block_height", first_live_block_height);
    report.field("first_live_block_hash", hex(&first_live_block_hash));
    report.field("registered_miner_count", chain.state().miners().len());
    report.field(
        "registered_validator_count",
        chain.state().validators().len(),
    );
    report.field("job_count", chain.state().jobs().len());
    report.field("receipt_count", chain.state().receipts().len());
    report.field(
        "settled_receipt_count",
        chain.state().settled_receipts().len(),
    );
    report.field("attestation_count", attestation_count);
    report.field("reward_account_count", reward_account_count);
    report.field("model_count", chain.state().model_states().len());
    report.field("bootstrap_peer_count", bootstrap_peer_count);
    report.field("node_store_ready", true);
    report.field("status_source", "node_store");
    Ok(report.finish())
}
