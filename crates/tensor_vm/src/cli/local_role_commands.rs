use super::local_runtime_args::{NodeRuntimeArgs, default_p2p_listen_addr};
use super::value_types::MinerDeviceArg;
use clap::{Args, Subcommand, ValueHint};
use libp2p::Multiaddr;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub(crate) enum MinerCommand {
    #[command(about = "Check miner registration stake requirements.")]
    Register(StakeArgs),
    #[command(about = "Check miner runtime inputs without running the role.")]
    Check(MinerCheckArgs),
    #[command(about = "Run a miner service.")]
    Run(MinerRunArgs),
    #[command(about = "Show miner readiness metadata.")]
    Status,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub(crate) enum ValidatorCommand {
    #[command(about = "Check validator registration stake requirements.")]
    Register(StakeArgs),
    #[command(about = "Check validator runtime inputs without running the role.")]
    Check(ValidatorCheckArgs),
    #[command(about = "Run a validator service.")]
    Run(ValidatorRunArgs),
    #[command(about = "Show validator readiness metadata.")]
    Status,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub(crate) enum ProposerCommand {
    #[command(about = "Run a proposer service.")]
    Run(ValidatorRunArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct StakeArgs {
    #[arg(
        long,
        value_name = "TOKENS",
        help = "Stake amount to validate for registration."
    )]
    pub(crate) stake: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct MinerCheckArgs {
    #[command(flatten)]
    pub(crate) wallet: RoleWalletArgs,
    #[arg(
        long,
        default_value_t = MinerDeviceArg::default(),
        value_name = "DEVICE",
        help = "Miner backend: cpu or cuda:N"
    )]
    pub(crate) device: MinerDeviceArg,
    #[command(flatten)]
    pub(crate) node: RoleNodeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct MinerRunArgs {
    #[command(flatten)]
    pub(crate) wallet: RoleWalletArgs,
    #[arg(
        long,
        default_value_t = MinerDeviceArg::default(),
        value_name = "DEVICE",
        help = "Miner backend: cpu or cuda:N"
    )]
    pub(crate) device: MinerDeviceArg,
    #[command(flatten)]
    pub(crate) runtime: RoleRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct ValidatorCheckArgs {
    #[command(flatten)]
    pub(crate) wallet: RoleWalletArgs,
    #[command(flatten)]
    pub(crate) node: RoleNodeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct ValidatorRunArgs {
    #[command(flatten)]
    pub(crate) wallet: RoleWalletArgs,
    #[command(flatten)]
    pub(crate) runtime: RoleRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RoleWalletArgs {
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "Path to the role wallet key."
    )]
    pub(crate) wallet: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RoleNodeArgs {
    #[arg(
        long,
        default_value_t = default_p2p_listen_addr(),
        value_name = "MULTIADDR",
        help = "libp2p address of the TensorVM node to use."
    )]
    pub(crate) node: Multiaddr,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RoleRuntimeArgs {
    #[command(flatten)]
    pub(crate) node: RoleNodeArgs,
    #[command(flatten)]
    pub(crate) node_runtime: NodeRuntimeArgs,
}
