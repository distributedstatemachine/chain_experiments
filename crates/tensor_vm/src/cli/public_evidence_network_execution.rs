use super::network_evidence::{
    NetworkObservationEvidenceLine, network_observation_evidence_line,
    network_observation_evidence_line_from_service_log,
};
use super::public_evidence_commands::EvidenceNetworkCommand;
use super::public_evidence_network_commands::{
    NetworkObservationArgs, NetworkObservationProtocolCountsArgs, NetworkObservationTargetArgs,
    NetworkObservationTransportLimitsArgs,
};
use super::validation::path_argument;
use crate::error::{Result, TvmError};
use crate::types::Hash;

pub(super) fn execute_public_evidence_network_command(
    command: &EvidenceNetworkCommand,
) -> Result<String> {
    match command {
        EvidenceNetworkCommand::Observation(args) => network_observation_from_args(args),
        EvidenceNetworkCommand::FromServiceLog(args) => network_observation_from_service_log(
            network_observation_target(&args.target),
            &path_argument(&args.service_log),
        ),
    }
}

fn network_observation_from_args(args: &NetworkObservationArgs) -> Result<String> {
    let target = network_observation_target(&args.target);
    let protocol_counts = network_observation_protocol_counts(&args.protocol_counts);
    let transport_limits = network_observation_transport_limits(&args.transport_limits);
    let peer_id = args.peer_id.to_string();
    network_observation_evidence_line(NetworkObservationEvidenceLine {
        operator_id: target.operator_id,
        peer_id: &peer_id,
        listen_address: &target.listen_address,
        observed_at_unix_seconds: target.observed_at,
        gossip_topic_count: protocol_counts.gossip_topic_count,
        request_response_protocol_count: protocol_counts.request_response_protocol_count,
        bootstrap_peer_count: protocol_counts.bootstrap_peer_count,
        max_transmit_bytes: transport_limits.max_transmit_bytes,
        request_timeout_seconds: transport_limits.request_timeout_seconds,
        max_concurrent_streams: transport_limits.max_concurrent_streams,
        idle_connection_timeout_seconds: transport_limits.idle_connection_timeout_seconds,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NetworkObservationTargetContext {
    operator_id: Hash,
    listen_address: String,
    observed_at: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NetworkObservationProtocolCounts {
    gossip_topic_count: u64,
    request_response_protocol_count: u64,
    bootstrap_peer_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NetworkObservationTransportLimits {
    max_transmit_bytes: u64,
    request_timeout_seconds: u64,
    max_concurrent_streams: u64,
    idle_connection_timeout_seconds: u64,
}

fn network_observation_target(
    args: &NetworkObservationTargetArgs,
) -> NetworkObservationTargetContext {
    NetworkObservationTargetContext {
        operator_id: args.operator.operator_id.into_hash(),
        listen_address: args.listen_address.to_string(),
        observed_at: args.observation.observed_at,
    }
}

fn network_observation_protocol_counts(
    args: &NetworkObservationProtocolCountsArgs,
) -> NetworkObservationProtocolCounts {
    NetworkObservationProtocolCounts {
        gossip_topic_count: args.gossip_topic_count,
        request_response_protocol_count: args.request_response_protocol_count,
        bootstrap_peer_count: args.bootstrap_peer_count,
    }
}

fn network_observation_transport_limits(
    args: &NetworkObservationTransportLimitsArgs,
) -> NetworkObservationTransportLimits {
    NetworkObservationTransportLimits {
        max_transmit_bytes: args.max_transmit_bytes,
        request_timeout_seconds: args.request_timeout_seconds,
        max_concurrent_streams: args.max_concurrent_streams,
        idle_connection_timeout_seconds: args.idle_timeout_seconds,
    }
}

fn network_observation_from_service_log(
    target: NetworkObservationTargetContext,
    service_log: &str,
) -> Result<String> {
    let log_contents = std::fs::read_to_string(service_log)
        .map_err(|_| TvmError::Storage("failed to read service log file"))?;
    network_observation_evidence_line_from_service_log(
        target.operator_id,
        &target.listen_address,
        target.observed_at,
        &log_contents,
    )
}
