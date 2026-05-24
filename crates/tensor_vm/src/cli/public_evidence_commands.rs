use super::value_types::{AddressArg, HashArg, HexBytesArg};
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use clap::{Args, Subcommand, ValueEnum, ValueHint};
use libp2p::{Multiaddr, PeerId};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum PublicCommand {
    #[command(about = "Validate a public-testnet preflight manifest.")]
    Preflight(PublicTestnetManifestArgs),
    #[command(about = "Generate or validate public-testnet evidence.")]
    #[command(subcommand)]
    Evidence(EvidenceCommand),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum EvidenceCommand {
    #[command(about = "Validate a public-testnet evidence manifest.")]
    Validate(PublicEvidenceManifestArgs),
    #[command(about = "Generate publication evidence for an evidence bundle.")]
    Publish(PublicationArgs),
    #[command(about = "Generate independent auditor evidence.")]
    Audit(AuditorRecordArgs),
    #[command(about = "Generate run-window evidence.")]
    #[command(subcommand)]
    Run(EvidenceRunCommand),
    #[command(about = "Generate node and operator evidence.")]
    #[command(subcommand)]
    Node(EvidenceNodeCommand),
    #[command(about = "Generate public service evidence.")]
    #[command(subcommand)]
    Service(EvidenceServiceCommand),
    #[command(about = "Generate public libp2p network evidence.")]
    #[command(subcommand)]
    Network(EvidenceNetworkCommand),
    #[command(about = "Generate supporting-record evidence.")]
    #[command(subcommand)]
    Record(EvidenceRecordCommand),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum EvidenceServiceCommand {
    #[command(about = "Generate service health evidence.")]
    Health(ServiceHealthArgs),
    #[command(about = "Generate service health evidence from captured observations.")]
    HealthFile(ServiceHealthFromFileArgs),
    #[command(about = "Generate service content evidence from a known content root.")]
    Content(ServiceContentArgs),
    #[command(about = "Generate service content evidence from observed bytes.")]
    ContentBytes(ServiceContentFromBytesArgs),
    #[command(about = "Generate service content evidence from a captured file.")]
    ContentFile(ServiceContentFromFileArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
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

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum EvidenceNetworkCommand {
    #[command(about = "Generate public libp2p network runtime evidence.")]
    Observation(NetworkObservationArgs),
    #[command(about = "Generate public libp2p network runtime evidence from a service log.")]
    FromServiceLog(NetworkObservationFromServiceLogArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum EvidenceRunCommand {
    #[command(about = "Generate signed run-window evidence.")]
    Window(RunWindowArgs),
    #[command(about = "Generate signed run-window evidence from block observations.")]
    WindowFile(RunWindowFromFileArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum EvidenceNodeCommand {
    #[command(about = "Generate public node heartbeat evidence.")]
    Heartbeat(NodeHeartbeatArgs),
    #[command(about = "Generate public node heartbeat evidence from a file.")]
    HeartbeatFile(NodeHeartbeatFromFileArgs),
    #[command(about = "Generate public operator identity attestation evidence.")]
    OperatorAttestation(OperatorAttestationArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicTestnetManifestArgs {
    #[arg(value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub manifest: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicEvidenceManifestArgs {
    #[arg(value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub manifest: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceHealthArgs {
    #[arg(long)]
    pub kind: PublicServiceKindArg,
    #[arg(long, value_name = "HEX")]
    pub endpoint_id: HashArg,
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
    #[arg(long, value_name = "HEX")]
    pub endpoint_id: HashArg,
    #[arg(long, value_name = "URL")]
    pub public_url: String,
    #[arg(long, value_name = "PATH")]
    pub health_path: String,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub observation_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentArgs {
    #[arg(long)]
    pub kind: PublicServiceKindArg,
    #[arg(long, value_name = "HEX")]
    pub endpoint_id: HashArg,
    #[arg(long, value_name = "URL")]
    pub public_url: String,
    #[arg(long, value_name = "PATH")]
    pub content_path: String,
    #[arg(long, value_name = "HEX")]
    pub content_root: HashArg,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub observed_at: u64,
    #[arg(long, value_name = "BYTES")]
    pub min_content_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentFromBytesArgs {
    #[arg(long)]
    pub kind: PublicServiceKindArg,
    #[arg(long, value_name = "HEX")]
    pub endpoint_id: HashArg,
    #[arg(long, value_name = "URL")]
    pub public_url: String,
    #[arg(long, value_name = "PATH")]
    pub content_path: String,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub observed_at: u64,
    #[arg(long = "content-hex", value_name = "HEX")]
    pub content: HexBytesArg,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceContentFromFileArgs {
    #[arg(long)]
    pub kind: PublicServiceKindArg,
    #[arg(long, value_name = "HEX")]
    pub endpoint_id: HashArg,
    #[arg(long, value_name = "URL")]
    pub public_url: String,
    #[arg(long, value_name = "PATH")]
    pub content_path: String,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub observed_at: u64,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub content_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordSummaryArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX")]
    pub bundle_id: HashArg,
    #[arg(long, value_name = "HEX")]
    pub manifest_signer: AddressArg,
    #[arg(long, value_name = "HEX")]
    pub record_root: HashArg,
    #[arg(long, value_name = "N")]
    pub record_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX")]
    pub bundle_id: HashArg,
    #[arg(long, value_name = "HEX")]
    pub manifest_signer: AddressArg,
    #[arg(long, value_name = "URI")]
    pub artifact_uri: String,
    #[arg(long, value_name = "HEX")]
    pub record_root: HashArg,
    #[arg(long, value_name = "N")]
    pub record_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactFromRootsArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX")]
    pub bundle_id: HashArg,
    #[arg(long, value_name = "HEX")]
    pub manifest_signer: AddressArg,
    #[arg(long, value_name = "URI")]
    pub artifact_uri: String,
    #[arg(long, value_name = "HEX[,HEX...]", value_delimiter = ',', num_args = 1..)]
    pub record_roots: Vec<HashArg>,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordArtifactFromFileArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX")]
    pub bundle_id: HashArg,
    #[arg(long, value_name = "HEX")]
    pub manifest_signer: AddressArg,
    #[arg(long, value_name = "URI")]
    pub artifact_uri: String,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub record_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordSummaryFromRootsArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX")]
    pub bundle_id: HashArg,
    #[arg(long, value_name = "HEX")]
    pub manifest_signer: AddressArg,
    #[arg(long, value_name = "HEX[,HEX...]", value_delimiter = ',', num_args = 1..)]
    pub record_roots: Vec<HashArg>,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RecordSummaryFromFileArgs {
    #[arg(long)]
    pub kind: PublicEvidenceRecordKindArg,
    #[arg(long, value_name = "HEX")]
    pub bundle_id: HashArg,
    #[arg(long, value_name = "HEX")]
    pub manifest_signer: AddressArg,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub record_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NetworkObservationArgs {
    #[arg(long, value_name = "HEX")]
    pub operator_id: HashArg,
    #[arg(long, value_name = "PEER_ID")]
    pub peer_id: PeerId,
    #[arg(long, value_name = "MULTIADDR")]
    pub listen_address: Multiaddr,
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
    #[arg(long, value_name = "HEX")]
    pub operator_id: HashArg,
    #[arg(long, value_name = "MULTIADDR")]
    pub listen_address: Multiaddr,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub observed_at: u64,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub service_log: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicationArgs {
    #[arg(long, value_name = "HEX")]
    pub bundle_id: HashArg,
    #[arg(long, value_name = "URI")]
    pub public_uri: String,
    #[arg(long, value_name = "HEX")]
    pub manifest_signer: AddressArg,
    #[arg(long, value_name = "N")]
    pub manifest_signature_count: u64,
    #[arg(long, value_name = "N")]
    pub independent_auditor_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct AuditorRecordArgs {
    #[arg(long, value_name = "HEX")]
    pub bundle_id: HashArg,
    #[arg(long, value_name = "URI")]
    pub public_uri: String,
    #[arg(long, value_name = "HEX")]
    pub auditor_id: AddressArg,
    #[arg(long, value_name = "URI")]
    pub audit_uri: String,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub observed_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RunWindowArgs {
    #[arg(long, value_name = "HEX")]
    pub bundle_id: HashArg,
    #[arg(long, value_name = "HEX")]
    pub manifest_signer: AddressArg,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub started_at: u64,
    #[arg(long, value_name = "UNIX_SECONDS")]
    pub ended_at: u64,
    #[arg(long, value_name = "N")]
    pub observed_blocks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RunWindowFromFileArgs {
    #[arg(long, value_name = "HEX")]
    pub bundle_id: HashArg,
    #[arg(long, value_name = "HEX")]
    pub manifest_signer: AddressArg,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub block_observation_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeHeartbeatArgs {
    #[arg(long)]
    pub role: PublicNodeRoleArg,
    #[arg(long, value_name = "HEX")]
    pub address: AddressArg,
    #[arg(long, value_name = "HEX")]
    pub operator_id: HashArg,
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
    #[arg(long, value_name = "HEX")]
    pub address: AddressArg,
    #[arg(long, value_name = "HEX")]
    pub operator_id: HashArg,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub heartbeat_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct OperatorAttestationArgs {
    #[arg(long)]
    pub role: PublicNodeRoleArg,
    #[arg(long, value_name = "HEX")]
    pub address: AddressArg,
    #[arg(long, value_name = "HEX")]
    pub operator_id: HashArg,
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
