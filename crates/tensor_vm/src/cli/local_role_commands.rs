use super::local_runtime_args::{NodeRuntimeArgs, default_p2p_listen_addr};
use super::value_types::MinerDeviceArg;
use clap::{Args, Subcommand, ValueHint};
use libp2p::Multiaddr;
use std::path::{Path, PathBuf};

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
    stake: u64,
}

impl StakeArgs {
    pub fn new(stake: u64) -> Self {
        Self { stake }
    }

    pub fn amount(&self) -> u64 {
        self.stake
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct MinerCheckArgs {
    #[command(flatten)]
    wallet: RoleWalletArgs,
    #[arg(
        long,
        default_value_t = MinerDeviceArg::default(),
        value_name = "DEVICE",
        help = "Miner backend: cpu or cuda:N"
    )]
    device: MinerDeviceArg,
    #[command(flatten)]
    node: RoleNodeArgs,
}

impl MinerCheckArgs {
    pub fn new(wallet: RoleWalletArgs, device: MinerDeviceArg, node: RoleNodeArgs) -> Self {
        Self {
            wallet,
            device,
            node,
        }
    }

    pub fn wallet(&self) -> &RoleWalletArgs {
        &self.wallet
    }

    pub fn device(&self) -> &MinerDeviceArg {
        &self.device
    }

    pub fn node(&self) -> &RoleNodeArgs {
        &self.node
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct MinerRunArgs {
    #[command(flatten)]
    wallet: RoleWalletArgs,
    #[arg(
        long,
        default_value_t = MinerDeviceArg::default(),
        value_name = "DEVICE",
        help = "Miner backend: cpu or cuda:N"
    )]
    device: MinerDeviceArg,
    #[command(flatten)]
    runtime: RoleRuntimeArgs,
}

impl MinerRunArgs {
    pub fn new(wallet: RoleWalletArgs, device: MinerDeviceArg, runtime: RoleRuntimeArgs) -> Self {
        Self {
            wallet,
            device,
            runtime,
        }
    }

    pub fn wallet(&self) -> &RoleWalletArgs {
        &self.wallet
    }

    pub fn device(&self) -> &MinerDeviceArg {
        &self.device
    }

    pub fn runtime(&self) -> &RoleRuntimeArgs {
        &self.runtime
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ValidatorCheckArgs {
    #[command(flatten)]
    wallet: RoleWalletArgs,
    #[command(flatten)]
    node: RoleNodeArgs,
}

impl ValidatorCheckArgs {
    pub fn new(wallet: RoleWalletArgs, node: RoleNodeArgs) -> Self {
        Self { wallet, node }
    }

    pub fn wallet(&self) -> &RoleWalletArgs {
        &self.wallet
    }

    pub fn node(&self) -> &RoleNodeArgs {
        &self.node
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ValidatorRunArgs {
    #[command(flatten)]
    wallet: RoleWalletArgs,
    #[command(flatten)]
    runtime: RoleRuntimeArgs,
}

impl ValidatorRunArgs {
    pub fn new(wallet: RoleWalletArgs, runtime: RoleRuntimeArgs) -> Self {
        Self { wallet, runtime }
    }

    pub fn wallet(&self) -> &RoleWalletArgs {
        &self.wallet
    }

    pub fn runtime(&self) -> &RoleRuntimeArgs {
        &self.runtime
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RoleWalletArgs {
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "Path to the role wallet key."
    )]
    wallet: PathBuf,
}

impl RoleWalletArgs {
    pub fn new(wallet: PathBuf) -> Self {
        Self { wallet }
    }

    pub fn path(&self) -> &Path {
        &self.wallet
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RoleNodeArgs {
    #[arg(
        long,
        default_value_t = default_p2p_listen_addr(),
        value_name = "MULTIADDR",
        help = "libp2p address of the TensorVM node to use."
    )]
    node: Multiaddr,
}

impl RoleNodeArgs {
    pub fn new(node: Multiaddr) -> Self {
        Self { node }
    }

    pub fn multiaddr(&self) -> &Multiaddr {
        &self.node
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RoleRuntimeArgs {
    #[command(flatten)]
    node: RoleNodeArgs,
    #[command(flatten)]
    node_runtime: NodeRuntimeArgs,
}

impl RoleRuntimeArgs {
    pub fn new(node: RoleNodeArgs, node_runtime: NodeRuntimeArgs) -> Self {
        Self { node, node_runtime }
    }

    pub fn node(&self) -> &RoleNodeArgs {
        &self.node
    }

    pub fn node_runtime(&self) -> &NodeRuntimeArgs {
        &self.node_runtime
    }
}
