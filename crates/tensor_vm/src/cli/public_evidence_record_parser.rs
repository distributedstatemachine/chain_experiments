use super::parser_values::{
    HashList, PublicEvidenceRecordKindArg, parse_hash_list_value, parse_hash_value,
};
use crate::types::{Address, Hash};
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordSummaryArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long, value_parser = parse_hash_value)]
    pub record_root: Hash,
    #[arg(long)]
    pub record_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long)]
    pub artifact_uri: String,
    #[arg(long, value_parser = parse_hash_value)]
    pub record_root: Hash,
    #[arg(long)]
    pub record_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactFromRootsArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long)]
    pub artifact_uri: String,
    #[arg(long, value_parser = parse_hash_list_value)]
    pub record_roots: HashList,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactFromFileArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long)]
    pub artifact_uri: String,
    #[arg(long)]
    pub record_file: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordSummaryFromRootsArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long, value_parser = parse_hash_list_value)]
    pub record_roots: HashList,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordSummaryFromFileArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long)]
    pub record_file: String,
}
