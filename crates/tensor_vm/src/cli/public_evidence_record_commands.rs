use super::public_evidence_bundle_commands::EvidenceBundleIdArgs;
use super::public_evidence_record_artifact_commands::{
    RecordArtifactArgs, RecordArtifactFromFileArgs, RecordArtifactFromRootsArgs,
};
use super::public_evidence_signing_commands::ManifestSignerArgs;
use super::value_types::HashArg;
use crate::testnet::PublicEvidenceRecordKind;
use crate::types::{Address, Hash};
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
    context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    root: RecordRootArgs,
}

impl RecordSummaryArgs {
    #[cfg(test)]
    pub(crate) fn new(context: PublicEvidenceRecordContextArgs, root: RecordRootArgs) -> Self {
        Self { context, root }
    }

    pub fn kind(&self) -> PublicEvidenceRecordKind {
        self.context.kind()
    }

    pub fn bundle_id(&self) -> Hash {
        self.context.bundle_id()
    }

    pub fn manifest_signer(&self) -> Address {
        self.context.manifest_signer()
    }

    pub fn root(&self) -> Hash {
        self.root.root()
    }

    pub fn count(&self) -> u64 {
        self.root.count()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordSummaryFromRootsArgs {
    #[command(flatten)]
    context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    roots: RecordRootsArgs,
}

impl RecordSummaryFromRootsArgs {
    #[cfg(test)]
    pub(crate) fn new(context: PublicEvidenceRecordContextArgs, roots: RecordRootsArgs) -> Self {
        Self { context, roots }
    }

    pub fn kind(&self) -> PublicEvidenceRecordKind {
        self.context.kind()
    }

    pub fn bundle_id(&self) -> Hash {
        self.context.bundle_id()
    }

    pub fn manifest_signer(&self) -> Address {
        self.context.manifest_signer()
    }

    pub fn roots(&self) -> Vec<Hash> {
        self.roots.roots()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordSummaryFromFileArgs {
    #[command(flatten)]
    context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    file: RecordFileArgs,
}

impl RecordSummaryFromFileArgs {
    #[cfg(test)]
    pub(crate) fn new(context: PublicEvidenceRecordContextArgs, file: RecordFileArgs) -> Self {
        Self { context, file }
    }

    pub fn kind(&self) -> PublicEvidenceRecordKind {
        self.context.kind()
    }

    pub fn bundle_id(&self) -> Hash {
        self.context.bundle_id()
    }

    pub fn manifest_signer(&self) -> Address {
        self.context.manifest_signer()
    }

    pub fn file_path(&self) -> &std::path::Path {
        self.file.path()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicEvidenceRecordContextArgs {
    #[arg(long, help = "Supporting-record class.")]
    kind: PublicEvidenceRecordKindArg,
    #[command(flatten)]
    bundle: EvidenceBundleIdArgs,
    #[command(flatten)]
    signer: ManifestSignerArgs,
}

impl PublicEvidenceRecordContextArgs {
    #[cfg(test)]
    pub(crate) fn new(
        kind: PublicEvidenceRecordKindArg,
        bundle: EvidenceBundleIdArgs,
        signer: ManifestSignerArgs,
    ) -> Self {
        Self {
            kind,
            bundle,
            signer,
        }
    }

    pub fn kind(&self) -> PublicEvidenceRecordKind {
        self.kind.into()
    }

    pub fn bundle_id(&self) -> Hash {
        self.bundle.id()
    }

    pub fn manifest_signer(&self) -> Address {
        self.signer.signer()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordRootArgs {
    #[arg(
        long,
        value_name = "HEX",
        help = "Root hash of the supporting-record set."
    )]
    record_root: HashArg,
    #[arg(
        long,
        value_name = "N",
        help = "Number of records covered by the root."
    )]
    record_count: u64,
}

impl RecordRootArgs {
    #[cfg(test)]
    pub(crate) fn new(record_root: Hash, record_count: u64) -> Self {
        Self {
            record_root: HashArg::new(record_root),
            record_count,
        }
    }

    pub fn root(&self) -> Hash {
        self.record_root.into_hash()
    }

    pub fn count(&self) -> u64 {
        self.record_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordRootsArgs {
    #[arg(
        long,
        value_name = "HEX[,HEX...]",
        value_delimiter = ',',
        num_args = 1..,
        help = "Comma-delimited record roots to aggregate."
    )]
    record_roots: Vec<HashArg>,
}

impl RecordRootsArgs {
    #[cfg(test)]
    pub(crate) fn new(record_roots: Vec<Hash>) -> Self {
        Self {
            record_roots: record_roots.into_iter().map(HashArg::new).collect(),
        }
    }

    pub fn roots(&self) -> Vec<Hash> {
        self.record_roots
            .iter()
            .copied()
            .map(HashArg::into_hash)
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordFileArgs {
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "File containing supporting records to summarize."
    )]
    record_file: PathBuf,
}

impl RecordFileArgs {
    #[cfg(test)]
    pub(crate) fn new(record_file: PathBuf) -> Self {
        Self { record_file }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.record_file
    }
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
