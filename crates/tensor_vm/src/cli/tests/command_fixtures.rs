use super::*;
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use crate::types::{Address, Hash};
use clap::Parser;
use libp2p::PeerId;
use std::path::PathBuf;

pub(super) fn parse_test_cli(
    args: &[&str],
) -> std::result::Result<super::TvmdCommand, clap::Error> {
    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push("tvmd");
    argv.extend_from_slice(args);
    TvmdCli::try_parse_from(argv).map(|cli| cli.command)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum CommandFixture {
    EvidenceValidate {
        manifest: String,
    },
    EvidenceServiceHealth {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        health_path: String,
        first_seen_block: u64,
        last_seen_block: u64,
        reachable_observation_count: u64,
        signed_health_check_count: u64,
    },
    EvidenceServiceHealthFromFile {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        health_path: String,
        observation_file: String,
    },
    EvidenceServiceContent {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        content_path: String,
        content_root: Hash,
        observed_at_unix_seconds: u64,
        min_content_bytes: u64,
    },
    EvidenceServiceContentFromBytes {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        content_path: String,
        observed_at_unix_seconds: u64,
        content_bytes: Vec<u8>,
    },
    EvidenceServiceContentFromFile {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        content_path: String,
        observed_at_unix_seconds: u64,
        content_file: String,
    },
    EvidenceRecordSummary {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        record_root: Hash,
        record_count: u64,
    },
    EvidenceRecordArtifact {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        artifact_uri: String,
        record_root: Hash,
        record_count: u64,
    },
    EvidenceRecordArtifactFromRoots {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        artifact_uri: String,
        record_roots: Vec<Hash>,
    },
    EvidenceRecordArtifactFromFile {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        artifact_uri: String,
        record_file: String,
    },
    EvidenceRecordSummaryFromRoots {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        record_roots: Vec<Hash>,
    },
    EvidenceRecordSummaryFromFile {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        record_file: String,
    },
    EvidenceNetworkObservation {
        operator_id: Hash,
        peer_id: String,
        listen_address: String,
        observed_at_unix_seconds: u64,
        gossip_topic_count: u64,
        request_response_protocol_count: u64,
        bootstrap_peer_count: u64,
        max_transmit_bytes: u64,
        request_timeout_seconds: u64,
        max_concurrent_streams: u64,
        idle_connection_timeout_seconds: u64,
    },
    EvidenceNetworkObservationFromServiceLog {
        operator_id: Hash,
        listen_address: String,
        observed_at_unix_seconds: u64,
        service_log: String,
    },
    EvidencePublication {
        bundle_id: Hash,
        public_uri: String,
        manifest_signer: Address,
        manifest_signature_count: u64,
        independent_auditor_count: u64,
    },
    EvidenceAuditorRecord {
        bundle_id: Hash,
        public_uri: String,
        auditor_id: Address,
        audit_uri: String,
        observed_at_unix_seconds: u64,
    },
    EvidenceRunWindow {
        bundle_id: Hash,
        manifest_signer: Address,
        run_started_at_unix_seconds: u64,
        run_ended_at_unix_seconds: u64,
        observed_blocks: u64,
    },
    EvidenceRunWindowFromFile {
        bundle_id: Hash,
        manifest_signer: Address,
        block_observation_file: String,
    },
    EvidenceNodeHeartbeat {
        role: PublicNodeRole,
        address: Address,
        operator_id: Hash,
        first_seen_block: u64,
        last_seen_block: u64,
        signed_heartbeat_count: u64,
    },
    EvidenceNodeHeartbeatFromFile {
        role: PublicNodeRole,
        address: Address,
        operator_id: Hash,
        heartbeat_file: String,
    },
    EvidenceOperatorAttestation {
        role: PublicNodeRole,
        address: Address,
        operator_id: Hash,
        identity_uri: String,
        observed_at_unix_seconds: u64,
    },
    TestnetPreflight {
        manifest: String,
    },
}

impl PartialEq<CommandFixture> for super::TvmdCommand {
    fn eq(&self, other: &CommandFixture) -> bool {
        self == &other.clone().into_cli_command()
    }
}

pub(super) fn execute_command_fixture(command: &CommandFixture) -> crate::error::Result<String> {
    let cli_command = command.clone().into_cli_command();
    execute_test_cli_command(&cli_command)
}

pub(super) fn execute_test_cli_command(
    cli_command: &super::TvmdCommand,
) -> crate::error::Result<String> {
    match cli_command {
        super::TvmdCommand::Miner(_)
        | super::TvmdCommand::Validator(_)
        | super::TvmdCommand::Proposer(_)
        | super::TvmdCommand::Node(_)
        | super::TvmdCommand::Localnet(_) => {
            super::local_execution::execute_local_cli_command(cli_command)
        }
        super::TvmdCommand::Public(super::PublicCommand::Preflight(_))
        | super::TvmdCommand::Public(super::PublicCommand::Evidence(
            super::EvidenceCommand::Validate(_),
        )) => Err(crate::error::TvmError::InvalidReceipt(
            "public artifact validation reads manifests through the app dispatcher",
        )),
        super::TvmdCommand::Public(super::PublicCommand::Evidence(command)) => {
            super::execute_public_evidence_command(command)
        }
    }
}

pub(super) fn path_arg(value: String) -> PathBuf {
    value.into()
}

pub(super) fn multiaddr_arg(value: String) -> libp2p::Multiaddr {
    value.parse().expect("fixture multiaddr must parse")
}

pub(super) fn peer_id_arg(value: String) -> PeerId {
    value.parse().expect("fixture peer ID must parse")
}

pub(super) fn hash_arg(value: Hash) -> HashArg {
    HashArg::new(value)
}

pub(super) fn address_arg(value: Address) -> AddressArg {
    AddressArg::new(value)
}

pub(super) fn hash_args(values: Vec<Hash>) -> Vec<HashArg> {
    values.into_iter().map(HashArg::new).collect()
}

fn public_evidence_command(command: EvidenceCommand) -> super::TvmdCommand {
    super::TvmdCommand::Public(PublicCommand::Evidence(command))
}

impl CommandFixture {
    fn into_cli_command(self) -> super::TvmdCommand {
        match self {
            Self::EvidenceValidate { manifest } => super::TvmdCommand::Public(
                PublicCommand::Evidence(EvidenceCommand::Validate(PublicEvidenceManifestArgs {
                    manifest: path_arg(manifest),
                })),
            ),
            Self::EvidenceServiceHealth {
                kind,
                endpoint_id,
                public_url,
                health_path,
                first_seen_block,
                last_seen_block,
                reachable_observation_count,
                signed_health_check_count,
            } => public_evidence_command(EvidenceCommand::Service(EvidenceServiceCommand::Health(
                ServiceHealthArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id: hash_arg(endpoint_id),
                    public_url,
                    health_path,
                    first_block: first_seen_block,
                    last_block: last_seen_block,
                    reachable_count: reachable_observation_count,
                    signed_health_check_count,
                },
            ))),
            Self::EvidenceServiceHealthFromFile {
                kind,
                endpoint_id,
                public_url,
                health_path,
                observation_file,
            } => public_evidence_command(EvidenceCommand::Service(
                EvidenceServiceCommand::HealthFile(ServiceHealthFromFileArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id: hash_arg(endpoint_id),
                    public_url,
                    health_path,
                    observation_file: path_arg(observation_file),
                }),
            )),
            Self::EvidenceServiceContent {
                kind,
                endpoint_id,
                public_url,
                content_path,
                content_root,
                observed_at_unix_seconds,
                min_content_bytes,
            } => public_evidence_command(EvidenceCommand::Service(
                EvidenceServiceCommand::Content(ServiceContentArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id: hash_arg(endpoint_id),
                    public_url,
                    content_path,
                    content_root: hash_arg(content_root),
                    observed_at: observed_at_unix_seconds,
                    min_content_bytes,
                }),
            )),
            Self::EvidenceServiceContentFromBytes {
                kind,
                endpoint_id,
                public_url,
                content_path,
                observed_at_unix_seconds,
                content_bytes,
            } => public_evidence_command(EvidenceCommand::Service(
                EvidenceServiceCommand::ContentBytes(ServiceContentFromBytesArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id: hash_arg(endpoint_id),
                    public_url,
                    content_path,
                    observed_at: observed_at_unix_seconds,
                    content: HexBytesArg::new(content_bytes),
                }),
            )),
            Self::EvidenceServiceContentFromFile {
                kind,
                endpoint_id,
                public_url,
                content_path,
                observed_at_unix_seconds,
                content_file,
            } => public_evidence_command(EvidenceCommand::Service(
                EvidenceServiceCommand::ContentFile(ServiceContentFromFileArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id: hash_arg(endpoint_id),
                    public_url,
                    content_path,
                    observed_at: observed_at_unix_seconds,
                    content_file: path_arg(content_file),
                }),
            )),
            Self::EvidenceRecordSummary {
                kind,
                bundle_id,
                manifest_signer,
                record_root,
                record_count,
            } => public_evidence_command(EvidenceCommand::Record(EvidenceRecordCommand::Summary(
                RecordSummaryArgs {
                    kind: record_kind_arg(kind),
                    bundle_id: hash_arg(bundle_id),
                    manifest_signer: address_arg(manifest_signer),
                    record_root: hash_arg(record_root),
                    record_count,
                },
            ))),
            Self::EvidenceRecordArtifact {
                kind,
                bundle_id,
                manifest_signer,
                artifact_uri,
                record_root,
                record_count,
            } => public_evidence_command(EvidenceCommand::Record(EvidenceRecordCommand::Artifact(
                RecordArtifactArgs {
                    kind: record_kind_arg(kind),
                    bundle_id: hash_arg(bundle_id),
                    manifest_signer: address_arg(manifest_signer),
                    artifact_uri,
                    record_root: hash_arg(record_root),
                    record_count,
                },
            ))),
            Self::EvidenceRecordArtifactFromRoots {
                kind,
                bundle_id,
                manifest_signer,
                artifact_uri,
                record_roots,
            } => public_evidence_command(EvidenceCommand::Record(
                EvidenceRecordCommand::ArtifactRoots(RecordArtifactFromRootsArgs {
                    kind: record_kind_arg(kind),
                    bundle_id: hash_arg(bundle_id),
                    manifest_signer: address_arg(manifest_signer),
                    artifact_uri,
                    record_roots: hash_args(record_roots),
                }),
            )),
            Self::EvidenceRecordArtifactFromFile {
                kind,
                bundle_id,
                manifest_signer,
                artifact_uri,
                record_file,
            } => public_evidence_command(EvidenceCommand::Record(
                EvidenceRecordCommand::ArtifactFile(RecordArtifactFromFileArgs {
                    kind: record_kind_arg(kind),
                    bundle_id: hash_arg(bundle_id),
                    manifest_signer: address_arg(manifest_signer),
                    artifact_uri,
                    record_file: path_arg(record_file),
                }),
            )),
            Self::EvidenceRecordSummaryFromRoots {
                kind,
                bundle_id,
                manifest_signer,
                record_roots,
            } => public_evidence_command(EvidenceCommand::Record(
                EvidenceRecordCommand::SummaryRoots(RecordSummaryFromRootsArgs {
                    kind: record_kind_arg(kind),
                    bundle_id: hash_arg(bundle_id),
                    manifest_signer: address_arg(manifest_signer),
                    record_roots: hash_args(record_roots),
                }),
            )),
            Self::EvidenceRecordSummaryFromFile {
                kind,
                bundle_id,
                manifest_signer,
                record_file,
            } => public_evidence_command(EvidenceCommand::Record(
                EvidenceRecordCommand::SummaryFile(RecordSummaryFromFileArgs {
                    kind: record_kind_arg(kind),
                    bundle_id: hash_arg(bundle_id),
                    manifest_signer: address_arg(manifest_signer),
                    record_file: path_arg(record_file),
                }),
            )),
            Self::EvidenceNetworkObservation {
                operator_id,
                peer_id,
                listen_address,
                observed_at_unix_seconds,
                gossip_topic_count,
                request_response_protocol_count,
                bootstrap_peer_count,
                max_transmit_bytes,
                request_timeout_seconds,
                max_concurrent_streams,
                idle_connection_timeout_seconds,
            } => public_evidence_command(EvidenceCommand::Network(
                EvidenceNetworkCommand::Observation(NetworkObservationArgs {
                    operator_id: hash_arg(operator_id),
                    peer_id: peer_id_arg(peer_id),
                    listen_address: multiaddr_arg(listen_address),
                    observed_at: observed_at_unix_seconds,
                    gossip_topics: gossip_topic_count,
                    request_response_protocols: request_response_protocol_count,
                    bootstrap_peers: bootstrap_peer_count,
                    max_transmit_bytes,
                    request_timeout_seconds,
                    max_concurrent_streams,
                    idle_timeout_seconds: idle_connection_timeout_seconds,
                }),
            )),
            Self::EvidenceNetworkObservationFromServiceLog {
                operator_id,
                listen_address,
                observed_at_unix_seconds,
                service_log,
            } => public_evidence_command(EvidenceCommand::Network(
                EvidenceNetworkCommand::FromServiceLog(NetworkObservationFromServiceLogArgs {
                    operator_id: hash_arg(operator_id),
                    listen_address: multiaddr_arg(listen_address),
                    observed_at: observed_at_unix_seconds,
                    service_log: path_arg(service_log),
                }),
            )),
            Self::EvidencePublication {
                bundle_id,
                public_uri,
                manifest_signer,
                manifest_signature_count,
                independent_auditor_count,
            } => public_evidence_command(EvidenceCommand::Publish(PublicationArgs {
                bundle_id: hash_arg(bundle_id),
                public_uri,
                manifest_signer: address_arg(manifest_signer),
                manifest_signature_count,
                independent_auditor_count,
            })),
            Self::EvidenceAuditorRecord {
                bundle_id,
                public_uri,
                auditor_id,
                audit_uri,
                observed_at_unix_seconds,
            } => public_evidence_command(EvidenceCommand::Audit(AuditorRecordArgs {
                bundle_id: hash_arg(bundle_id),
                public_uri,
                auditor_id: address_arg(auditor_id),
                audit_uri,
                observed_at: observed_at_unix_seconds,
            })),
            Self::EvidenceRunWindow {
                bundle_id,
                manifest_signer,
                run_started_at_unix_seconds,
                run_ended_at_unix_seconds,
                observed_blocks,
            } => public_evidence_command(EvidenceCommand::Run(EvidenceRunCommand::Window(
                RunWindowArgs {
                    bundle_id: hash_arg(bundle_id),
                    manifest_signer: address_arg(manifest_signer),
                    started_at: run_started_at_unix_seconds,
                    ended_at: run_ended_at_unix_seconds,
                    observed_blocks,
                },
            ))),
            Self::EvidenceRunWindowFromFile {
                bundle_id,
                manifest_signer,
                block_observation_file,
            } => public_evidence_command(EvidenceCommand::Run(EvidenceRunCommand::WindowFile(
                RunWindowFromFileArgs {
                    bundle_id: hash_arg(bundle_id),
                    manifest_signer: address_arg(manifest_signer),
                    block_observation_file: path_arg(block_observation_file),
                },
            ))),
            Self::EvidenceNodeHeartbeat {
                role,
                address,
                operator_id,
                first_seen_block,
                last_seen_block,
                signed_heartbeat_count,
            } => public_evidence_command(EvidenceCommand::Node(EvidenceNodeCommand::Heartbeat(
                NodeHeartbeatArgs {
                    role: node_role_arg(role),
                    address: address_arg(address),
                    operator_id: hash_arg(operator_id),
                    first_block: first_seen_block,
                    last_block: last_seen_block,
                    heartbeat_count: signed_heartbeat_count,
                },
            ))),
            Self::EvidenceNodeHeartbeatFromFile {
                role,
                address,
                operator_id,
                heartbeat_file,
            } => public_evidence_command(EvidenceCommand::Node(
                EvidenceNodeCommand::HeartbeatFile(NodeHeartbeatFromFileArgs {
                    role: node_role_arg(role),
                    address: address_arg(address),
                    operator_id: hash_arg(operator_id),
                    heartbeat_file: path_arg(heartbeat_file),
                }),
            )),
            Self::EvidenceOperatorAttestation {
                role,
                address,
                operator_id,
                identity_uri,
                observed_at_unix_seconds,
            } => public_evidence_command(EvidenceCommand::Node(
                EvidenceNodeCommand::OperatorAttestation(OperatorAttestationArgs {
                    role: node_role_arg(role),
                    address: address_arg(address),
                    operator_id: hash_arg(operator_id),
                    identity_uri,
                    observed_at: observed_at_unix_seconds,
                }),
            )),
            Self::TestnetPreflight { manifest } => {
                super::TvmdCommand::Public(PublicCommand::Preflight(PublicTestnetManifestArgs {
                    manifest: path_arg(manifest),
                }))
            }
        }
    }
}

