use super::commands::PublicEvidenceCommand;
use super::network_evidence::{
    NetworkObservationEvidenceLine, network_observation_evidence_line,
    network_observation_evidence_line_from_service_log,
};
use crate::error::{Result, TvmError};
use crate::types::Hash;

pub(super) fn execute_public_evidence_network_command(
    command: &PublicEvidenceCommand,
) -> Option<Result<String>> {
    match command {
        PublicEvidenceCommand::NetworkObservation(args) => Some(network_observation_evidence_line(
            NetworkObservationEvidenceLine {
                operator_id: args.operator_id,
                peer_id: &args.peer_id,
                listen_address: &args.listen_address,
                observed_at_unix_seconds: args.observed_at,
                gossip_topic_count: args.gossip_topics,
                request_response_protocol_count: args.request_response_protocols,
                bootstrap_peer_count: args.bootstrap_peers,
                max_transmit_bytes: args.max_transmit_bytes,
                request_timeout_seconds: args.request_timeout_seconds,
                max_concurrent_streams: args.max_concurrent_streams,
                idle_connection_timeout_seconds: args.idle_timeout_seconds,
            },
        )),
        PublicEvidenceCommand::NetworkObservationFromServiceLog(args) => {
            Some(network_observation_from_service_log(
                args.operator_id,
                &args.listen_address,
                args.observed_at,
                &args.service_log,
            ))
        }
        _ => None,
    }
}

fn network_observation_from_service_log(
    operator_id: Hash,
    listen_address: &str,
    observed_at_unix_seconds: u64,
    service_log: &str,
) -> Result<String> {
    let log_contents = std::fs::read_to_string(service_log)
        .map_err(|_| TvmError::Storage("failed to read service log file"))?;
    network_observation_evidence_line_from_service_log(
        operator_id,
        listen_address,
        observed_at_unix_seconds,
        &log_contents,
    )
}
