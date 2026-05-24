use super::CliCommand;
use super::parser_values::parse_hash_value;
use crate::types::Hash;
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct NetworkObservationArgs {
    #[arg(long, value_parser = parse_hash_value)]
    operator_id: Hash,
    #[arg(long)]
    peer_id: String,
    #[arg(long)]
    listen_address: String,
    #[arg(long)]
    observed_at: u64,
    #[arg(long)]
    gossip_topics: u64,
    #[arg(long)]
    request_response_protocols: u64,
    #[arg(long)]
    bootstrap_peers: u64,
    #[arg(long)]
    max_transmit_bytes: u64,
    #[arg(long)]
    request_timeout_seconds: u64,
    #[arg(long)]
    max_concurrent_streams: u64,
    #[arg(long)]
    idle_timeout_seconds: u64,
}

impl NetworkObservationArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceNetworkObservation {
            operator_id: self.operator_id,
            peer_id: self.peer_id,
            listen_address: self.listen_address,
            observed_at_unix_seconds: self.observed_at,
            gossip_topic_count: self.gossip_topics,
            request_response_protocol_count: self.request_response_protocols,
            bootstrap_peer_count: self.bootstrap_peers,
            max_transmit_bytes: self.max_transmit_bytes,
            request_timeout_seconds: self.request_timeout_seconds,
            max_concurrent_streams: self.max_concurrent_streams,
            idle_connection_timeout_seconds: self.idle_timeout_seconds,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct NetworkObservationFromServiceLogArgs {
    #[arg(long, value_parser = parse_hash_value)]
    operator_id: Hash,
    #[arg(long)]
    listen_address: String,
    #[arg(long)]
    observed_at: u64,
    #[arg(long)]
    service_log: String,
}

impl NetworkObservationFromServiceLogArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceNetworkObservationFromServiceLog {
            operator_id: self.operator_id,
            listen_address: self.listen_address,
            observed_at_unix_seconds: self.observed_at,
            service_log: self.service_log,
        }
    }
}
