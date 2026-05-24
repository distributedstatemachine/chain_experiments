use super::parser_values::{PublicServiceKindArg, parse_hash_value};
use crate::types::Hash;
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceHealthArgs {
    #[arg(long)]
    pub kind: PublicServiceKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    pub endpoint_id: Hash,
    #[arg(long)]
    pub public_url: String,
    #[arg(long)]
    pub health_path: String,
    #[arg(long)]
    pub first_block: u64,
    #[arg(long)]
    pub last_block: u64,
    #[arg(long)]
    pub reachable_count: u64,
    #[arg(long)]
    pub signed_health_check_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceHealthFromFileArgs {
    #[arg(long)]
    pub kind: PublicServiceKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    pub endpoint_id: Hash,
    #[arg(long)]
    pub public_url: String,
    #[arg(long)]
    pub health_path: String,
    #[arg(long)]
    pub observation_file: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentArgs {
    #[arg(long)]
    pub kind: PublicServiceKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    pub endpoint_id: Hash,
    #[arg(long)]
    pub public_url: String,
    #[arg(long)]
    pub content_path: String,
    #[arg(long, value_parser = parse_hash_value)]
    pub content_root: Hash,
    #[arg(long)]
    pub observed_at: u64,
    #[arg(long)]
    pub min_content_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentFromBytesArgs {
    #[arg(long)]
    pub kind: PublicServiceKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    pub endpoint_id: Hash,
    #[arg(long)]
    pub public_url: String,
    #[arg(long)]
    pub content_path: String,
    #[arg(long)]
    pub observed_at: u64,
    #[arg(long)]
    pub content_hex: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentFromFileArgs {
    #[arg(long)]
    pub kind: PublicServiceKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    pub endpoint_id: Hash,
    #[arg(long)]
    pub public_url: String,
    #[arg(long)]
    pub content_path: String,
    #[arg(long)]
    pub observed_at: u64,
    #[arg(long)]
    pub content_file: String,
}
