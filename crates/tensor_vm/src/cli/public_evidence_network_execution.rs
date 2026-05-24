use super::CliCommand;
use super::network_evidence::{
    NetworkObservationEvidenceLine, network_observation_evidence_line,
    network_observation_evidence_line_from_service_log,
};
use crate::error::{Result, TvmError};
use crate::types::Hash;

pub(super) fn execute_public_evidence_network_command(
    command: &CliCommand,
) -> Option<Result<String>> {
    match command {
        CliCommand::PublicEvidenceNetworkObservation {
            operator_id,
            peer_id,
            listen_address,
            observed_at_unix_seconds,
            gossip_topic_count,
            request_response_protocol_count,
            bootstrap_peer_count,
            max_transmit_bytes,
            request_timeout_seconds,
            max_concurrent_streams,
            idle_connection_timeout_seconds,
        } => Some(network_observation_evidence_line(
            NetworkObservationEvidenceLine {
                operator_id: *operator_id,
                peer_id,
                listen_address,
                observed_at_unix_seconds: *observed_at_unix_seconds,
                gossip_topic_count: *gossip_topic_count,
                request_response_protocol_count: *request_response_protocol_count,
                bootstrap_peer_count: *bootstrap_peer_count,
                max_transmit_bytes: *max_transmit_bytes,
                request_timeout_seconds: *request_timeout_seconds,
                max_concurrent_streams: *max_concurrent_streams,
                idle_connection_timeout_seconds: *idle_connection_timeout_seconds,
            },
        )),
        CliCommand::PublicEvidenceNetworkObservationFromServiceLog {
            operator_id,
            listen_address,
            observed_at_unix_seconds,
            service_log,
        } => Some(network_observation_from_service_log(
            *operator_id,
            listen_address,
            *observed_at_unix_seconds,
            service_log,
        )),
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
