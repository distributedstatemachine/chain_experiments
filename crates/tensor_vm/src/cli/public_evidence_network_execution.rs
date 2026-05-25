use super::network_evidence::{
    NetworkObservationEvidenceLine, network_observation_evidence_line,
    network_observation_evidence_line_from_service_log,
};
use super::public_evidence_commands::EvidenceNetworkCommand;
use super::validation::path_argument;
use crate::error::{Result, TvmError};
use crate::types::Hash;

pub(super) fn execute_public_evidence_network_command(
    command: &EvidenceNetworkCommand,
) -> Result<String> {
    match command {
        EvidenceNetworkCommand::Observation(args) => {
            network_observation_evidence_line(NetworkObservationEvidenceLine {
                operator_id: args.target.operator.operator_id.into_hash(),
                peer_id: &args.peer_id.to_string(),
                listen_address: &args.target.listen_address.to_string(),
                observed_at_unix_seconds: args.target.observation.observed_at,
                gossip_topic_count: args.protocol_counts.gossip_topic_count,
                request_response_protocol_count: args
                    .protocol_counts
                    .request_response_protocol_count,
                bootstrap_peer_count: args.protocol_counts.bootstrap_peer_count,
                max_transmit_bytes: args.transport_limits.max_transmit_bytes,
                request_timeout_seconds: args.transport_limits.request_timeout_seconds,
                max_concurrent_streams: args.transport_limits.max_concurrent_streams,
                idle_connection_timeout_seconds: args.transport_limits.idle_timeout_seconds,
            })
        }
        EvidenceNetworkCommand::FromServiceLog(args) => network_observation_from_service_log(
            args.target.operator.operator_id.into_hash(),
            &args.target.listen_address.to_string(),
            args.target.observation.observed_at,
            &path_argument(&args.service_log),
        ),
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
