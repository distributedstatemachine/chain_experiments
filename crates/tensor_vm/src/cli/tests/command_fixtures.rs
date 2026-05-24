use super::*;
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use crate::types::{Address, Hash};
use clap::Parser;
use libp2p::PeerId;
use std::net::SocketAddr;
use std::path::PathBuf;

pub(super) fn parse_test_cli(args: &[&str]) -> std::result::Result<CommandFixture, clap::Error> {
    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push("tvmd");
    argv.extend_from_slice(args);
    TvmdCli::try_parse_from(argv).map(|cli| CommandFixture::from(cli.command))
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
        content_hex: String,
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

pub(super) fn execute_command_fixture(command: &CommandFixture) -> crate::error::Result<String> {
    super::execute_cli_command(&command.clone().into_cli_command())
}

pub(super) fn describe_command_fixture(command: &CommandFixture) -> String {
    super::describe_cli_command(&command.clone().into_cli_command())
}

pub(super) fn path_arg(value: String) -> PathBuf {
    value.into()
}

pub(super) fn path_to_string(value: PathBuf) -> String {
    value.to_string_lossy().into_owned()
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

impl CommandFixture {
    fn into_cli_command(self) -> super::TvmdCommand {
        match self {
            Self::MinerRegister { stake } => super::TvmdCommand::Miner {
                command: MinerCommand::Register(StakeArgs { stake }),
            },
            Self::MinerStart {
                wallet,
                device,
                node,
            } => super::TvmdCommand::Miner {
                command: MinerCommand::Start(MinerStartArgs {
                    wallet: path_arg(wallet),
                    device,
                    node: multiaddr_arg(node),
                }),
            },
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
            } => super::TvmdCommand::Miner {
                command: MinerCommand::Run(MinerRunArgs {
                    wallet: path_arg(wallet),
                    device,
                    runtime: RoleRuntimeArgs {
                        node: multiaddr_arg(node),
                        service: ServiceRuntimeArgs {
                            listen: socket_addr_arg(listen),
                            p2p_listen: multiaddr_arg(p2p_listen),
                            data_dir: path_arg(data_dir),
                            identity_seed,
                            auth_token,
                            max_requests,
                        },
                    },
                }),
            },
            Self::MinerStatus => super::TvmdCommand::Miner {
                command: MinerCommand::Status,
            },
            Self::ValidatorRegister { stake } => super::TvmdCommand::Validator {
                command: ValidatorCommand::Register(StakeArgs { stake }),
            },
            Self::ValidatorStart { wallet, node } => super::TvmdCommand::Validator {
                command: ValidatorCommand::Start(ValidatorStartArgs {
                    wallet: path_arg(wallet),
                    node: multiaddr_arg(node),
                }),
            },
            Self::ValidatorRun {
                wallet,
                node,
                listen,
                p2p_listen,
                data_dir,
                identity_seed,
                auth_token,
                max_requests,
            } => super::TvmdCommand::Validator {
                command: ValidatorCommand::Run(ValidatorRunArgs {
                    wallet: path_arg(wallet),
                    runtime: RoleRuntimeArgs {
                        node: multiaddr_arg(node),
                        service: ServiceRuntimeArgs {
                            listen: socket_addr_arg(listen),
                            p2p_listen: multiaddr_arg(p2p_listen),
                            data_dir: path_arg(data_dir),
                            identity_seed,
                            auth_token,
                            max_requests,
                        },
                    },
                }),
            },
            Self::ValidatorStatus => super::TvmdCommand::Validator {
                command: ValidatorCommand::Status,
            },
            Self::ProposerRun {
                wallet,
                node,
                listen,
                p2p_listen,
                data_dir,
                identity_seed,
                auth_token,
                max_requests,
            } => super::TvmdCommand::Proposer {
                command: ProposerCommand::Run(ValidatorRunArgs {
                    wallet: path_arg(wallet),
                    runtime: RoleRuntimeArgs {
                        node: multiaddr_arg(node),
                        service: ServiceRuntimeArgs {
                            listen: socket_addr_arg(listen),
                            p2p_listen: multiaddr_arg(p2p_listen),
                            data_dir: path_arg(data_dir),
                            identity_seed,
                            auth_token,
                            max_requests,
                        },
                    },
                }),
            },
            Self::ServiceInit { data_dir } => super::TvmdCommand::Service {
                command: ServiceCommand::Init(DataDirArgs {
                    data_dir: path_arg(data_dir),
                }),
            },
            Self::ServicePeerAdd {
                data_dir,
                peer_id,
                address,
            } => super::TvmdCommand::Service {
                command: ServiceCommand::Peer {
                    command: ServicePeerCommand::Add(ServicePeerAddArgs {
                        data_dir: path_arg(data_dir),
                        peer_id: peer_id_arg(peer_id),
                        address: multiaddr_arg(address),
                    }),
                },
            },
            Self::ServiceReadiness {
                p2p_listen,
                data_dir,
                identity_seed,
            } => super::TvmdCommand::Service {
                command: ServiceCommand::Readiness(ServiceReadinessArgs {
                    p2p_listen: multiaddr_arg(p2p_listen),
                    data_dir: path_arg(data_dir),
                    identity_seed,
                }),
            },
            Self::ServiceServe {
                listen,
                p2p_listen,
                data_dir,
                identity_seed,
                auth_token,
                max_requests,
            } => super::TvmdCommand::Service {
                command: ServiceCommand::Serve(ServiceServeArgs {
                    runtime: ServiceRuntimeArgs {
                        listen: socket_addr_arg(listen),
                        p2p_listen: multiaddr_arg(p2p_listen),
                        data_dir: path_arg(data_dir),
                        identity_seed,
                        auth_token,
                        max_requests,
                    },
                }),
            },
            Self::ServiceStatus { data_dir } => super::TvmdCommand::Service {
                command: ServiceCommand::Status(DataDirArgs {
                    data_dir: path_arg(data_dir),
                }),
            },
            Self::ServiceBlock { data_dir, height } => super::TvmdCommand::Service {
                command: ServiceCommand::Block(ServiceBlockArgs {
                    data_dir: path_arg(data_dir),
                    height,
                }),
            },
            Self::LocalTestnetSeed { data_dir } => super::TvmdCommand::LocalTestnet {
                command: LocalTestnetCommand::Seed(DataDirArgs {
                    data_dir: path_arg(data_dir),
                }),
            },
            Self::LocalCpuVerify { data_dir, json } => super::TvmdCommand::LocalCpu {
                command: LocalCpuCommand::Verify(LocalCpuVerifyArgs {
                    data_dir: path_arg(data_dir),
                    json,
                }),
            },
            Self::PublicEvidenceValidate { manifest } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::Validate(PublicEvidenceManifestArgs {
                    manifest: path_arg(manifest),
                }),
            },
            Self::PublicEvidenceServiceHealth {
                kind,
                endpoint_id,
                public_url,
                health_path,
                first_seen_block,
                last_seen_block,
                reachable_observation_count,
                signed_health_check_count,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::ServiceHealth(ServiceHealthArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id,
                    public_url,
                    health_path,
                    first_block: first_seen_block,
                    last_block: last_seen_block,
                    reachable_count: reachable_observation_count,
                    signed_health_check_count,
                }),
            },
            Self::PublicEvidenceServiceHealthFromFile {
                kind,
                endpoint_id,
                public_url,
                health_path,
                observation_file,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::ServiceHealthFromFile(ServiceHealthFromFileArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id,
                    public_url,
                    health_path,
                    observation_file: path_arg(observation_file),
                }),
            },
            Self::PublicEvidenceServiceContent {
                kind,
                endpoint_id,
                public_url,
                content_path,
                content_root,
                observed_at_unix_seconds,
                min_content_bytes,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::ServiceContent(ServiceContentArgs {
                    kind: service_kind_arg(kind),
                    endpoint_id,
                    public_url,
                    content_path,
                    content_root,
                    observed_at: observed_at_unix_seconds,
                    min_content_bytes,
                }),
            },
            Self::PublicEvidenceServiceContentFromBytes {
                kind,
                endpoint_id,
                public_url,
                content_path,
                observed_at_unix_seconds,
                content_hex,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::ServiceContentFromBytes(
                    ServiceContentFromBytesArgs {
                        kind: service_kind_arg(kind),
                        endpoint_id,
                        public_url,
                        content_path,
                        observed_at: observed_at_unix_seconds,
                        content_hex,
                    },
                ),
            },
            Self::PublicEvidenceServiceContentFromFile {
                kind,
                endpoint_id,
                public_url,
                content_path,
                observed_at_unix_seconds,
                content_file,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::ServiceContentFromFile(
                    ServiceContentFromFileArgs {
                        kind: service_kind_arg(kind),
                        endpoint_id,
                        public_url,
                        content_path,
                        observed_at: observed_at_unix_seconds,
                        content_file: path_arg(content_file),
                    },
                ),
            },
            Self::PublicEvidenceRecordSummary {
                kind,
                bundle_id,
                manifest_signer,
                record_root,
                record_count,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::RecordSummary(RecordSummaryArgs {
                    kind: record_kind_arg(kind),
                    bundle_id,
                    manifest_signer,
                    record_root,
                    record_count,
                }),
            },
            Self::PublicEvidenceRecordArtifact {
                kind,
                bundle_id,
                manifest_signer,
                artifact_uri,
                record_root,
                record_count,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::RecordArtifact(RecordArtifactArgs {
                    kind: record_kind_arg(kind),
                    bundle_id,
                    manifest_signer,
                    artifact_uri,
                    record_root,
                    record_count,
                }),
            },
            Self::PublicEvidenceRecordArtifactFromRoots {
                kind,
                bundle_id,
                manifest_signer,
                artifact_uri,
                record_roots,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::RecordArtifactFromRoots(
                    RecordArtifactFromRootsArgs {
                        kind: record_kind_arg(kind),
                        bundle_id,
                        manifest_signer,
                        artifact_uri,
                        record_roots,
                    },
                ),
            },
            Self::PublicEvidenceRecordArtifactFromFile {
                kind,
                bundle_id,
                manifest_signer,
                artifact_uri,
                record_file,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::RecordArtifactFromFile(
                    RecordArtifactFromFileArgs {
                        kind: record_kind_arg(kind),
                        bundle_id,
                        manifest_signer,
                        artifact_uri,
                        record_file: path_arg(record_file),
                    },
                ),
            },
            Self::PublicEvidenceRecordSummaryFromRoots {
                kind,
                bundle_id,
                manifest_signer,
                record_roots,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::RecordSummaryFromRoots(
                    RecordSummaryFromRootsArgs {
                        kind: record_kind_arg(kind),
                        bundle_id,
                        manifest_signer,
                        record_roots,
                    },
                ),
            },
            Self::PublicEvidenceRecordSummaryFromFile {
                kind,
                bundle_id,
                manifest_signer,
                record_file,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::RecordSummaryFromFile(RecordSummaryFromFileArgs {
                    kind: record_kind_arg(kind),
                    bundle_id,
                    manifest_signer,
                    record_file: path_arg(record_file),
                }),
            },
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
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::NetworkObservation(NetworkObservationArgs {
                    operator_id,
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
            },
            Self::PublicEvidenceNetworkObservationFromServiceLog {
                operator_id,
                listen_address,
                observed_at_unix_seconds,
                service_log,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::NetworkObservationFromServiceLog(
                    NetworkObservationFromServiceLogArgs {
                        operator_id,
                        listen_address: multiaddr_arg(listen_address),
                        observed_at: observed_at_unix_seconds,
                        service_log: path_arg(service_log),
                    },
                ),
            },
            Self::PublicEvidencePublication {
                bundle_id,
                public_uri,
                manifest_signer,
                manifest_signature_count,
                independent_auditor_count,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::Publication(PublicationArgs {
                    bundle_id,
                    public_uri,
                    manifest_signer,
                    manifest_signature_count,
                    independent_auditor_count,
                }),
            },
            Self::PublicEvidenceAuditorRecord {
                bundle_id,
                public_uri,
                auditor_id,
                audit_uri,
                observed_at_unix_seconds,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::AuditorRecord(AuditorRecordArgs {
                    bundle_id,
                    public_uri,
                    auditor_id,
                    audit_uri,
                    observed_at: observed_at_unix_seconds,
                }),
            },
            Self::PublicEvidenceRunWindow {
                bundle_id,
                manifest_signer,
                run_started_at_unix_seconds,
                run_ended_at_unix_seconds,
                observed_blocks,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::RunWindow(RunWindowArgs {
                    bundle_id,
                    manifest_signer,
                    started_at: run_started_at_unix_seconds,
                    ended_at: run_ended_at_unix_seconds,
                    observed_blocks,
                }),
            },
            Self::PublicEvidenceRunWindowFromFile {
                bundle_id,
                manifest_signer,
                block_observation_file,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::RunWindowFromFile(RunWindowFromFileArgs {
                    bundle_id,
                    manifest_signer,
                    block_observation_file: path_arg(block_observation_file),
                }),
            },
            Self::PublicEvidenceNodeHeartbeat {
                role,
                address,
                operator_id,
                first_seen_block,
                last_seen_block,
                signed_heartbeat_count,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::NodeHeartbeat(NodeHeartbeatArgs {
                    role: node_role_arg(role),
                    address,
                    operator_id,
                    first_block: first_seen_block,
                    last_block: last_seen_block,
                    heartbeat_count: signed_heartbeat_count,
                }),
            },
            Self::PublicEvidenceNodeHeartbeatFromFile {
                role,
                address,
                operator_id,
                heartbeat_file,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::NodeHeartbeatFromFile(NodeHeartbeatFromFileArgs {
                    role: node_role_arg(role),
                    address,
                    operator_id,
                    heartbeat_file: path_arg(heartbeat_file),
                }),
            },
            Self::PublicEvidenceOperatorAttestation {
                role,
                address,
                operator_id,
                identity_uri,
                observed_at_unix_seconds,
            } => super::TvmdCommand::PublicEvidence {
                command: PublicEvidenceCommand::OperatorAttestation(OperatorAttestationArgs {
                    role: node_role_arg(role),
                    address,
                    operator_id,
                    identity_uri,
                    observed_at: observed_at_unix_seconds,
                }),
            },
            Self::PublicTestnetPreflight { manifest } => super::TvmdCommand::PublicTestnet {
                command: PublicTestnetCommand::Preflight(PublicTestnetManifestArgs {
                    manifest: path_arg(manifest),
                }),
            },
        }
    }
}

