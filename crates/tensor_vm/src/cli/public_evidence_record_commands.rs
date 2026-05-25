use super::public_evidence_record_artifact_commands::{
    RecordArtifactArgs, RecordArtifactFromFileArgs, RecordArtifactFromRootsArgs,
};
use super::value_types::{AddressArg, HashArg};
use crate::testnet::PublicEvidenceRecordKind;
use clap::{Args, Subcommand, ValueEnum, ValueHint};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum EvidenceRecordCommand {
    #[command(about = "Generate a supporting-record summary.")]
    Summary(RecordSummaryArgs),
    #[command(about = "Generate a supporting-record artifact locator.")]
    Artifact(RecordArtifactArgs),
    #[command(about = "Generate a supporting-record artifact locator from roots.")]
    ArtifactRoots(RecordArtifactFromRootsArgs),
    #[command(about = "Generate a supporting-record artifact locator from a file.")]
    ArtifactFile(RecordArtifactFromFileArgs),
    #[command(about = "Generate a supporting-record summary from roots.")]
    SummaryRoots(RecordSummaryFromRootsArgs),
    #[command(about = "Generate a supporting-record summary from a file.")]
    SummaryFile(RecordSummaryFromFileArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordSummaryArgs {
    #[command(flatten)]
    pub context: PublicEvidenceRecordContextArgs,
    #[arg(
        long,
        value_name = "HEX",
        help = "Root hash of the supporting-record set."
    )]
    pub record_root: HashArg,
    #[arg(
        long,
        value_name = "N",
        help = "Number of records covered by the root."
    )]
    pub record_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordSummaryFromRootsArgs {
    #[command(flatten)]
    pub context: PublicEvidenceRecordContextArgs,
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
pub struct RecordSummaryFromFileArgs {
    #[command(flatten)]
    pub context: PublicEvidenceRecordContextArgs,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "File containing supporting records to summarize."
    )]
    pub record_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicEvidenceRecordContextArgs {
    #[arg(long, help = "Supporting-record class.")]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX", help = "Public evidence bundle identifier.")]
    pub bundle_id: HashArg,
    #[arg(
        long,
        value_name = "HEX",
        help = "Address signing the evidence manifest."
    )]
    pub manifest_signer: AddressArg,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum PublicEvidenceRecordKindArg {
    BlockHistory,
    FinalityHistory,
    NetworkRuntime,
    DataAvailability,
    InvalidWork,
    RewardSettlement,
}

impl From<PublicEvidenceRecordKindArg> for PublicEvidenceRecordKind {
    fn from(kind: PublicEvidenceRecordKindArg) -> Self {
        match kind {
            PublicEvidenceRecordKindArg::BlockHistory => Self::BlockHistory,
            PublicEvidenceRecordKindArg::FinalityHistory => Self::FinalityHistory,
            PublicEvidenceRecordKindArg::NetworkRuntime => Self::NetworkRuntimeObservations,
            PublicEvidenceRecordKindArg::DataAvailability => Self::DataAvailabilityMeasurements,
            PublicEvidenceRecordKindArg::InvalidWork => Self::InvalidWorkRejections,
            PublicEvidenceRecordKindArg::RewardSettlement => Self::RewardSettlements,
        }
    }
}