pub(super) fn service_kind_arg(kind: PublicServiceKind) -> PublicServiceKindArg {
    match kind {
        PublicServiceKind::Rpc => PublicServiceKindArg::Rpc,
        PublicServiceKind::Explorer => PublicServiceKindArg::Explorer,
        PublicServiceKind::Faucet => PublicServiceKindArg::Faucet,
        PublicServiceKind::Telemetry => PublicServiceKindArg::Telemetry,
    }
}

pub(super) fn node_role_arg(role: PublicNodeRole) -> PublicNodeRoleArg {
    match role {
        PublicNodeRole::Miner => PublicNodeRoleArg::Miner,
        PublicNodeRole::Validator => PublicNodeRoleArg::Validator,
    }
}

pub(super) fn record_kind_arg(kind: PublicEvidenceRecordKind) -> PublicEvidenceRecordKindArg {
    match kind {
        PublicEvidenceRecordKind::BlockHistory => PublicEvidenceRecordKindArg::BlockHistory,
        PublicEvidenceRecordKind::FinalityHistory => PublicEvidenceRecordKindArg::FinalityHistory,
        PublicEvidenceRecordKind::NetworkRuntimeObservations => {
            PublicEvidenceRecordKindArg::NetworkRuntime
        }
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => {
            PublicEvidenceRecordKindArg::DataAvailability
        }
        PublicEvidenceRecordKind::InvalidWorkRejections => PublicEvidenceRecordKindArg::InvalidWork,
        PublicEvidenceRecordKind::RewardSettlements => {
            PublicEvidenceRecordKindArg::RewardSettlement
        }
    }
}
