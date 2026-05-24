use super::*;
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use crate::types::{Address, Hash};
use clap::Parser;
use libp2p::PeerId;
use std::net::SocketAddr;
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
    MinerRegister {
        stake: u64,
    },
    MinerStart {
        wallet: String,
        device: String,
        node: String,
    },
    MinerRun {
        wallet: String,
        device: String,
        node: String,
        listen: String,
        p2p_listen: String,
        data_dir: String,
        identity_seed: Option<Hash>,
        auth_token: String,
        max_requests: usize,
    },
    MinerStatus,
    ValidatorRegister {
        stake: u64,
    },
    ValidatorStart {
        wallet: String,
        node: String,
    },
    ValidatorRun {
        wallet: String,
        node: String,
        listen: String,
        p2p_listen: String,
        data_dir: String,
        identity_seed: Option<Hash>,
        auth_token: String,
        max_requests: usize,
    },
    ValidatorStatus,
    ProposerRun {
        wallet: String,
        node: String,
        listen: String,
        p2p_listen: String,
        data_dir: String,
        identity_seed: Option<Hash>,
        auth_token: String,
        max_requests: usize,
    },
    ServiceInit {
        data_dir: String,
    },
    ServicePeerAdd {
        data_dir: String,
        peer_id: String,
        address: String,
    },
    ServiceReadiness {
        p2p_listen: String,
        data_dir: String,
        identity_seed: Option<Hash>,
    },
    ServiceServe {
        listen: String,
        p2p_listen: String,
        data_dir: String,
        identity_seed: Option<Hash>,
        auth_token: String,
        max_requests: usize,
    },
    ServiceStatus {
        data_dir: String,
    },
    ServiceBlock {
        data_dir: String,
        height: u64,
    },
    LocalTestnetSeed {
        data_dir: String,
    },
    LocalCpuVerify {
        data_dir: String,
        json: bool,
    },
    PublicEvidenceValidate {
        manifest: String,
    },
    PublicEvidenceServiceHealth {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        health_path: String,
        first_seen_block: u64,
        last_seen_block: u64,
        reachable_observation_count: u64,
        signed_health_check_count: u64,
    },
    PublicEvidenceServiceHealthFromFile {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        health_path: String,
        observation_file: String,
    },
    PublicEvidenceServiceContent {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        content_path: String,
        content_root: Hash,
        observed_at_unix_seconds: u64,
        min_content_bytes: u64,
    },
    PublicEvidenceServiceContentFromBytes {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        content_path: String,
        observed_at_unix_seconds: u64,
        content_bytes: Vec<u8>,
    },
    PublicEvidenceServiceContentFromFile {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        content_path: String,
        observed_at_unix_seconds: u64,
        content_file: String,
    },
    PublicEvidenceRecordSummary {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        record_root: Hash,
        record_count: u64,
    },
    PublicEvidenceRecordArtifact {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        artifact_uri: String,
        record_root: Hash,
        record_count: u64,
    },
    PublicEvidenceRecordArtifactFromRoots {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        artifact_uri: String,
        record_roots: Vec<Hash>,
    },
    PublicEvidenceRecordArtifactFromFile {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        artifact_uri: String,
        record_file: String,
    },
    PublicEvidenceRecordSummaryFromRoots {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        record_roots: Vec<Hash>,
    },
    PublicEvidenceRecordSummaryFromFile {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        record_file: String,
    },
    PublicEvidenceNetworkObservation {
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
    PublicEvidenceNetworkObservationFromServiceLog {
        operator_id: Hash,
        listen_address: String,
        observed_at_unix_seconds: u64,
        service_log: String,
    },
    PublicEvidencePublication {
        bundle_id: Hash,
        public_uri: String,
        manifest_signer: Address,
        manifest_signature_count: u64,
        independent_auditor_count: u64,
    },
    PublicEvidenceAuditorRecord {
        bundle_id: Hash,
        public_uri: String,
        auditor_id: Address,
        audit_uri: String,
        observed_at_unix_seconds: u64,
    },
    PublicEvidenceRunWindow {
        bundle_id: Hash,
        manifest_signer: Address,
        run_started_at_unix_seconds: u64,
        run_ended_at_unix_seconds: u64,
        observed_blocks: u64,
    },
    PublicEvidenceRunWindowFromFile {
        bundle_id: Hash,
        manifest_signer: Address,
        block_observation_file: String,
    },
    PublicEvidenceNodeHeartbeat {
        role: PublicNodeRole,
        address: Address,
        operator_id: Hash,
        first_seen_block: u64,
        last_seen_block: u64,
        signed_heartbeat_count: u64,
    },
    PublicEvidenceNodeHeartbeatFromFile {
        role: PublicNodeRole,
        address: Address,
        operator_id: Hash,
        heartbeat_file: String,
    },
    PublicEvidenceOperatorAttestation {
        role: PublicNodeRole,
        address: Address,
        operator_id: Hash,
        identity_uri: String,
        observed_at_unix_seconds: u64,
    },
    PublicTestnetPreflight {
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
    match &cli_command {
        super::TvmdCommand::Miner(_)
        | super::TvmdCommand::Validator(_)
        | super::TvmdCommand::Proposer(_)
        | super::TvmdCommand::Node(_)
        | super::TvmdCommand::Localnet(_) => {
            super::local_execution::execute_local_cli_command(&cli_command)
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

pub(super) fn socket_addr_arg(value: String) -> SocketAddr {
    value.parse().expect("fixture socket address must parse")
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
            Self::MinerRegister { stake } => {
                super::TvmdCommand::Miner(MinerCommand::Register(StakeArgs { stake }))
            }
            Self::MinerStart {
                wallet,
                device,
                node,
            } => super::TvmdCommand::Miner(MinerCommand::Check(MinerCheckArgs {
                wallet: path_arg(wallet),
                device,
                node: multiaddr_arg(node),
            })),
            Self::MinerRun {
                wallet,
                device,
                node,
                listen,
                p2p_listen,
                data_dir,
                identity_seed,
                auth_token,
                max_requests,
            } => super::TvmdCommand::Miner(MinerCommand::Run(MinerRunArgs {
                wallet: path_arg(wallet),
                device,
                runtime: RoleRuntimeArgs {
                    node: multiaddr_arg(node),
                    node_runtime: NodeRuntimeArgs {
                        listen: socket_addr_arg(listen),
                        p2p_listen: multiaddr_arg(p2p_listen),
                        data_dir: path_arg(data_dir),
                        identity_seed: identity_seed.map(hash_arg),
                        auth_token,
                        max_requests,
                    },
                },
            })),
            Self::MinerStatus => super::TvmdCommand::Miner(MinerCommand::Status),
            Self::ValidatorRegister { stake } => {
                super::TvmdCommand::Validator(ValidatorCommand::Register(StakeArgs { stake }))
            }
            Self::ValidatorStart { wallet, node } => {
                super::TvmdCommand::Validator(ValidatorCommand::Check(ValidatorCheckArgs {
                    wallet: path_arg(wallet),
                    node: multiaddr_arg(node),
                }))
            }
            Self::ValidatorRun {
                wallet,
                node,
                listen,
                p2p_listen,
                data_dir,
                identity_seed,
                auth_token,
                max_requests,
            } => super::TvmdCommand::Validator(ValidatorCommand::Run(ValidatorRunArgs {
                wallet: path_arg(wallet),
                runtime: RoleRuntimeArgs {
                    node: multiaddr_arg(node),
                    node_runtime: NodeRuntimeArgs {
                        listen: socket_addr_arg(listen),
                        p2p_listen: multiaddr_arg(p2p_listen),
                        data_dir: path_arg(data_dir),
                        identity_seed: identity_seed.map(hash_arg),
                        auth_token,
                        max_requests,
                    },
                },
            })),
            Self::ValidatorStatus => super::TvmdCommand::Validator(ValidatorCommand::Status),
            Self::ProposerRun {
                wallet,
                node,
                listen,
                p2p_listen,
                data_dir,
                identity_seed,
                auth_token,
                max_requests,
            } => super::TvmdCommand::Proposer(ProposerCommand::Run(ValidatorRunArgs {
                wallet: path_arg(wallet),
                runtime: RoleRuntimeArgs {
                    node: multiaddr_arg(node),
                    node_runtime: NodeRuntimeArgs {
                        listen: socket_addr_arg(listen),
                        p2p_listen: multiaddr_arg(p2p_listen),
                        data_dir: path_arg(data_dir),
                        identity_seed: identity_seed.map(hash_arg),
                        auth_token,
                        max_requests,
                    },
                },
            })),
            Self::ServiceInit { data_dir } => {
                super::TvmdCommand::Node(NodeCommand::Init(DataDirArgs {
                    data_dir: path_arg(data_dir),
                }))
            }
            Self::ServicePeerAdd {
                data_dir,
                peer_id,
                address,
            } => {
                super::TvmdCommand::Node(NodeCommand::Peer(NodePeerCommand::Add(NodePeerAddArgs {
                    data_dir: path_arg(data_dir),
                    peer_id: peer_id_arg(peer_id),
                    address: multiaddr_arg(address),
                })))
            }
            Self::ServiceReadiness {
                p2p_listen,
                data_dir,
                identity_seed,
            } => super::TvmdCommand::Node(NodeCommand::Check(NodeCheckArgs {
                p2p_listen: multiaddr_arg(p2p_listen),
                data_dir: path_arg(data_dir),
                identity_seed: identity_seed.map(hash_arg),
            })),
            Self::ServiceServe {
                listen,
                p2p_listen,
                data_dir,
                identity_seed,
                auth_token,
                max_requests,
            } => super::TvmdCommand::Node(NodeCommand::Serve(NodeServeArgs {
                runtime: NodeRuntimeArgs {
                    listen: socket_addr_arg(listen),
                    p2p_listen: multiaddr_arg(p2p_listen),
                    data_dir: path_arg(data_dir),
                    identity_seed: identity_seed.map(hash_arg),
                    auth_token,
                    max_requests,
                },
            })),
            Self::ServiceStatus { data_dir } => {
                super::TvmdCommand::Node(NodeCommand::Status(DataDirArgs {
                    data_dir: path_arg(data_dir),
                }))
            }
            Self::ServiceBlock { data_dir, height } => {
                super::TvmdCommand::Node(NodeCommand::Block(NodeBlockArgs {
                    data_dir: path_arg(data_dir),
                    height,
                }))
            }
            Self::LocalTestnetSeed { data_dir } => {
                super::TvmdCommand::Localnet(LocalnetCommand::Seed(DataDirArgs {
                    data_dir: path_arg(data_dir),
                }))
            }
            Self::LocalCpuVerify { data_dir, json } => {
                super::TvmdCommand::Localnet(LocalnetCommand::Verify(LocalCpuVerifyArgs {
                    data_dir: path_arg(data_dir),
                    json,
                }))
            }
            Self::PublicEvidenceValidate { manifest } => super::TvmdCommand::Public(
                PublicCommand::Evidence(EvidenceCommand::Validate(PublicEvidenceManifestArgs {
                    manifest: path_arg(manifest),
                })),
            ),
            Self::PublicEvidenceServiceHealth {
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
            Self::PublicEvidenceServiceHealthFromFile {
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
            Self::PublicEvidenceServiceContent {
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
            Self::PublicEvidenceServiceContentFromBytes {
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
            Self::PublicEvidenceServiceContentFromFile {
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
            Self::PublicEvidenceRecordSummary {
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
            Self::PublicEvidenceRecordArtifact {
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
            Self::PublicEvidenceRecordArtifactFromRoots {
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
            Self::PublicEvidenceRecordArtifactFromFile {
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
            Self::PublicEvidenceRecordSummaryFromRoots {
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
            Self::PublicEvidenceRecordSummaryFromFile {
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
            Self::PublicEvidenceNetworkObservation {
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
            Self::PublicEvidenceNetworkObservationFromServiceLog {
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
            Self::PublicEvidencePublication {
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
            Self::PublicEvidenceAuditorRecord {
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
            Self::PublicEvidenceRunWindow {
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
            Self::PublicEvidenceRunWindowFromFile {
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
            Self::PublicEvidenceNodeHeartbeat {
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
            Self::PublicEvidenceNodeHeartbeatFromFile {
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
            Self::PublicEvidenceOperatorAttestation {
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
            Self::PublicTestnetPreflight { manifest } => {
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
