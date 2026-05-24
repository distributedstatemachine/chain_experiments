use super::command_values::{HashList, parse_hash_list_value, parse_hash_value};
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use crate::types::{Address, Hash};
use clap::{Args, Subcommand, ValueEnum, ValueHint};

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
