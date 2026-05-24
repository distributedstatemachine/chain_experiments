use super::parser_values::parse_hash_value;
use crate::types::{Address, Hash};
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RunWindowArgs {
    #[arg(long, value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long)]
    pub started_at: u64,
    #[arg(long)]
    pub ended_at: u64,
    #[arg(long)]
    pub observed_blocks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RunWindowFromFileArgs {
    #[arg(long, value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long)]
    pub block_observation_file: String,
}
