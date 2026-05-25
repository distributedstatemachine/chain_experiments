use super::public_evidence_record_commands::PublicEvidenceRecordKindArg;
use super::value_types::{AddressArg, HashArg};
use clap::{Args, ValueHint};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactArgs {
    #[arg(long, help = "Supporting-record class being published.")]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX", help = "Public evidence bundle identifier.")]
    pub bundle_id: HashArg,
    #[arg(
        long,
        value_name = "HEX",
        help = "Address signing the evidence manifest."
    )]
    pub manifest_signer: AddressArg,
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public URI for the supporting-record artifact."
    )]
    pub artifact_uri: String,
    #[arg(
        long,
        value_name = "HEX",
        help = "Root hash of the supporting-record set."
    )]
    pub record_root: HashArg,
    #[arg(
        long,
        value_name = "N",
        help = "Number of records covered by the artifact."
    )]
    pub record_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactFromRootsArgs {
    #[arg(long, help = "Supporting-record class being published.")]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX", help = "Public evidence bundle identifier.")]
    pub bundle_id: HashArg,
    #[arg(
        long,
        value_name = "HEX",
        help = "Address signing the evidence manifest."
    )]
    pub manifest_signer: AddressArg,
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public URI for the supporting-record artifact."
    )]
    pub artifact_uri: String,
    #[arg(
        long,
        value_name = "HEX[,HEX...]",
        value_delimiter = ',',
        num_args = 1..,
        help = "Comma-delimited record roots to aggregate."
    )]
    pub record_roots: Vec<HashArg>,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactFromFileArgs {
    #[arg(long, help = "Supporting-record class being published.")]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX", help = "Public evidence bundle identifier.")]
    pub bundle_id: HashArg,
    #[arg(
        long,
        value_name = "HEX",
        help = "Address signing the evidence manifest."
    )]
    pub manifest_signer: AddressArg,
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public URI for the supporting-record artifact."
    )]
    pub artifact_uri: String,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "File containing supporting records to summarize."
    )]
    pub record_file: PathBuf,
}
