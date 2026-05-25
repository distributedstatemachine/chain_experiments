use super::public_evidence_record_commands::{
    PublicEvidenceRecordContextArgs, RecordFileArgs, RecordRootArgs, RecordRootsArgs,
};
use clap::{Args, ValueHint};

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RecordArtifactArgs {
    #[command(flatten)]
    pub(crate) context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    pub(crate) artifact: RecordArtifactLocatorArgs,
    #[command(flatten)]
    pub(crate) root: RecordRootArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RecordArtifactFromRootsArgs {
    #[command(flatten)]
    pub(crate) context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    pub(crate) artifact: RecordArtifactLocatorArgs,
    #[command(flatten)]
    pub(crate) roots: RecordRootsArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RecordArtifactFromFileArgs {
    #[command(flatten)]
    pub(crate) context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    pub(crate) artifact: RecordArtifactLocatorArgs,
    #[command(flatten)]
    pub(crate) file: RecordFileArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RecordArtifactLocatorArgs {
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public URI for the supporting-record artifact."
    )]
    pub(crate) artifact_uri: String,
}
