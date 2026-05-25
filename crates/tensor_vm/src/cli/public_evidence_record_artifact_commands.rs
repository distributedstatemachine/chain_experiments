use super::public_evidence_record_commands::{
    PublicEvidenceRecordContextArgs, RecordRootArgs, RecordRootsArgs,
};
use clap::{Args, ValueHint};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactArgs {
    #[command(flatten)]
    pub context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    pub artifact: RecordArtifactLocatorArgs,
    #[command(flatten)]
    pub root: RecordRootArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactFromRootsArgs {
    #[command(flatten)]
    pub context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    pub artifact: RecordArtifactLocatorArgs,
    #[command(flatten)]
    pub roots: RecordRootsArgs,
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
