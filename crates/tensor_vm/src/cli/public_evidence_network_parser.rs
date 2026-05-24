use super::parser_values::parse_hash_value;
use crate::types::Hash;
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NetworkObservationArgs {
    #[arg(long, value_parser = parse_hash_value)]
    pub operator_id: Hash,
    #[arg(long)]
    pub peer_id: String,
    #[arg(long)]
    pub listen_address: String,
    #[arg(long)]
    pub observed_at: u64,
    #[arg(long)]
    pub gossip_topics: u64,
    #[arg(long)]
    pub request_response_protocols: u64,
    #[arg(long)]
    pub bootstrap_peers: u64,
    #[arg(long)]
    pub max_transmit_bytes: u64,
    #[arg(long)]
    pub request_timeout_seconds: u64,
    #[arg(long)]
    pub max_concurrent_streams: u64,
    #[arg(long)]
    pub idle_timeout_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NetworkObservationFromServiceLogArgs {
    #[arg(long, value_parser = parse_hash_value)]
    pub operator_id: Hash,
    #[arg(long)]
    pub listen_address: String,
    #[arg(long)]
    pub observed_at: u64,
    #[arg(long)]
    pub service_log: String,
}
