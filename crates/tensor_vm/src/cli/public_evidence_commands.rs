pub use super::public_evidence_network_commands::{
    EvidenceNetworkCommand, NetworkObservationArgs, NetworkObservationFromServiceLogArgs,
};
pub use super::public_evidence_record_commands::{
    EvidenceRecordCommand, PublicEvidenceRecordKindArg, RecordArtifactArgs,
    RecordArtifactFromFileArgs, RecordArtifactFromRootsArgs, RecordSummaryArgs,
    RecordSummaryFromFileArgs, RecordSummaryFromRootsArgs,
};
pub use super::public_evidence_service_commands::{
    EvidenceServiceCommand, PublicServiceKindArg, ServiceContentArgs, ServiceContentFromBytesArgs,
    ServiceContentFromFileArgs, ServiceHealthArgs, ServiceHealthFromFileArgs,
};
use super::value_types::{AddressArg, HashArg};
use crate::testnet::PublicNodeRole;
use clap::{Args, Subcommand, ValueEnum, ValueHint};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum PublicCommand {
    #[command(about = "Validate a public-testnet preflight manifest.")]
    Preflight(PublicTestnetManifestArgs),
    #[command(about = "Generate or validate public-testnet evidence.")]
    #[command(subcommand)]
    Evidence(EvidenceCommand),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
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
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum EvidenceRunCommand {
    #[command(about = "Generate signed run-window evidence.")]
    Window(RunWindowArgs),
    #[command(about = "Generate signed run-window evidence from block observations.")]
    WindowFile(RunWindowFromFileArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
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
    #[arg(value_name = "PATH", value_hint = ValueHint::FilePath, help = "Public-testnet preflight manifest to validate.")]
    pub manifest: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicEvidenceManifestArgs {
    #[arg(value_name = "PATH", value_hint = ValueHint::FilePath, help = "Public-testnet evidence manifest to validate.")]
    pub manifest: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicationArgs {
    #[arg(long, value_name = "HEX", help = "Public evidence bundle identifier.")]
    pub bundle_id: HashArg,
    #[arg(long, value_name = "URI", value_hint = ValueHint::Url, help = "Public URI where the evidence bundle is published.")]
    pub public_uri: String,
    #[arg(
        long,
        value_name = "HEX",
        help = "Address signing the evidence manifest."
    )]
    pub manifest_signer: AddressArg,
    #[arg(
        long,
        value_name = "N",
        help = "Number of manifest signatures included."
    )]
    pub manifest_signature_count: u64,
    #[arg(
        long,
        value_name = "N",
        help = "Number of independent auditor records included."
    )]
    pub independent_auditor_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct AuditorRecordArgs {
    #[arg(long, value_name = "HEX", help = "Public evidence bundle identifier.")]
    pub bundle_id: HashArg,
    #[arg(long, value_name = "URI", value_hint = ValueHint::Url, help = "Public URI where the evidence bundle is published.")]
    pub public_uri: String,
    #[arg(
        long,
        value_name = "HEX",
        help = "Address or identifier of the independent auditor."
    )]
    pub auditor_id: AddressArg,
    #[arg(long, value_name = "URI", value_hint = ValueHint::Url, help = "Public URI for the auditor's review artifact.")]
    pub audit_uri: String,
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp for the audit observation."
    )]
    pub observed_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RunWindowArgs {
    #[arg(long, value_name = "HEX", help = "Public evidence bundle identifier.")]
    pub bundle_id: HashArg,
    #[arg(
        long,
        value_name = "HEX",
        help = "Address signing the evidence manifest."
    )]
    pub manifest_signer: AddressArg,
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp at run-window start."
    )]
    pub started_at: u64,
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp at run-window end."
    )]
    pub ended_at: u64,
    #[arg(
        long,
        value_name = "N",
        help = "Blocks observed during the run window."
    )]
    pub observed_blocks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RunWindowFromFileArgs {
    #[arg(long, value_name = "HEX", help = "Public evidence bundle identifier.")]
    pub bundle_id: HashArg,
    #[arg(
        long,
        value_name = "HEX",
        help = "Address signing the evidence manifest."
    )]
    pub manifest_signer: AddressArg,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath, help = "File containing observed block records.")]
    pub block_observation_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeHeartbeatArgs {
    #[arg(long, help = "Public node role being observed.")]
    pub role: PublicNodeRoleArg,
    #[arg(long, value_name = "HEX", help = "Node account address.")]
    pub address: AddressArg,
    #[arg(long, value_name = "HEX", help = "Operator identifier for the node.")]
    pub operator_id: HashArg,
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "First block height covered by the heartbeat window."
    )]
    pub first_block: u64,
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "Last block height covered by the heartbeat window."
    )]
    pub last_block: u64,
    #[arg(
        long,
        value_name = "N",
        help = "Heartbeat records observed in the window."
    )]
    pub heartbeat_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeHeartbeatFromFileArgs {
    #[arg(long, help = "Public node role being observed.")]
    pub role: PublicNodeRoleArg,
    #[arg(long, value_name = "HEX", help = "Node account address.")]
    pub address: AddressArg,
    #[arg(long, value_name = "HEX", help = "Operator identifier for the node.")]
    pub operator_id: HashArg,
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath, help = "File containing heartbeat records.")]
    pub heartbeat_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct OperatorAttestationArgs {
    #[arg(long, help = "Public node role being attested.")]
    pub role: PublicNodeRoleArg,
    #[arg(long, value_name = "HEX", help = "Node account address.")]
    pub address: AddressArg,
    #[arg(long, value_name = "HEX", help = "Operator identifier for the node.")]
    pub operator_id: HashArg,
    #[arg(long, value_name = "URI", value_hint = ValueHint::Url, help = "Public operator identity URI.")]
    pub identity_uri: String,
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp for the attestation observation."
    )]
    pub observed_at: u64,
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
