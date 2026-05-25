use super::public_evidence_record_commands::PublicEvidenceRecordContextArgs;
use super::value_types::HashArg;
use clap::{Args, ValueHint};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactArgs {
    #[command(flatten)]
    pub context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    pub artifact: RecordArtifactLocatorArgs,
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
    #[command(flatten)]
    pub context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    pub artifact: RecordArtifactLocatorArgs,
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
    #[command(flatten)]
    pub context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    pub artifact: RecordArtifactLocatorArgs,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "File containing supporting records to summarize."
    )]
    pub record_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactLocatorArgs {
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public URI for the supporting-record artifact."
    )]
    pub artifact_uri: String,
}

impl RecordArtifactLocatorArgs {
    pub fn uri(&self) -> &str {
        &self.artifact_uri
    }
}
