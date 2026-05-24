pub use super::local_commands::{
    DataDirArgs, LocalCpuVerifyArgs, MinerCommand, MinerRunArgs, MinerStartArgs, ProposerCommand,
    RoleRuntimeArgs, ServiceBlockArgs, ServiceCommand, ServicePeerAddArgs, ServicePeerCommand,
    ServiceReadinessArgs, ServiceRuntimeArgs, ServiceServeArgs, StakeArgs, TestnetCommand,
    ValidatorCommand, ValidatorRunArgs, ValidatorStartArgs,
};
pub use super::public_evidence_commands::{
    AuditorRecordArgs, EvidenceCommand, EvidenceNetworkCommand, EvidenceNodeCommand,
    EvidenceRecordCommand, EvidenceRunCommand, EvidenceServiceCommand, NetworkObservationArgs,
    NetworkObservationFromServiceLogArgs, NodeHeartbeatArgs, NodeHeartbeatFromFileArgs,
    OperatorAttestationArgs, PublicEvidenceManifestArgs, PublicEvidenceRecordKindArg,
    PublicNodeRoleArg, PublicServiceKindArg, PublicTestnetManifestArgs, PublicationArgs,
    RecordArtifactArgs, RecordArtifactFromFileArgs, RecordArtifactFromRootsArgs, RecordSummaryArgs,
    RecordSummaryFromFileArgs, RecordSummaryFromRootsArgs, RunWindowArgs, RunWindowFromFileArgs,
    ServiceContentArgs, ServiceContentFromBytesArgs, ServiceContentFromFileArgs, ServiceHealthArgs,
    ServiceHealthFromFileArgs,
};
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
    #[command(about = "Register, start, run, or inspect a miner node.")]
    #[command(subcommand)]
    Miner(MinerCommand),
    #[command(about = "Register, start, run, or inspect a validator node.")]
    #[command(subcommand)]
    Validator(ValidatorCommand),
    #[command(about = "Run a proposer service role.")]
    #[command(subcommand)]
    Proposer(ProposerCommand),
    #[command(about = "Manage the local service process and its node store.")]
    #[command(subcommand)]
    Service(ServiceCommand),
    #[command(about = "Seed, verify, and preflight TensorVM testnets.")]
    #[command(subcommand)]
    Testnet(TestnetCommand),
    #[command(about = "Generate or validate public-testnet evidence records.")]
    #[command(subcommand)]
    Evidence(EvidenceCommand),
}
