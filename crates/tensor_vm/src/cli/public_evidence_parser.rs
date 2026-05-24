use super::CliCommand;
use super::parser_values::{PublicNodeRoleArg, parse_hash_value};
use super::public_evidence_network_parser::{
    NetworkObservationArgs, NetworkObservationFromServiceLogArgs,
};
use super::public_evidence_publication_parser::{AuditorRecordArgs, PublicationArgs};
use super::public_evidence_record_parser::{
    RecordArtifactArgs, RecordArtifactFromFileArgs, RecordArtifactFromRootsArgs, RecordSummaryArgs,
    RecordSummaryFromFileArgs, RecordSummaryFromRootsArgs,
};
use super::public_evidence_run_window_parser::{RunWindowArgs, RunWindowFromFileArgs};
use super::public_evidence_service_parser::{
    ServiceContentArgs, ServiceContentFromBytesArgs, ServiceContentFromFileArgs, ServiceHealthArgs,
    ServiceHealthFromFileArgs,
};
use crate::types::{Address, Hash};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
pub(super) enum PublicEvidenceCommand {
    Validate(ManifestArgs),
    ServiceHealth(ServiceHealthArgs),
    ServiceHealthFromFile(ServiceHealthFromFileArgs),
    ServiceContent(ServiceContentArgs),
    ServiceContentFromBytes(ServiceContentFromBytesArgs),
    ServiceContentFromFile(ServiceContentFromFileArgs),
    RecordSummary(RecordSummaryArgs),
    RecordArtifact(RecordArtifactArgs),
    RecordArtifactFromRoots(RecordArtifactFromRootsArgs),
    RecordArtifactFromFile(RecordArtifactFromFileArgs),
    RecordSummaryFromRoots(RecordSummaryFromRootsArgs),
    RecordSummaryFromFile(RecordSummaryFromFileArgs),
    NetworkObservation(NetworkObservationArgs),
    NetworkObservationFromServiceLog(NetworkObservationFromServiceLogArgs),
    Publication(PublicationArgs),
    AuditorRecord(AuditorRecordArgs),
    RunWindow(RunWindowArgs),
    RunWindowFromFile(RunWindowFromFileArgs),
    NodeHeartbeat(NodeHeartbeatArgs),
    NodeHeartbeatFromFile(NodeHeartbeatFromFileArgs),
    OperatorAttestation(OperatorAttestationArgs),
}

impl PublicEvidenceCommand {
    pub(super) fn into_command(self) -> CliCommand {
        match self {
            PublicEvidenceCommand::Validate(args) => CliCommand::PublicEvidenceValidate {
                manifest: args.manifest,
            },
            PublicEvidenceCommand::ServiceHealth(args) => args.into_command(),
            PublicEvidenceCommand::ServiceHealthFromFile(args) => args.into_command(),
            PublicEvidenceCommand::ServiceContent(args) => args.into_command(),
            PublicEvidenceCommand::ServiceContentFromBytes(args) => args.into_command(),
            PublicEvidenceCommand::ServiceContentFromFile(args) => args.into_command(),
            PublicEvidenceCommand::RecordSummary(args) => args.into_command(),
            PublicEvidenceCommand::RecordArtifact(args) => args.into_command(),
            PublicEvidenceCommand::RecordArtifactFromRoots(args) => args.into_command(),
            PublicEvidenceCommand::RecordArtifactFromFile(args) => args.into_command(),
            PublicEvidenceCommand::RecordSummaryFromRoots(args) => args.into_command(),
            PublicEvidenceCommand::RecordSummaryFromFile(args) => args.into_command(),
            PublicEvidenceCommand::NetworkObservation(args) => args.into_command(),
            PublicEvidenceCommand::NetworkObservationFromServiceLog(args) => args.into_command(),
            PublicEvidenceCommand::Publication(args) => args.into_command(),
            PublicEvidenceCommand::AuditorRecord(args) => args.into_command(),
            PublicEvidenceCommand::RunWindow(args) => args.into_command(),
            PublicEvidenceCommand::RunWindowFromFile(args) => args.into_command(),
            PublicEvidenceCommand::NodeHeartbeat(args) => CliCommand::PublicEvidenceNodeHeartbeat {
                role: args.role.into(),
                address: args.address,
                operator_id: args.operator_id,
                first_seen_block: args.first_block,
                last_seen_block: args.last_block,
                signed_heartbeat_count: args.heartbeat_count,
            },
            PublicEvidenceCommand::NodeHeartbeatFromFile(args) => {
                CliCommand::PublicEvidenceNodeHeartbeatFromFile {
                    role: args.role.into(),
                    address: args.address,
                    operator_id: args.operator_id,
                    heartbeat_file: args.heartbeat_file,
                }
            }
            PublicEvidenceCommand::OperatorAttestation(args) => {
                CliCommand::PublicEvidenceOperatorAttestation {
                    role: args.role.into(),
                    address: args.address,
                    operator_id: args.operator_id,
                    identity_uri: args.identity_uri,
                    observed_at_unix_seconds: args.observed_at,
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ManifestArgs {
    #[arg(long)]
    manifest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct NodeHeartbeatArgs {
    #[arg(long)]
    role: PublicNodeRoleArg,
    #[arg(long, value_parser = parse_hash_value)]
    address: Address,
    #[arg(long, value_parser = parse_hash_value)]
    operator_id: Hash,
    #[arg(long)]
    first_block: u64,
    #[arg(long)]
    last_block: u64,
    #[arg(long)]
    heartbeat_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct NodeHeartbeatFromFileArgs {
    #[arg(long)]
    role: PublicNodeRoleArg,
    #[arg(long, value_parser = parse_hash_value)]
    address: Address,
    #[arg(long, value_parser = parse_hash_value)]
    operator_id: Hash,
    #[arg(long)]
    heartbeat_file: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct OperatorAttestationArgs {
    #[arg(long)]
    role: PublicNodeRoleArg,
    #[arg(long, value_parser = parse_hash_value)]
    address: Address,
    #[arg(long, value_parser = parse_hash_value)]
    operator_id: Hash,
    #[arg(long)]
    identity_uri: String,
    #[arg(long)]
    observed_at: u64,
}
