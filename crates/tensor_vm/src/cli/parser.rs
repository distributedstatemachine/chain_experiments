use super::CliCommand;
use super::arguments::{parse_hash_argument, parse_hash_list_argument};
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use crate::types::{Address, Hash};
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
#[command(
    name = "tvmd",
    version,
    about = "Run TensorVM nodes and produce public-testnet evidence."
)]
pub struct Cli {
    #[command(subcommand)]
    command: TopLevelCommand,
}

impl Cli {
    pub fn into_command(self) -> CliCommand {
        self.command.into_command()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
enum TopLevelCommand {
    Miner {
        #[command(subcommand)]
        command: MinerCommand,
    },
    Validator {
        #[command(subcommand)]
        command: ValidatorCommand,
    },
    Proposer {
        #[command(subcommand)]
        command: ProposerCommand,
    },
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
    LocalTestnet {
        #[command(subcommand)]
        command: LocalTestnetCommand,
    },
    LocalCpu {
        #[command(subcommand)]
        command: LocalCpuCommand,
    },
    PublicEvidence {
        #[command(subcommand)]
        command: PublicEvidenceCommand,
    },
    PublicTestnet {
        #[command(subcommand)]
        command: PublicTestnetCommand,
    },
}

impl TopLevelCommand {
    fn into_command(self) -> CliCommand {
        match self {
            TopLevelCommand::Miner { command } => command.into_command(),
            TopLevelCommand::Validator { command } => command.into_command(),
            TopLevelCommand::Proposer { command } => command.into_command(),
            TopLevelCommand::Service { command } => command.into_command(),
            TopLevelCommand::LocalTestnet { command } => command.into_command(),
            TopLevelCommand::LocalCpu { command } => command.into_command(),
            TopLevelCommand::PublicEvidence { command } => command.into_command(),
            TopLevelCommand::PublicTestnet { command } => command.into_command(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
enum MinerCommand {
    Register(StakeArgs),
    Start(MinerStartArgs),
    Run(MinerRunArgs),
    Status,
}

impl MinerCommand {
    fn into_command(self) -> CliCommand {
        match self {
            MinerCommand::Register(args) => CliCommand::MinerRegister { stake: args.stake },
            MinerCommand::Start(args) => CliCommand::MinerStart {
                wallet: args.wallet,
                device: args.device,
                node: args.node,
            },
            MinerCommand::Run(args) => CliCommand::MinerRun {
                wallet: args.wallet,
                device: args.device,
                node: args.node,
                listen: args.listen,
                p2p_listen: args.p2p_listen,
                data_dir: args.data_dir,
                identity_seed: args.identity_seed,
                auth_token: args.auth_token,
                max_requests: args.max_requests,
            },
            MinerCommand::Status => CliCommand::MinerStatus,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
enum ValidatorCommand {
    Register(StakeArgs),
    Start(ValidatorStartArgs),
    Run(ValidatorRunArgs),
    Status,
}

impl ValidatorCommand {
    fn into_command(self) -> CliCommand {
        match self {
            ValidatorCommand::Register(args) => CliCommand::ValidatorRegister { stake: args.stake },
            ValidatorCommand::Start(args) => CliCommand::ValidatorStart {
                wallet: args.wallet,
                node: args.node,
            },
            ValidatorCommand::Run(args) => CliCommand::ValidatorRun {
                wallet: args.wallet,
                node: args.node,
                listen: args.listen,
                p2p_listen: args.p2p_listen,
                data_dir: args.data_dir,
                identity_seed: args.identity_seed,
                auth_token: args.auth_token,
                max_requests: args.max_requests,
            },
            ValidatorCommand::Status => CliCommand::ValidatorStatus,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
enum ProposerCommand {
    Run(ValidatorRunArgs),
}

impl ProposerCommand {
    fn into_command(self) -> CliCommand {
        match self {
            ProposerCommand::Run(args) => CliCommand::ProposerRun {
                wallet: args.wallet,
                node: args.node,
                listen: args.listen,
                p2p_listen: args.p2p_listen,
                data_dir: args.data_dir,
                identity_seed: args.identity_seed,
                auth_token: args.auth_token,
                max_requests: args.max_requests,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
enum ServiceCommand {
    Init(DataDirArgs),
    Peer {
        #[command(subcommand)]
        command: ServicePeerCommand,
    },
    Readiness(ServiceReadinessArgs),
    Serve(ServiceServeArgs),
    Status(DataDirArgs),
    Block(ServiceBlockArgs),
}

impl ServiceCommand {
    fn into_command(self) -> CliCommand {
        match self {
            ServiceCommand::Init(args) => CliCommand::ServiceInit {
                data_dir: args.data_dir,
            },
            ServiceCommand::Peer { command } => command.into_command(),
            ServiceCommand::Readiness(args) => CliCommand::ServiceReadiness {
                p2p_listen: args.p2p_listen,
                data_dir: args.data_dir,
                identity_seed: args.identity_seed,
            },
            ServiceCommand::Serve(args) => CliCommand::ServiceServe {
                listen: args.listen,
                p2p_listen: args.p2p_listen,
                data_dir: args.data_dir,
                identity_seed: args.identity_seed,
                auth_token: args.auth_token,
                max_requests: args.max_requests,
            },
            ServiceCommand::Status(args) => CliCommand::ServiceStatus {
                data_dir: args.data_dir,
            },
            ServiceCommand::Block(args) => CliCommand::ServiceBlock {
                data_dir: args.data_dir,
                height: args.height,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
enum ServicePeerCommand {
    Add(ServicePeerAddArgs),
}

impl ServicePeerCommand {
    fn into_command(self) -> CliCommand {
        match self {
            ServicePeerCommand::Add(args) => CliCommand::ServicePeerAdd {
                data_dir: args.data_dir,
                peer_id: args.peer_id,
                address: args.address,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
enum LocalTestnetCommand {
    Seed(DataDirArgs),
}

impl LocalTestnetCommand {
    fn into_command(self) -> CliCommand {
        match self {
            LocalTestnetCommand::Seed(args) => CliCommand::LocalTestnetSeed {
                data_dir: args.data_dir,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
enum LocalCpuCommand {
    Verify(LocalCpuVerifyArgs),
}

impl LocalCpuCommand {
    fn into_command(self) -> CliCommand {
        match self {
            LocalCpuCommand::Verify(args) => CliCommand::LocalCpuVerify {
                data_dir: args.data_dir,
                json: args.json,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
enum PublicEvidenceCommand {
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
    fn into_command(self) -> CliCommand {
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

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
enum PublicTestnetCommand {
    Preflight(ManifestArgs),
}

impl PublicTestnetCommand {
    fn into_command(self) -> CliCommand {
        match self {
            PublicTestnetCommand::Preflight(args) => CliCommand::PublicTestnetPreflight {
                manifest: args.manifest,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct StakeArgs {
    #[arg(long)]
    stake: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct DataDirArgs {
    #[arg(long)]
    data_dir: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct ManifestArgs {
    #[arg(long)]
    manifest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct MinerStartArgs {
    #[arg(long)]
    wallet: String,
    #[arg(long)]
    device: String,
    #[arg(long)]
    node: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct MinerRunArgs {
    #[arg(long)]
    wallet: String,
    #[arg(long)]
    device: String,
    #[arg(long)]
    node: String,
    #[arg(long)]
    listen: String,
    #[arg(long)]
    p2p_listen: String,
    #[arg(long)]
    data_dir: String,
    #[arg(long, value_parser = parse_hash_value)]
    identity_seed: Option<Hash>,
    #[arg(long)]
    auth_token: String,
    #[arg(long)]
    max_requests: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct ValidatorStartArgs {
    #[arg(long)]
    wallet: String,
    #[arg(long)]
    node: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct ValidatorRunArgs {
    #[arg(long)]
    wallet: String,
    #[arg(long)]
    node: String,
    #[arg(long)]
    listen: String,
    #[arg(long)]
    p2p_listen: String,
    #[arg(long)]
    data_dir: String,
    #[arg(long, value_parser = parse_hash_value)]
    identity_seed: Option<Hash>,
    #[arg(long)]
    auth_token: String,
    #[arg(long)]
    max_requests: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct ServicePeerAddArgs {
    #[arg(long)]
    data_dir: String,
    #[arg(long)]
    peer_id: String,
    #[arg(long)]
    address: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct ServiceReadinessArgs {
    #[arg(long)]
    p2p_listen: String,
    #[arg(long)]
    data_dir: String,
    #[arg(long, value_parser = parse_hash_value)]
    identity_seed: Option<Hash>,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct ServiceServeArgs {
    #[arg(long)]
    listen: String,
    #[arg(long)]
    p2p_listen: String,
    #[arg(long)]
    data_dir: String,
    #[arg(long, value_parser = parse_hash_value)]
    identity_seed: Option<Hash>,
    #[arg(long)]
    auth_token: String,
    #[arg(long)]
    max_requests: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct ServiceBlockArgs {
    #[arg(long)]
    data_dir: String,
    #[arg(long)]
    height: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct LocalCpuVerifyArgs {
    #[arg(long)]
    data_dir: String,
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct ServiceHealthArgs {
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
struct ServiceHealthFromFileArgs {
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
struct ServiceContentArgs {
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
struct ServiceContentFromBytesArgs {
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
struct ServiceContentFromFileArgs {
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
struct RecordSummaryArgs {
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
struct RecordArtifactArgs {
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
struct RecordArtifactFromRootsArgs {
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
struct RecordArtifactFromFileArgs {
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
struct RecordSummaryFromRootsArgs {
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
struct RecordSummaryFromFileArgs {
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
struct NetworkObservationArgs {
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
struct NetworkObservationFromServiceLogArgs {
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
struct PublicationArgs {
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
struct AuditorRecordArgs {
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
struct RunWindowArgs {
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
struct RunWindowFromFileArgs {
    #[arg(long, value_parser = parse_hash_value)]
    bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    manifest_signer: Address,
    #[arg(long)]
    block_observation_file: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct NodeHeartbeatArgs {
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
struct NodeHeartbeatFromFileArgs {
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
struct OperatorAttestationArgs {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum PublicServiceKindArg {
    Rpc,
    Explorer,
    Faucet,
    Telemetry,
}

impl From<PublicServiceKindArg> for PublicServiceKind {
    fn from(kind: PublicServiceKindArg) -> Self {
        match kind {
            PublicServiceKindArg::Rpc => Self::Rpc,
            PublicServiceKindArg::Explorer => Self::Explorer,
            PublicServiceKindArg::Faucet => Self::Faucet,
            PublicServiceKindArg::Telemetry => Self::Telemetry,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum PublicNodeRoleArg {
    Miner,
    Validator,
}

impl From<PublicNodeRoleArg> for PublicNodeRole {
    fn from(role: PublicNodeRoleArg) -> Self {
        match role {
            PublicNodeRoleArg::Miner => Self::Miner,
            PublicNodeRoleArg::Validator => Self::Validator,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum PublicEvidenceRecordKindArg {
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct HashList(Vec<Hash>);

fn parse_hash_value(value: &str) -> std::result::Result<Hash, String> {
    parse_hash_argument(value).map_err(|error| error.to_string())
}

fn parse_hash_list_value(value: &str) -> std::result::Result<HashList, String> {
    parse_hash_list_argument(value)
        .map(HashList)
        .map_err(|error| error.to_string())
}
