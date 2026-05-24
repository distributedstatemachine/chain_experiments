pub use super::command_values::HashList;
pub use super::local_commands::{
    DataDirArgs, LocalCpuCommand, LocalCpuVerifyArgs, LocalTestnetCommand, MinerCommand,
    MinerRunArgs, MinerStartArgs, ProposerCommand, RoleRuntimeArgs, ServiceBlockArgs,
    ServiceCommand, ServicePeerAddArgs, ServicePeerCommand, ServiceReadinessArgs,
    ServiceRuntimeArgs, ServiceServeArgs, StakeArgs, ValidatorCommand, ValidatorRunArgs,
    ValidatorStartArgs,
};
pub use super::public_evidence_commands::{
    AuditorRecordArgs, NetworkObservationArgs, NetworkObservationFromServiceLogArgs,
    NodeHeartbeatArgs, NodeHeartbeatFromFileArgs, OperatorAttestationArgs, PublicEvidenceCommand,
    PublicEvidenceManifestArgs, PublicEvidenceRecordKindArg, PublicNodeRoleArg,
    PublicServiceKindArg, PublicTestnetCommand, PublicTestnetManifestArgs, PublicationArgs,
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
