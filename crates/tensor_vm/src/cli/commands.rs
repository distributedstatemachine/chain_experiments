pub use super::local_commands::{
    DataDirArgs, LocalCpuVerifyArgs, LocalnetCommand, MinerCheckArgs, MinerCommand, MinerRunArgs,
    NodeBlockArgs, NodeCheckArgs, NodeCommand, NodePeerAddArgs, NodePeerCommand, NodeRuntimeArgs,
    NodeServeArgs, ProposerCommand, RoleNodeArgs, RoleRuntimeArgs, RoleWalletArgs, StakeArgs,
    ValidatorCheckArgs, ValidatorCommand, ValidatorRunArgs,
};
pub use super::public_evidence_commands::{
    AuditorRecordArgs, EvidenceCommand, EvidenceNetworkCommand, EvidenceNodeCommand,
    EvidenceRecordCommand, EvidenceRunCommand, EvidenceServiceCommand, NetworkObservationArgs,
    NetworkObservationFromServiceLogArgs, NetworkObservationTargetArgs, NodeHeartbeatArgs,
    NodeHeartbeatFromFileArgs, OperatorAttestationArgs, PublicCommand, PublicEvidenceManifestArgs,
    PublicEvidenceRecordContextArgs, PublicEvidenceRecordKindArg, PublicNodeIdentityArgs,
    PublicNodeRoleArg, PublicServiceEndpointArgs, PublicServiceKindArg, PublicTestnetManifestArgs,
    PublicationArgs, RecordArtifactArgs, RecordArtifactFromFileArgs, RecordArtifactFromRootsArgs,
    RecordSummaryArgs, RecordSummaryFromFileArgs, RecordSummaryFromRootsArgs, RunWindowArgs,
    RunWindowFromFileArgs, ServiceContentArgs, ServiceContentFromBytesArgs,
    ServiceContentFromFileArgs, ServiceContentTargetArgs, ServiceHealthArgs,
    ServiceHealthFromFileArgs,
};
pub use super::value_types::{AddressArg, HashArg, HexBytesArg, MinerDeviceArg};
use clap::{Parser, Subcommand};

const TVMD_AFTER_HELP: &str = "Examples:
  tvmd node init --data-dir .tensorvm
  tvmd node serve --auth-token local-dev-token
  tvmd miner run --wallet miner.key --auth-token local-dev-token
  tvmd public preflight docs/tensorvm/public-testnet.preflight";

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
#[command(
    name = "tvmd",
    version,
    about = "Run TensorVM nodes and generate public-testnet evidence.",
    after_help = TVMD_AFTER_HELP,
    propagate_version = true,
    arg_required_else_help = true
)]
pub struct TvmdCli {
    #[command(subcommand)]
    pub command: TvmdCommand,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum TvmdCommand {
    #[command(about = "Manage a TensorVM node store, RPC service, and libp2p peers.")]
    #[command(subcommand)]
    Node(NodeCommand),
    #[command(about = "Register, check, run, or inspect a miner role.")]
    #[command(subcommand)]
    Miner(MinerCommand),
    #[command(about = "Register, check, run, or inspect a validator role.")]
    #[command(subcommand)]
    Validator(ValidatorCommand),
    #[command(about = "Run a proposer role.")]
    #[command(subcommand)]
    Proposer(ProposerCommand),
    #[command(about = "Seed and verify a local TensorVM testnet.")]
    #[command(subcommand)]
    Localnet(LocalnetCommand),
    #[command(about = "Validate public-testnet preflight and evidence artifacts.")]
    #[command(subcommand)]
    Public(PublicCommand),
}
