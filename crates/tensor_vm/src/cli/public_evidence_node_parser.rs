use super::parser_values::{PublicNodeRoleArg, parse_hash_value};
use crate::types::{Address, Hash};
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeHeartbeatArgs {
    #[arg(long)]
    pub role: PublicNodeRoleArg,
    #[arg(long, value_parser = parse_hash_value)]
    pub address: Address,
    #[arg(long, value_parser = parse_hash_value)]
    pub operator_id: Hash,
    #[arg(long)]
    pub first_block: u64,
    #[arg(long)]
    pub last_block: u64,
    #[arg(long)]
    pub heartbeat_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeHeartbeatFromFileArgs {
    #[arg(long)]
    pub role: PublicNodeRoleArg,
    #[arg(long, value_parser = parse_hash_value)]
    pub address: Address,
    #[arg(long, value_parser = parse_hash_value)]
    pub operator_id: Hash,
    #[arg(long)]
    pub heartbeat_file: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct OperatorAttestationArgs {
    #[arg(long)]
    pub role: PublicNodeRoleArg,
    #[arg(long, value_parser = parse_hash_value)]
    pub address: Address,
    #[arg(long, value_parser = parse_hash_value)]
    pub operator_id: Hash,
    #[arg(long)]
    pub identity_uri: String,
    #[arg(long)]
    pub observed_at: u64,
}
