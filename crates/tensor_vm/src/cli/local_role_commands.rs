use super::local_runtime_args::{NodeRuntimeArgs, default_p2p_listen_addr};
use super::value_types::MinerDeviceArg;
use clap::{Args, Subcommand, ValueHint};
use libp2p::Multiaddr;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum MinerCommand {
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
pub enum ValidatorCommand {
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
pub enum ProposerCommand {
    #[command(about = "Run a proposer service.")]
    Run(ValidatorRunArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct StakeArgs {
    #[arg(
        long,
        value_name = "TOKENS",
        help = "Stake amount to validate for registration."
    )]
    pub stake: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct MinerCheckArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath, help = "Path to the miner wallet key.")]
    pub wallet: PathBuf,
    #[arg(
        long,
        default_value_t = MinerDeviceArg::default(),
        value_name = "DEVICE",
        help = "Miner backend: cpu or cuda:N"
    )]
    pub device: MinerDeviceArg,
    #[arg(
        long,
        default_value_t = default_p2p_listen_addr(),
        value_name = "MULTIADDR",
        help = "libp2p address of the TensorVM node to use."
    )]
    pub node: Multiaddr,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct MinerRunArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath, help = "Path to the miner wallet key.")]
    pub wallet: PathBuf,
    #[arg(
        long,
        default_value_t = MinerDeviceArg::default(),
        value_name = "DEVICE",
        help = "Miner backend: cpu or cuda:N"
    )]
    pub device: MinerDeviceArg,
    #[command(flatten)]
    pub runtime: RoleRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ValidatorCheckArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath, help = "Path to the validator wallet key.")]
    pub wallet: PathBuf,
    #[arg(
        long,
        default_value_t = default_p2p_listen_addr(),
        value_name = "MULTIADDR",
        help = "libp2p address of the TensorVM node to use."
    )]
    pub node: Multiaddr,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ValidatorRunArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath, help = "Path to the wallet key for the role.")]
    pub wallet: PathBuf,
    #[command(flatten)]
    pub runtime: RoleRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RoleRuntimeArgs {
    #[arg(
        long,
        default_value_t = default_p2p_listen_addr(),
        value_name = "MULTIADDR",
        help = "libp2p address of the TensorVM node to use."
    )]
    pub node: Multiaddr,
    #[command(flatten)]
    pub node_runtime: NodeRuntimeArgs,
}
