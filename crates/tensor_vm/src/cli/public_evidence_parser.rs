use super::CliCommand;
use super::parser_values::{
    HashList, PublicEvidenceRecordKindArg, PublicNodeRoleArg, PublicServiceKindArg,
    parse_hash_list_value, parse_hash_value,
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
            PublicEvidenceCommand::ServiceHealth(args) => CliCommand::PublicEvidenceServiceHealth {
                kind: args.kind.into(),
                endpoint_id: args.endpoint_id,
                public_url: args.public_url,
                health_path: args.health_path,
                first_seen_block: args.first_block,
                last_seen_block: args.last_block,
                reachable_observation_count: args.reachable_count,
                signed_health_check_count: args.signed_health_check_count,
            },
            PublicEvidenceCommand::ServiceHealthFromFile(args) => {
                CliCommand::PublicEvidenceServiceHealthFromFile {
                    kind: args.kind.into(),
                    endpoint_id: args.endpoint_id,
                    public_url: args.public_url,
                    health_path: args.health_path,
                    observation_file: args.observation_file,
                }
            }
            PublicEvidenceCommand::ServiceContent(args) => {
                CliCommand::PublicEvidenceServiceContent {
                    kind: args.kind.into(),
                    endpoint_id: args.endpoint_id,
                    public_url: args.public_url,
                    content_path: args.content_path,
                    content_root: args.content_root,
                    observed_at_unix_seconds: args.observed_at,
                    min_content_bytes: args.min_content_bytes,
                }
            }
            PublicEvidenceCommand::ServiceContentFromBytes(args) => {
                CliCommand::PublicEvidenceServiceContentFromBytes {
                    kind: args.kind.into(),
                    endpoint_id: args.endpoint_id,
                    public_url: args.public_url,
                    content_path: args.content_path,
                    observed_at_unix_seconds: args.observed_at,
                    content_hex: args.content_hex,
                }
            }
            PublicEvidenceCommand::ServiceContentFromFile(args) => {
                CliCommand::PublicEvidenceServiceContentFromFile {
                    kind: args.kind.into(),
                    endpoint_id: args.endpoint_id,
                    public_url: args.public_url,
                    content_path: args.content_path,
                    observed_at_unix_seconds: args.observed_at,
                    content_file: args.content_file,
                }
            }
            PublicEvidenceCommand::RecordSummary(args) => CliCommand::PublicEvidenceRecordSummary {
                kind: args.kind.into(),
                bundle_id: args.bundle_id,
                manifest_signer: args.manifest_signer,
                record_root: args.record_root,
                record_count: args.record_count,
            },
            PublicEvidenceCommand::RecordArtifact(args) => {
                CliCommand::PublicEvidenceRecordArtifact {
                    kind: args.kind.into(),
                    bundle_id: args.bundle_id,
                    manifest_signer: args.manifest_signer,
                    artifact_uri: args.artifact_uri,
                    record_root: args.record_root,
                    record_count: args.record_count,
                }
            }
            PublicEvidenceCommand::RecordArtifactFromRoots(args) => {
                CliCommand::PublicEvidenceRecordArtifactFromRoots {
                    kind: args.kind.into(),
                    bundle_id: args.bundle_id,
                    manifest_signer: args.manifest_signer,
                    artifact_uri: args.artifact_uri,
                    record_roots: args.record_roots.0,
                }
            }
            PublicEvidenceCommand::RecordArtifactFromFile(args) => {
                CliCommand::PublicEvidenceRecordArtifactFromFile {
                    kind: args.kind.into(),
                    bundle_id: args.bundle_id,
                    manifest_signer: args.manifest_signer,
                    artifact_uri: args.artifact_uri,
                    record_file: args.record_file,
                }
            }
            PublicEvidenceCommand::RecordSummaryFromRoots(args) => {
                CliCommand::PublicEvidenceRecordSummaryFromRoots {
                    kind: args.kind.into(),
                    bundle_id: args.bundle_id,
                    manifest_signer: args.manifest_signer,
                    record_roots: args.record_roots.0,
                }
            }
            PublicEvidenceCommand::RecordSummaryFromFile(args) => {
                CliCommand::PublicEvidenceRecordSummaryFromFile {
                    kind: args.kind.into(),
                    bundle_id: args.bundle_id,
                    manifest_signer: args.manifest_signer,
                    record_file: args.record_file,
                }
            }
            PublicEvidenceCommand::NetworkObservation(args) => {
                CliCommand::PublicEvidenceNetworkObservation {
                    operator_id: args.operator_id,
                    peer_id: args.peer_id,
                    listen_address: args.listen_address,
                    observed_at_unix_seconds: args.observed_at,
                    gossip_topic_count: args.gossip_topics,
                    request_response_protocol_count: args.request_response_protocols,
                    bootstrap_peer_count: args.bootstrap_peers,
                    max_transmit_bytes: args.max_transmit_bytes,
                    request_timeout_seconds: args.request_timeout_seconds,
                    max_concurrent_streams: args.max_concurrent_streams,
                    idle_connection_timeout_seconds: args.idle_timeout_seconds,
                }
            }
            PublicEvidenceCommand::NetworkObservationFromServiceLog(args) => {
                CliCommand::PublicEvidenceNetworkObservationFromServiceLog {
                    operator_id: args.operator_id,
                    listen_address: args.listen_address,
                    observed_at_unix_seconds: args.observed_at,
                    service_log: args.service_log,
                }
            }
            PublicEvidenceCommand::Publication(args) => CliCommand::PublicEvidencePublication {
                bundle_id: args.bundle_id,
                public_uri: args.public_uri,
                manifest_signer: args.manifest_signer,
                manifest_signature_count: args.manifest_signature_count,
                independent_auditor_count: args.independent_auditor_count,
            },
            PublicEvidenceCommand::AuditorRecord(args) => CliCommand::PublicEvidenceAuditorRecord {
                bundle_id: args.bundle_id,
                public_uri: args.public_uri,
                auditor_id: args.auditor_id,
                audit_uri: args.audit_uri,
                observed_at_unix_seconds: args.observed_at,
            },
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
pub(super) struct ServiceHealthArgs {
    #[arg(long)]
    kind: PublicServiceKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    endpoint_id: Hash,
    #[arg(long)]
    public_url: String,
    #[arg(long)]
    health_path: String,
    #[arg(long)]
    first_block: u64,
    #[arg(long)]
    last_block: u64,
    #[arg(long)]
    reachable_count: u64,
    #[arg(long)]
    signed_health_check_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ServiceHealthFromFileArgs {
    #[arg(long)]
    kind: PublicServiceKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    endpoint_id: Hash,
    #[arg(long)]
    public_url: String,
    #[arg(long)]
    health_path: String,
    #[arg(long)]
    observation_file: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ServiceContentArgs {
    #[arg(long)]
    kind: PublicServiceKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    endpoint_id: Hash,
    #[arg(long)]
    public_url: String,
    #[arg(long)]
    content_path: String,
    #[arg(long, value_parser = parse_hash_value)]
    content_root: Hash,
    #[arg(long)]
    observed_at: u64,
    #[arg(long)]
    min_content_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ServiceContentFromBytesArgs {
    #[arg(long)]
    kind: PublicServiceKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    endpoint_id: Hash,
    #[arg(long)]
    public_url: String,
    #[arg(long)]
    content_path: String,
    #[arg(long)]
    observed_at: u64,
    #[arg(long)]
    content_hex: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ServiceContentFromFileArgs {
    #[arg(long)]
    kind: PublicServiceKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    endpoint_id: Hash,
    #[arg(long)]
    public_url: String,
    #[arg(long)]
    content_path: String,
    #[arg(long)]
    observed_at: u64,
    #[arg(long)]
    content_file: String,
}

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

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct NetworkObservationArgs {
    #[arg(long, value_parser = parse_hash_value)]
    operator_id: Hash,
    #[arg(long)]
    peer_id: String,
    #[arg(long)]
    listen_address: String,
    #[arg(long)]
    observed_at: u64,
    #[arg(long)]
    gossip_topics: u64,
    #[arg(long)]
    request_response_protocols: u64,
    #[arg(long)]
    bootstrap_peers: u64,
    #[arg(long)]
    max_transmit_bytes: u64,
    #[arg(long)]
    request_timeout_seconds: u64,
    #[arg(long)]
    max_concurrent_streams: u64,
    #[arg(long)]
    idle_timeout_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct NetworkObservationFromServiceLogArgs {
    #[arg(long, value_parser = parse_hash_value)]
    operator_id: Hash,
    #[arg(long)]
    listen_address: String,
    #[arg(long)]
    observed_at: u64,
    #[arg(long)]
    service_log: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct PublicationArgs {
    #[arg(long, value_parser = parse_hash_value)]
    bundle_id: Hash,
    #[arg(long)]
    public_uri: String,
    #[arg(long, value_parser = parse_hash_value)]
    manifest_signer: Address,
    #[arg(long)]
    manifest_signature_count: u64,
    #[arg(long)]
    independent_auditor_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct AuditorRecordArgs {
    #[arg(long, value_parser = parse_hash_value)]
    bundle_id: Hash,
    #[arg(long)]
    public_uri: String,
    #[arg(long, value_parser = parse_hash_value)]
    auditor_id: Hash,
    #[arg(long)]
    audit_uri: String,
    #[arg(long)]
    observed_at: u64,
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
