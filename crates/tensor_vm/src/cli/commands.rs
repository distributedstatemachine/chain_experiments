use super::arguments::{parse_hash_argument, parse_hash_list_argument};
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use crate::types::{Address, Hash};
use clap::{Args, Parser, Subcommand, ValueEnum, ValueHint};
use libp2p::Multiaddr;
use std::net::SocketAddr;

const DEFAULT_DATA_DIR: &str = ".tensorvm";
const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:8545";
const DEFAULT_P2P_LISTEN_ADDR: &str = "/ip4/127.0.0.1/tcp/4001";
const DEFAULT_MAX_REQUESTS: usize = 0;

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
#[command(
    name = "tvmd",
    version,
    about = "Run TensorVM nodes and generate public-testnet evidence.",
    propagate_version = true,
    arg_required_else_help = true
)]
pub struct TvmdCli {
    #[command(subcommand)]
    pub command: TvmdCommand,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum TvmdCommand {
    #[command(about = "Register, start, run, or inspect a miner node.")]
    Miner {
        #[command(subcommand)]
        command: MinerCommand,
    },
    #[command(about = "Register, start, run, or inspect a validator node.")]
    Validator {
        #[command(subcommand)]
        command: ValidatorCommand,
    },
    #[command(about = "Run a proposer service role.")]
    Proposer {
        #[command(subcommand)]
        command: ProposerCommand,
    },
    #[command(about = "Manage the local service process and its node store.")]
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
    #[command(about = "Seed or manage the local TensorVM testnet.")]
    LocalTestnet {
        #[command(subcommand)]
        command: LocalTestnetCommand,
    },
    #[command(about = "Inspect the local CPU testnet state.")]
    LocalCpu {
        #[command(subcommand)]
        command: LocalCpuCommand,
    },
    #[command(about = "Generate or validate public-testnet evidence records.")]
    PublicEvidence {
        #[command(subcommand)]
        command: PublicEvidenceCommand,
    },
    #[command(about = "Validate public-testnet launch preflight manifests.")]
    PublicTestnet {
        #[command(subcommand)]
        command: PublicTestnetCommand,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MinerCommand {
    #[command(about = "Check miner registration stake requirements.")]
    Register(StakeArgs),
    #[command(about = "Check miner startup inputs without running a service.")]
    Start(MinerStartArgs),
    #[command(about = "Run a miner service.")]
    Run(MinerRunArgs),
    #[command(about = "Show miner readiness metadata.")]
    Status,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ValidatorCommand {
    #[command(about = "Check validator registration stake requirements.")]
    Register(StakeArgs),
    #[command(about = "Check validator startup inputs without running a service.")]
    Start(ValidatorStartArgs),
    #[command(about = "Run a validator service.")]
    Run(ValidatorRunArgs),
    #[command(about = "Show validator readiness metadata.")]
    Status,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ProposerCommand {
    #[command(about = "Run a proposer service.")]
    Run(ValidatorRunArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ServiceCommand {
    #[command(about = "Initialize the service node store.")]
    Init(DataDirArgs),
    #[command(about = "Manage libp2p peers.")]
    Peer {
        #[command(subcommand)]
        command: ServicePeerCommand,
    },
    #[command(about = "Check libp2p and node-store readiness.")]
    Readiness(ServiceReadinessArgs),
    #[command(about = "Serve RPC, explorer, faucet, telemetry, and libp2p.")]
    Serve(ServiceServeArgs),
    #[command(about = "Show node-store status.")]
    Status(DataDirArgs),
    #[command(about = "Show one persisted block from the node store.")]
    Block(ServiceBlockArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ServicePeerCommand {
    #[command(about = "Add a libp2p bootstrap peer to the node store.")]
    Add(ServicePeerAddArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum LocalTestnetCommand {
    #[command(about = "Seed local CPU testnet data.")]
    Seed(DataDirArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum LocalCpuCommand {
    #[command(about = "Verify local CPU testnet state.")]
    Verify(LocalCpuVerifyArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum PublicTestnetCommand {
    #[command(about = "Validate a public-testnet preflight manifest.")]
    Preflight(PublicTestnetManifestArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum PublicEvidenceCommand {
    #[command(about = "Validate a public-testnet evidence manifest.")]
    Validate(PublicEvidenceManifestArgs),
    #[command(about = "Generate service health evidence.")]
    ServiceHealth(ServiceHealthArgs),
    #[command(about = "Generate service health evidence from captured observations.")]
    ServiceHealthFromFile(ServiceHealthFromFileArgs),
    #[command(about = "Generate service content evidence from a known content root.")]
    ServiceContent(ServiceContentArgs),
    #[command(about = "Generate service content evidence from observed bytes.")]
    ServiceContentFromBytes(ServiceContentFromBytesArgs),
    #[command(about = "Generate service content evidence from a captured file.")]
    ServiceContentFromFile(ServiceContentFromFileArgs),
    #[command(about = "Generate a supporting-record summary.")]
    RecordSummary(RecordSummaryArgs),
    #[command(about = "Generate a supporting-record artifact locator.")]
    RecordArtifact(RecordArtifactArgs),
    #[command(about = "Generate a supporting-record artifact locator from roots.")]
    RecordArtifactFromRoots(RecordArtifactFromRootsArgs),
    #[command(about = "Generate a supporting-record artifact locator from a file.")]
    RecordArtifactFromFile(RecordArtifactFromFileArgs),
    #[command(about = "Generate a supporting-record summary from roots.")]
    RecordSummaryFromRoots(RecordSummaryFromRootsArgs),
    #[command(about = "Generate a supporting-record summary from a file.")]
    RecordSummaryFromFile(RecordSummaryFromFileArgs),
    #[command(about = "Generate public libp2p network runtime evidence.")]
    NetworkObservation(NetworkObservationArgs),
    #[command(about = "Generate public libp2p network runtime evidence from a service log.")]
    NetworkObservationFromServiceLog(NetworkObservationFromServiceLogArgs),
    #[command(about = "Generate publication evidence for an evidence bundle.")]
    Publication(PublicationArgs),
    #[command(about = "Generate independent auditor evidence.")]
    AuditorRecord(AuditorRecordArgs),
    #[command(about = "Generate signed run-window evidence.")]
    RunWindow(RunWindowArgs),
    #[command(about = "Generate signed run-window evidence from block observations.")]
    RunWindowFromFile(RunWindowFromFileArgs),
    #[command(about = "Generate public node heartbeat evidence.")]
    NodeHeartbeat(NodeHeartbeatArgs),
    #[command(about = "Generate public node heartbeat evidence from a file.")]
    NodeHeartbeatFromFile(NodeHeartbeatFromFileArgs),
    #[command(about = "Generate public operator identity attestation evidence.")]
    OperatorAttestation(OperatorAttestationArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct StakeArgs {
    #[arg(long, value_name = "TOKENS")]
    pub stake: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct MinerStartArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub wallet: String,
    #[arg(long, default_value = "cpu", value_name = "DEVICE")]
    pub device: String,
    #[arg(long, default_value = DEFAULT_P2P_LISTEN_ADDR, value_name = "MULTIADDR", value_parser = parse_multiaddr_value)]
    pub node: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct MinerRunArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub wallet: String,
    #[arg(long, default_value = "cpu", value_name = "DEVICE")]
    pub device: String,
    #[command(flatten)]
    pub runtime: RoleRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ValidatorStartArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub wallet: String,
    #[arg(long, default_value = DEFAULT_P2P_LISTEN_ADDR, value_name = "MULTIADDR", value_parser = parse_multiaddr_value)]
    pub node: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ValidatorRunArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub wallet: String,
    #[command(flatten)]
    pub runtime: RoleRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RoleRuntimeArgs {
    #[arg(long, default_value = DEFAULT_P2P_LISTEN_ADDR, value_name = "MULTIADDR", value_parser = parse_multiaddr_value)]
    pub node: String,
    #[command(flatten)]
    pub service: ServiceRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceRuntimeArgs {
    #[arg(long, env = "TVMD_LISTEN", default_value = DEFAULT_LISTEN_ADDR, value_name = "ADDR", value_parser = parse_socket_addr_value)]
    pub listen: String,
    #[arg(long, env = "TVMD_P2P_LISTEN", default_value = DEFAULT_P2P_LISTEN_ADDR, value_name = "MULTIADDR", value_parser = parse_multiaddr_value)]
    pub p2p_listen: String,
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: String,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub identity_seed: Option<Hash>,
    #[arg(long, env = "TVMD_AUTH_TOKEN", value_name = "TOKEN")]
    pub auth_token: String,
    #[arg(long, env = "TVMD_MAX_REQUESTS", default_value_t = DEFAULT_MAX_REQUESTS, value_name = "N")]
    pub max_requests: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct DataDirArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct LocalCpuVerifyArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServicePeerAddArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: String,
    #[arg(long, value_name = "PEER_ID")]
    pub peer_id: String,
    #[arg(long, value_name = "MULTIADDR", value_parser = parse_multiaddr_value)]
    pub address: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceReadinessArgs {
    #[arg(long, env = "TVMD_P2P_LISTEN", default_value = DEFAULT_P2P_LISTEN_ADDR, value_name = "MULTIADDR", value_parser = parse_multiaddr_value)]
    pub p2p_listen: String,
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: String,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub identity_seed: Option<Hash>,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceServeArgs {
    #[command(flatten)]
    pub runtime: ServiceRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceBlockArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: String,
    #[arg(long, value_name = "HEIGHT")]
    pub height: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicTestnetManifestArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub manifest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicEvidenceManifestArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub manifest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceHealthArgs {
    #[arg(long)]
    pub kind: PublicServiceKindArg,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub endpoint_id: Hash,
    #[arg(long, value_name = "URL")]
    pub public_url: String,
    #[arg(long, value_name = "PATH")]
    pub health_path: String,
    #[arg(long, value_name = "HEIGHT")]
    pub first_block: u64,
    #[arg(long, value_name = "HEIGHT")]
    pub last_block: u64,
    #[arg(long, value_name = "N")]
    pub reachable_count: u64,
    #[arg(long, value_name = "N")]
    pub signed_health_check_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceHealthFromFileArgs {
    #[arg(long)]
    pub kind: PublicServiceKindArg,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub endpoint_id: Hash,
    #[arg(long, value_name = "URL")]
    pub public_url: String,
    #[arg(long, value_name = "PATH")]
    pub health_path: String,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub observation_file: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentArgs {
    #[arg(long)]
    pub kind: PublicServiceKindArg,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub endpoint_id: Hash,
    #[arg(long, value_name = "URL")]
    pub public_url: String,
    #[arg(long, value_name = "PATH")]
    pub content_path: String,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub content_root: Hash,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub observed_at: u64,
    #[arg(long, value_name = "BYTES")]
    pub min_content_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentFromBytesArgs {
    #[arg(long)]
    pub kind: PublicServiceKindArg,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub endpoint_id: Hash,
    #[arg(long, value_name = "URL")]
    pub public_url: String,
    #[arg(long, value_name = "PATH")]
    pub content_path: String,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub observed_at: u64,
    #[arg(long, value_name = "HEX")]
    pub content_hex: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentFromFileArgs {
    #[arg(long)]
    pub kind: PublicServiceKindArg,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub endpoint_id: Hash,
    #[arg(long, value_name = "URL")]
    pub public_url: String,
    #[arg(long, value_name = "PATH")]
    pub content_path: String,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub observed_at: u64,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub content_file: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordSummaryArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub record_root: Hash,
    #[arg(long, value_name = "N")]
    pub record_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long, value_name = "URI")]
    pub artifact_uri: String,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub record_root: Hash,
    #[arg(long, value_name = "N")]
    pub record_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactFromRootsArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long, value_name = "URI")]
    pub artifact_uri: String,
    #[arg(long, value_name = "HEX[,HEX...]", value_parser = parse_hash_list_value)]
    pub record_roots: HashList,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactFromFileArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long, value_name = "URI")]
    pub artifact_uri: String,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub record_file: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordSummaryFromRootsArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long, value_name = "HEX[,HEX...]", value_parser = parse_hash_list_value)]
    pub record_roots: HashList,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordSummaryFromFileArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub record_file: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NetworkObservationArgs {
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub operator_id: Hash,
    #[arg(long, value_name = "PEER_ID")]
    pub peer_id: String,
    #[arg(long, value_name = "MULTIADDR")]
    pub listen_address: String,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub observed_at: u64,
    #[arg(long, value_name = "N")]
    pub gossip_topics: u64,
    #[arg(long, value_name = "N")]
    pub request_response_protocols: u64,
    #[arg(long, value_name = "N")]
    pub bootstrap_peers: u64,
    #[arg(long, value_name = "BYTES")]
    pub max_transmit_bytes: u64,
    #[arg(long, value_name = "SECONDS")]
    pub request_timeout_seconds: u64,
    #[arg(long, value_name = "N")]
    pub max_concurrent_streams: u64,
    #[arg(long, value_name = "SECONDS")]
    pub idle_timeout_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NetworkObservationFromServiceLogArgs {
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub operator_id: Hash,
    #[arg(long, value_name = "MULTIADDR")]
    pub listen_address: String,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub observed_at: u64,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub service_log: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicationArgs {
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_name = "URI")]
    pub public_uri: String,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long, value_name = "N")]
    pub manifest_signature_count: u64,
    #[arg(long, value_name = "N")]
    pub independent_auditor_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct AuditorRecordArgs {
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_name = "URI")]
    pub public_uri: String,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub auditor_id: Hash,
    #[arg(long, value_name = "URI")]
    pub audit_uri: String,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub observed_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RunWindowArgs {
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub started_at: u64,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub ended_at: u64,
    #[arg(long, value_name = "N")]
    pub observed_blocks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RunWindowFromFileArgs {
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub bundle_id: Hash,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub manifest_signer: Address,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub block_observation_file: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeHeartbeatArgs {
    #[arg(long)]
    pub role: PublicNodeRoleArg,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub address: Address,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub operator_id: Hash,
    #[arg(long, value_name = "HEIGHT")]
    pub first_block: u64,
    #[arg(long, value_name = "HEIGHT")]
    pub last_block: u64,
    #[arg(long, value_name = "N")]
    pub heartbeat_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeHeartbeatFromFileArgs {
    #[arg(long)]
    pub role: PublicNodeRoleArg,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub address: Address,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub operator_id: Hash,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub heartbeat_file: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct OperatorAttestationArgs {
    #[arg(long)]
    pub role: PublicNodeRoleArg,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub address: Address,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub operator_id: Hash,
    #[arg(long, value_name = "URI")]
    pub identity_uri: String,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub observed_at: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum PublicServiceKindArg {
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
#[value(rename_all = "kebab-case")]
pub enum PublicNodeRoleArg {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HashList(pub Vec<Hash>);

fn parse_hash_value(value: &str) -> std::result::Result<Hash, String> {
    parse_hash_argument(value).map_err(|error| error.to_string())
}

fn parse_hash_list_value(value: &str) -> std::result::Result<HashList, String> {
    parse_hash_list_argument(value)
        .map(HashList)
        .map_err(|error| error.to_string())
}

fn parse_socket_addr_value(value: &str) -> std::result::Result<String, String> {
    value
        .parse::<SocketAddr>()
        .map(|_| value.to_owned())
        .map_err(|_| "invalid socket address; expected host:port".to_owned())
}

fn parse_multiaddr_value(value: &str) -> std::result::Result<String, String> {
    value
        .parse::<Multiaddr>()
        .map(|_| value.to_owned())
        .map_err(|_| "invalid libp2p multiaddr".to_owned())
}
