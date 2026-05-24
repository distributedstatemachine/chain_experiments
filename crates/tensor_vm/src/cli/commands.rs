pub use super::local_commands::{
    DataDirArgs, LocalCpuVerifyArgs, LocalnetCommand, MinerCheckArgs, MinerCommand, MinerRunArgs,
    NodeBlockArgs, NodeCheckArgs, NodeCommand, NodePeerAddArgs, NodePeerCommand, NodeRuntimeArgs,
    NodeServeArgs, ProposerCommand, RoleCommand, RoleRuntimeArgs, StakeArgs, ValidatorCheckArgs,
    ValidatorCommand, ValidatorRunArgs,
};
pub use super::public_evidence_commands::{
    AuditorRecordArgs, EvidenceCommand, EvidenceNetworkCommand, EvidenceNodeCommand,
    EvidenceRecordCommand, EvidenceRunCommand, EvidenceServiceCommand, NetworkObservationArgs,
    NetworkObservationFromServiceLogArgs, NodeHeartbeatArgs, NodeHeartbeatFromFileArgs,
    OperatorAttestationArgs, PublicCommand, PublicEvidenceManifestArgs,
    PublicEvidenceRecordKindArg, PublicNodeRoleArg, PublicServiceKindArg,
    PublicTestnetManifestArgs, PublicationArgs, RecordArtifactArgs, RecordArtifactFromFileArgs,
    RecordArtifactFromRootsArgs, RecordSummaryArgs, RecordSummaryFromFileArgs,
    RecordSummaryFromRootsArgs, RunWindowArgs, RunWindowFromFileArgs, ServiceContentArgs,
    ServiceContentFromBytesArgs, ServiceContentFromFileArgs, ServiceHealthArgs,
    ServiceHealthFromFileArgs,
};
pub use super::value_types::{AddressArg, HashArg, HexBytesArg};
use clap::{Parser, Subcommand};

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
    #[command(about = "Manage a TensorVM node store, RPC service, and libp2p peers.")]
    #[command(subcommand)]
    Node(NodeCommand),
    #[command(about = "Register, check, run, or inspect miner, validator, and proposer roles.")]
    #[command(subcommand)]
    Role(RoleCommand),
    #[command(about = "Seed and verify a local TensorVM testnet.")]
    #[command(subcommand)]
    Localnet(LocalnetCommand),
    #[command(about = "Validate public-testnet preflight and evidence artifacts.")]
    #[command(subcommand)]
    Public(PublicCommand),
}