impl From<super::TvmdCommand> for CommandFixture {
    fn from(command: super::TvmdCommand) -> Self {
        match command {
            super::TvmdCommand::Miner { command } => match command {
                MinerCommand::Register(args) => Self::MinerRegister { stake: args.stake },
                MinerCommand::Start(args) => Self::MinerStart {
                    wallet: path_to_string(args.wallet),
                    device: args.device,
                    node: args.node.to_string(),
                },
                MinerCommand::Run(args) => Self::MinerRun {
                    wallet: path_to_string(args.wallet),
                    device: args.device,
                    node: args.runtime.node.to_string(),
                    listen: args.runtime.service.listen.to_string(),
                    p2p_listen: args.runtime.service.p2p_listen.to_string(),
                    data_dir: path_to_string(args.runtime.service.data_dir),
                    identity_seed: args.runtime.service.identity_seed,
                    auth_token: args.runtime.service.auth_token,
                    max_requests: args.runtime.service.max_requests,
                },
                MinerCommand::Status => Self::MinerStatus,
            },
            super::TvmdCommand::Validator { command } => match command {
                ValidatorCommand::Register(args) => Self::ValidatorRegister { stake: args.stake },
                ValidatorCommand::Start(args) => Self::ValidatorStart {
                    wallet: path_to_string(args.wallet),
                    node: args.node.to_string(),
                },
                ValidatorCommand::Run(args) => Self::ValidatorRun {
                    wallet: path_to_string(args.wallet),
                    node: args.runtime.node.to_string(),
                    listen: args.runtime.service.listen.to_string(),
                    p2p_listen: args.runtime.service.p2p_listen.to_string(),
                    data_dir: path_to_string(args.runtime.service.data_dir),
                    identity_seed: args.runtime.service.identity_seed,
                    auth_token: args.runtime.service.auth_token,
                    max_requests: args.runtime.service.max_requests,
                },
                ValidatorCommand::Status => Self::ValidatorStatus,
            },
            super::TvmdCommand::Proposer { command } => match command {
                ProposerCommand::Run(args) => Self::ProposerRun {
                    wallet: path_to_string(args.wallet),
                    node: args.runtime.node.to_string(),
                    listen: args.runtime.service.listen.to_string(),
                    p2p_listen: args.runtime.service.p2p_listen.to_string(),
                    data_dir: path_to_string(args.runtime.service.data_dir),
                    identity_seed: args.runtime.service.identity_seed,
                    auth_token: args.runtime.service.auth_token,
                    max_requests: args.runtime.service.max_requests,
                },
            },
            super::TvmdCommand::Service { command } => match command {
                ServiceCommand::Init(args) => Self::ServiceInit {
                    data_dir: path_to_string(args.data_dir),
                },
                ServiceCommand::Peer {
                    command: ServicePeerCommand::Add(args),
                } => Self::ServicePeerAdd {
                    data_dir: path_to_string(args.data_dir),
                    peer_id: args.peer_id.to_string(),
                    address: args.address.to_string(),
                },
                ServiceCommand::Readiness(args) => Self::ServiceReadiness {
                    p2p_listen: args.p2p_listen.to_string(),
                    data_dir: path_to_string(args.data_dir),
                    identity_seed: args.identity_seed,
                },
                ServiceCommand::Serve(args) => Self::ServiceServe {
                    listen: args.runtime.listen.to_string(),
                    p2p_listen: args.runtime.p2p_listen.to_string(),
                    data_dir: path_to_string(args.runtime.data_dir),
                    identity_seed: args.runtime.identity_seed,
                    auth_token: args.runtime.auth_token,
                    max_requests: args.runtime.max_requests,
                },
                ServiceCommand::Status(args) => Self::ServiceStatus {
                    data_dir: path_to_string(args.data_dir),
                },
                ServiceCommand::Block(args) => Self::ServiceBlock {
                    data_dir: path_to_string(args.data_dir),
                    height: args.height,
                },
            },
            super::TvmdCommand::LocalTestnet { command } => match command {
                LocalTestnetCommand::Seed(args) => Self::LocalTestnetSeed {
                    data_dir: path_to_string(args.data_dir),
                },
            },
            super::TvmdCommand::LocalCpu { command } => match command {
                LocalCpuCommand::Verify(args) => Self::LocalCpuVerify {
                    data_dir: path_to_string(args.data_dir),
                    json: args.json,
                },
            },
            super::TvmdCommand::PublicEvidence { command } => match command {
                PublicEvidenceCommand::Validate(args) => Self::PublicEvidenceValidate {
                    manifest: path_to_string(args.manifest),
                },
                PublicEvidenceCommand::ServiceHealth(args) => Self::PublicEvidenceServiceHealth {
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
                    Self::PublicEvidenceServiceHealthFromFile {
                        kind: args.kind.into(),
                        endpoint_id: args.endpoint_id,
                        public_url: args.public_url,
                        health_path: args.health_path,
                        observation_file: path_to_string(args.observation_file),
                    }
                }
                PublicEvidenceCommand::ServiceContent(args) => Self::PublicEvidenceServiceContent {
                    kind: args.kind.into(),
                    endpoint_id: args.endpoint_id,
                    public_url: args.public_url,
                    content_path: args.content_path,
                    content_root: args.content_root,
                    observed_at_unix_seconds: args.observed_at,
                    min_content_bytes: args.min_content_bytes,
                },
                PublicEvidenceCommand::ServiceContentFromBytes(args) => {
                    Self::PublicEvidenceServiceContentFromBytes {
                        kind: args.kind.into(),
                        endpoint_id: args.endpoint_id,
                        public_url: args.public_url,
                        content_path: args.content_path,
                        observed_at_unix_seconds: args.observed_at,
                        content_hex: args.content_hex,
                    }
                }
                PublicEvidenceCommand::ServiceContentFromFile(args) => {
                    Self::PublicEvidenceServiceContentFromFile {
                        kind: args.kind.into(),
                        endpoint_id: args.endpoint_id,
                        public_url: args.public_url,
                        content_path: args.content_path,
                        observed_at_unix_seconds: args.observed_at,
                        content_file: path_to_string(args.content_file),
                    }
                }
                PublicEvidenceCommand::RecordSummary(args) => Self::PublicEvidenceRecordSummary {
                    kind: args.kind.into(),
                    bundle_id: args.bundle_id,
                    manifest_signer: args.manifest_signer,
                    record_root: args.record_root,
                    record_count: args.record_count,
                },
                PublicEvidenceCommand::RecordArtifact(args) => Self::PublicEvidenceRecordArtifact {
                    kind: args.kind.into(),
                    bundle_id: args.bundle_id,
                    manifest_signer: args.manifest_signer,
                    artifact_uri: args.artifact_uri,
                    record_root: args.record_root,
                    record_count: args.record_count,
                },
                PublicEvidenceCommand::RecordArtifactFromRoots(args) => {
                    Self::PublicEvidenceRecordArtifactFromRoots {
                        kind: args.kind.into(),
                        bundle_id: args.bundle_id,
                        manifest_signer: args.manifest_signer,
                        artifact_uri: args.artifact_uri,
                        record_roots: args.record_roots,
                    }
                }
                PublicEvidenceCommand::RecordArtifactFromFile(args) => {
                    Self::PublicEvidenceRecordArtifactFromFile {
                        kind: args.kind.into(),
                        bundle_id: args.bundle_id,
                        manifest_signer: args.manifest_signer,
                        artifact_uri: args.artifact_uri,
                        record_file: path_to_string(args.record_file),
                    }
                }
                PublicEvidenceCommand::RecordSummaryFromRoots(args) => {
                    Self::PublicEvidenceRecordSummaryFromRoots {
                        kind: args.kind.into(),
                        bundle_id: args.bundle_id,
                        manifest_signer: args.manifest_signer,
                        record_roots: args.record_roots,
                    }
                }
                PublicEvidenceCommand::RecordSummaryFromFile(args) => {
                    Self::PublicEvidenceRecordSummaryFromFile {
                        kind: args.kind.into(),
                        bundle_id: args.bundle_id,
                        manifest_signer: args.manifest_signer,
                        record_file: path_to_string(args.record_file),
                    }
                }
                PublicEvidenceCommand::NetworkObservation(args) => {
                    Self::PublicEvidenceNetworkObservation {
                        operator_id: args.operator_id,
                        peer_id: args.peer_id.to_string(),
                        listen_address: args.listen_address.to_string(),
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
                    Self::PublicEvidenceNetworkObservationFromServiceLog {
                        operator_id: args.operator_id,
                        listen_address: args.listen_address.to_string(),
                        observed_at_unix_seconds: args.observed_at,
                        service_log: path_to_string(args.service_log),
                    }
                }
                PublicEvidenceCommand::Publication(args) => Self::PublicEvidencePublication {
                    bundle_id: args.bundle_id,
                    public_uri: args.public_uri,
                    manifest_signer: args.manifest_signer,
                    manifest_signature_count: args.manifest_signature_count,
                    independent_auditor_count: args.independent_auditor_count,
                },
                PublicEvidenceCommand::AuditorRecord(args) => Self::PublicEvidenceAuditorRecord {
                    bundle_id: args.bundle_id,
                    public_uri: args.public_uri,
                    auditor_id: args.auditor_id,
                    audit_uri: args.audit_uri,
                    observed_at_unix_seconds: args.observed_at,
                },
                PublicEvidenceCommand::RunWindow(args) => Self::PublicEvidenceRunWindow {
                    bundle_id: args.bundle_id,
                    manifest_signer: args.manifest_signer,
                    run_started_at_unix_seconds: args.started_at,
                    run_ended_at_unix_seconds: args.ended_at,
                    observed_blocks: args.observed_blocks,
                },
                PublicEvidenceCommand::RunWindowFromFile(args) => {
                    Self::PublicEvidenceRunWindowFromFile {
                        bundle_id: args.bundle_id,
                        manifest_signer: args.manifest_signer,
                        block_observation_file: path_to_string(args.block_observation_file),
                    }
                }
                PublicEvidenceCommand::NodeHeartbeat(args) => Self::PublicEvidenceNodeHeartbeat {
                    role: args.role.into(),
                    address: args.address,
                    operator_id: args.operator_id,
                    first_seen_block: args.first_block,
                    last_seen_block: args.last_block,
                    signed_heartbeat_count: args.heartbeat_count,
                },
                PublicEvidenceCommand::NodeHeartbeatFromFile(args) => {
                    Self::PublicEvidenceNodeHeartbeatFromFile {
                        role: args.role.into(),
                        address: args.address,
                        operator_id: args.operator_id,
                        heartbeat_file: path_to_string(args.heartbeat_file),
                    }
                }
                PublicEvidenceCommand::OperatorAttestation(args) => {
                    Self::PublicEvidenceOperatorAttestation {
                        role: args.role.into(),
                        address: args.address,
                        operator_id: args.operator_id,
                        identity_uri: args.identity_uri,
                        observed_at_unix_seconds: args.observed_at,
                    }
                }
            },
            super::TvmdCommand::PublicTestnet { command } => match command {
                PublicTestnetCommand::Preflight(args) => Self::PublicTestnetPreflight {
                    manifest: path_to_string(args.manifest),
                },
            },
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
