use super::parser_values::parse_hash_value;
use crate::types::{Address, Hash};
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicationArgs {
    #[arg(long, value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long)]
    pub public_uri: String,
    #[arg(long, value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long)]
    pub manifest_signature_count: u64,
    #[arg(long)]
    pub independent_auditor_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct AuditorRecordArgs {
    #[arg(long, value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long)]
    pub public_uri: String,
    #[arg(long, value_parser = parse_hash_value)]
    pub auditor_id: Hash,
    #[arg(long)]
    pub audit_uri: String,
    #[arg(long)]
    pub observed_at: u64,
}
