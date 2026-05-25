pub use super::public_evidence_network_commands::{
    EvidenceNetworkCommand, NetworkObservationArgs, NetworkObservationFromServiceLogArgs,
};
pub use super::public_evidence_node_commands::{
    EvidenceNodeCommand, NodeHeartbeatArgs, NodeHeartbeatFromFileArgs, OperatorAttestationArgs,
    PublicNodeIdentityArgs, PublicNodeRoleArg,
};
pub use super::public_evidence_publication_commands::{AuditorRecordArgs, PublicationArgs};
pub use super::public_evidence_record_artifact_commands::{
    RecordArtifactArgs, RecordArtifactFromFileArgs, RecordArtifactFromRootsArgs,
};
pub use super::public_evidence_record_commands::{
    EvidenceRecordCommand, PublicEvidenceRecordContextArgs, PublicEvidenceRecordKindArg,
    RecordSummaryArgs, RecordSummaryFromFileArgs, RecordSummaryFromRootsArgs,
};
pub use super::public_evidence_run_window_commands::{
    EvidenceRunCommand, RunWindowArgs, RunWindowFromFileArgs,
};
pub use super::public_evidence_service_commands::{
    EvidenceServiceCommand, PublicServiceEndpointArgs, PublicServiceKindArg, ServiceContentArgs,
    ServiceContentFromBytesArgs, ServiceContentFromFileArgs, ServiceContentTargetArgs,
    ServiceHealthArgs, ServiceHealthFromFileArgs,
};
use clap::{Args, Subcommand, ValueHint};
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
