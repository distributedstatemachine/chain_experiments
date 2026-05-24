use super::CliCommand;
use super::parser_values::{
    HashList, PublicEvidenceRecordKindArg, parse_hash_list_value, parse_hash_value,
};
use crate::types::{Address, Hash};
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct RecordSummaryArgs {
    #[arg(long)]
    kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    manifest_signer: Address,
    #[arg(long, value_parser = parse_hash_value)]
    record_root: Hash,
    #[arg(long)]
    record_count: u64,
}

impl RecordSummaryArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceRecordSummary {
            kind: self.kind.into(),
            bundle_id: self.bundle_id,
            manifest_signer: self.manifest_signer,
            record_root: self.record_root,
            record_count: self.record_count,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct RecordArtifactArgs {
    #[arg(long)]
    kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    manifest_signer: Address,
    #[arg(long)]
    artifact_uri: String,
    #[arg(long, value_parser = parse_hash_value)]
    record_root: Hash,
    #[arg(long)]
    record_count: u64,
}

impl RecordArtifactArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceRecordArtifact {
            kind: self.kind.into(),
            bundle_id: self.bundle_id,
            manifest_signer: self.manifest_signer,
            artifact_uri: self.artifact_uri,
            record_root: self.record_root,
            record_count: self.record_count,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct RecordArtifactFromRootsArgs {
    #[arg(long)]
    kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    manifest_signer: Address,
    #[arg(long)]
    artifact_uri: String,
    #[arg(long, value_parser = parse_hash_list_value)]
    record_roots: HashList,
}

impl RecordArtifactFromRootsArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceRecordArtifactFromRoots {
            kind: self.kind.into(),
            bundle_id: self.bundle_id,
            manifest_signer: self.manifest_signer,
            artifact_uri: self.artifact_uri,
            record_roots: self.record_roots.0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct RecordArtifactFromFileArgs {
    #[arg(long)]
    kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    manifest_signer: Address,
    #[arg(long)]
    artifact_uri: String,
    #[arg(long)]
    record_file: String,
}

impl RecordArtifactFromFileArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceRecordArtifactFromFile {
            kind: self.kind.into(),
            bundle_id: self.bundle_id,
            manifest_signer: self.manifest_signer,
            artifact_uri: self.artifact_uri,
            record_file: self.record_file,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct RecordSummaryFromRootsArgs {
    #[arg(long)]
    kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    manifest_signer: Address,
    #[arg(long, value_parser = parse_hash_list_value)]
    record_roots: HashList,
}

impl RecordSummaryFromRootsArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceRecordSummaryFromRoots {
            kind: self.kind.into(),
            bundle_id: self.bundle_id,
            manifest_signer: self.manifest_signer,
            record_roots: self.record_roots.0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct RecordSummaryFromFileArgs {
    #[arg(long)]
    kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    manifest_signer: Address,
    #[arg(long)]
    record_file: String,
}

impl RecordSummaryFromFileArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceRecordSummaryFromFile {
            kind: self.kind.into(),
            bundle_id: self.bundle_id,
            manifest_signer: self.manifest_signer,
            record_file: self.record_file,
        }
    }
}
