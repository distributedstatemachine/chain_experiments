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
            PublicEvidenceCommand::RunWindow(args) => CliCommand::PublicEvidenceRunWindow {
                bundle_id: args.bundle_id,
                manifest_signer: args.manifest_signer,
                run_started_at_unix_seconds: args.started_at,
                run_ended_at_unix_seconds: args.ended_at,
                observed_blocks: args.observed_blocks,
            },
            PublicEvidenceCommand::RunWindowFromFile(args) => {
                CliCommand::PublicEvidenceRunWindowFromFile {
                    bundle_id: args.bundle_id,
                    manifest_signer: args.manifest_signer,
                    block_observation_file: args.block_observation_file,
                }
            }
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
pub(super) struct RunWindowArgs {
    #[arg(long, value_parser = parse_hash_value)]
    bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    manifest_signer: Address,
    #[arg(long)]
    started_at: u64,
    #[arg(long)]
    ended_at: u64,
    #[arg(long)]
    observed_blocks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct RunWindowFromFileArgs {
    #[arg(long, value_parser = parse_hash_value)]
    bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    manifest_signer: Address,
    #[arg(long)]
    block_observation_file: String,
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
