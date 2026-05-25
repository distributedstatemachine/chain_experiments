use super::{
    KeyValueReportWriter, RuntimeP2pReport, RuntimeStatusSnapshot, ServiceRuntimeConfig,
    hex_hash_list, runtime_role_wallet_address_text,
};
use crate::hash::hex;

pub fn format_role_runtime_report(
    config: &ServiceRuntimeConfig,
    snapshot: &RuntimeStatusSnapshot,
    p2p: &RuntimeP2pReport<'_>,
) -> String {
    let network = &config.node.network;
    let network_events = snapshot.network_events;
    let mut report = KeyValueReportWriter::new();
    report.field("command", "service_serve");
    report.field("runtime_command", config.runtime_command);
    report.field("role", config.role.label());
    report.field("chain_profile", config.node.profile.label());
    report.field("role_loop_ready", true);
    report.field(
        "role_can_produce_blocks",
        config.node.can_produce_local_blocks(),
    );
    report.field(
        "role_wallet_address",
        runtime_role_wallet_address_text(snapshot.role_wallet_address),
    );
    report.field(
        "role_wallet_registration",
        snapshot.role_wallet_registration,
    );
    report.field("role_wallet_registered", snapshot.role_wallet_registered);
    report.field("miner_work_ready", snapshot.miner_work_ready);
    report.field(
        "miner_assigned_jobs_seen",
        snapshot.miner_assigned_jobs_seen,
    );
    report.field("miner_unreceipted_jobs", snapshot.miner_unreceipted_jobs);
    report.field(
        "miner_receipts_submitted",
        snapshot.miner_receipts_submitted,
    );
    report.field("miner_tensors_inserted", snapshot.miner_tensors_inserted);
    report.field("validator_work_ready", snapshot.validator_work_ready);
    report.field(
        "validator_assigned_receipts_seen",
        snapshot.validator_assigned_receipts_seen,
    );
    report.field(
        "validator_unattested_receipts",
        snapshot.validator_unattested_receipts,
    );
    report.field(
        "validator_artifact_ready_receipts",
        snapshot.validator_artifact_ready_receipts,
    );
    report.field(
        "validator_artifact_missing_receipts",
        snapshot.validator_artifact_missing_receipts,
    );
    report.field(
        "validator_remote_tensor_fetch_attempts",
        snapshot.validator_remote_tensor_fetch_attempts,
    );
    report.field(
        "validator_remote_tensor_fetch_successes",
        snapshot.validator_remote_tensor_fetch_successes,
    );
    report.field(
        "validator_remote_tensor_fetch_failures",
        snapshot.validator_remote_tensor_fetch_failures,
    );
    report.field(
        "validator_remote_tensor_fetch_bytes",
        snapshot.validator_remote_tensor_fetch_bytes,
    );
    report.field(
        "validator_remote_tensors_inserted",
        snapshot.validator_remote_tensors_inserted,
    );
    report.field(
        "validator_attestations_submitted",
        snapshot.validator_attestations_submitted,
    );
    report.field(
        "validator_block_votes_submitted",
        snapshot.validator_block_votes_submitted,
    );
    report.field("local_producer", snapshot.local_producer);
    report.field("listen", &network.rpc_listen);
    report.field("p2p_listen", &network.p2p_listen);
    report.field("p2p_runtime", "libp2p");
    report.field("p2p_peer_id", p2p.peer_id);
    report.field("p2p_connected_peers", snapshot.p2p_connected_peers);
    report.field(
        "p2p_observed_block_gossip_count",
        snapshot.p2p_observed_blocks,
    );
    report.field(
        "p2p_observed_block_payload_gossip_count",
        snapshot.p2p_observed_block_payloads,
    );
    report.field(
        "p2p_observed_block_vote_gossip_count",
        snapshot.p2p_observed_block_votes,
    );
    report.field("p2p_observed_job_gossip_count", snapshot.p2p_observed_jobs);
    report.field(
        "p2p_observed_receipt_gossip_count",
        snapshot.p2p_observed_receipts,
    );
    report.field(
        "p2p_observed_attestation_gossip_count",
        snapshot.p2p_observed_attestations,
    );
    report.field(
        "p2p_latest_observed_block_height",
        snapshot.p2p_latest_observed_block_height,
    );
    report.field(
        "p2p_latest_observed_block_hash",
        hex(&snapshot.p2p_latest_observed_block_hash),
    );
    report.field(
        "p2p_observed_block_hashes",
        hex_hash_list(&snapshot.p2p_observed_block_hashes),
    );
    report.field(
        "p2p_latest_observed_block_payload_height",
        snapshot.p2p_latest_observed_block_payload_height,
    );
    report.field(
        "p2p_latest_observed_block_payload_hash",
        hex(&snapshot.p2p_latest_observed_block_payload_hash),
    );
    report.field(
        "p2p_observed_block_payload_hashes",
        hex_hash_list(&snapshot.p2p_observed_block_payload_hashes),
    );
    report.field("p2p_gossipsub_topics", p2p.topics);
    report.field(
        "p2p_request_response_protocols",
        p2p.request_response_protocols,
    );
    report.field("p2p_bootstrap_peers", p2p.bootstrap_peer_count);
    report.append_report(p2p.identity);
    report.field("p2p_max_transmit_bytes", p2p.max_transmit_bytes);
    report.field("p2p_request_timeout_seconds", p2p.request_timeout_seconds);
    report.field("p2p_max_concurrent_streams", p2p.max_concurrent_streams);
    report.field("p2p_idle_timeout_seconds", p2p.idle_timeout_seconds);
    report.field("data_dir", config.node.data_dir().display());
    report.field("served_requests", snapshot.served_requests);
    report.field("produced_blocks", snapshot.produced_blocks);
    report.field("network_applied_blocks", snapshot.network_applied_blocks);
    report.field("network_events_ingested", network_events.events);
    report.field(
        "network_block_events_ingested",
        network_events.block_announcements,
    );
    report.field(
        "network_block_headers_ingested",
        network_events.block_headers,
    );
    report.field(
        "network_block_payloads_ingested",
        network_events.block_payloads,
    );
    report.field(
        "network_block_payloads_applied",
        network_events.block_payloads_applied,
    );
    report.field("network_block_votes_ingested", network_events.block_votes);
    report.field(
        "network_block_votes_applied",
        network_events.block_votes_applied,
    );
    report.field("network_job_events_ingested", network_events.jobs);
    report.field("network_job_payloads_ingested", network_events.job_payloads);
    report.field(
        "network_job_payloads_applied",
        network_events.job_payloads_applied,
    );
    report.field("network_receipt_events_ingested", network_events.receipts);
    report.field(
        "network_receipt_payloads_ingested",
        network_events.receipt_payloads,
    );
    report.field(
        "network_receipt_payloads_applied",
        network_events.receipt_payloads_applied,
    );
    report.field(
        "network_attestation_events_ingested",
        network_events.attestations,
    );
    report.field(
        "network_attestation_payloads_ingested",
        network_events.attestation_payloads,
    );
    report.field(
        "network_attestation_payloads_applied",
        network_events.attestation_payloads_applied,
    );
    report.field("network_peer_events_ingested", network_events.peers);
    report.field("network_invalid_events", network_events.invalid_events);
    report.finish()
}

pub fn write_role_runtime_status(
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
