use super::CliCommand;
use super::parser_values::parse_hash_value;
use crate::types::Hash;
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
pub(super) enum MinerCommand {
    Register(StakeArgs),
    Start(MinerStartArgs),
    Run(MinerRunArgs),
    Status,
}

impl MinerCommand {
    pub(super) fn into_command(self) -> CliCommand {
        match self {
            MinerCommand::Register(args) => CliCommand::MinerRegister { stake: args.stake },
            MinerCommand::Start(args) => CliCommand::MinerStart {
                wallet: args.wallet,
                device: args.device,
                node: args.node,
            },
            MinerCommand::Run(args) => CliCommand::MinerRun {
                wallet: args.wallet,
                device: args.device,
                node: args.node,
                listen: args.listen,
                p2p_listen: args.p2p_listen,
                data_dir: args.data_dir,
                identity_seed: args.identity_seed,
                auth_token: args.auth_token,
                max_requests: args.max_requests,
            },
            MinerCommand::Status => CliCommand::MinerStatus,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
pub(super) enum ValidatorCommand {
    Register(StakeArgs),
    Start(ValidatorStartArgs),
    Run(ValidatorRunArgs),
    Status,
}

impl ValidatorCommand {
    pub(super) fn into_command(self) -> CliCommand {
        match self {
            ValidatorCommand::Register(args) => CliCommand::ValidatorRegister { stake: args.stake },
            ValidatorCommand::Start(args) => CliCommand::ValidatorStart {
                wallet: args.wallet,
                node: args.node,
            },
            ValidatorCommand::Run(args) => CliCommand::ValidatorRun {
                wallet: args.wallet,
                node: args.node,
                listen: args.listen,
                p2p_listen: args.p2p_listen,
                data_dir: args.data_dir,
                identity_seed: args.identity_seed,
                auth_token: args.auth_token,
                max_requests: args.max_requests,
            },
            ValidatorCommand::Status => CliCommand::ValidatorStatus,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
pub(super) enum ProposerCommand {
    Run(ValidatorRunArgs),
}

impl ProposerCommand {
    pub(super) fn into_command(self) -> CliCommand {
        match self {
            ProposerCommand::Run(args) => CliCommand::ProposerRun {
                wallet: args.wallet,
                node: args.node,
                listen: args.listen,
                p2p_listen: args.p2p_listen,
                data_dir: args.data_dir,
                identity_seed: args.identity_seed,
                auth_token: args.auth_token,
                max_requests: args.max_requests,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
pub(super) enum ServiceCommand {
    Init(DataDirArgs),
    Peer {
        #[command(subcommand)]
        command: ServicePeerCommand,
    },
    Readiness(ServiceReadinessArgs),
    Serve(ServiceServeArgs),
    Status(DataDirArgs),
    Block(ServiceBlockArgs),
}

impl ServiceCommand {
    pub(super) fn into_command(self) -> CliCommand {
        match self {
            ServiceCommand::Init(args) => CliCommand::ServiceInit {
                data_dir: args.data_dir,
            },
            ServiceCommand::Peer { command } => command.into_command(),
            ServiceCommand::Readiness(args) => CliCommand::ServiceReadiness {
                p2p_listen: args.p2p_listen,
                data_dir: args.data_dir,
                identity_seed: args.identity_seed,
            },
            ServiceCommand::Serve(args) => CliCommand::ServiceServe {
                listen: args.listen,
                p2p_listen: args.p2p_listen,
                data_dir: args.data_dir,
                identity_seed: args.identity_seed,
                auth_token: args.auth_token,
                max_requests: args.max_requests,
            },
            ServiceCommand::Status(args) => CliCommand::ServiceStatus {
                data_dir: args.data_dir,
            },
            ServiceCommand::Block(args) => CliCommand::ServiceBlock {
                data_dir: args.data_dir,
                height: args.height,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
pub(super) enum ServicePeerCommand {
    Add(ServicePeerAddArgs),
}

impl ServicePeerCommand {
    fn into_command(self) -> CliCommand {
        match self {
            ServicePeerCommand::Add(args) => CliCommand::ServicePeerAdd {
                data_dir: args.data_dir,
                peer_id: args.peer_id,
                address: args.address,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
pub(super) enum LocalTestnetCommand {
    Seed(DataDirArgs),
}

impl LocalTestnetCommand {
    pub(super) fn into_command(self) -> CliCommand {
        match self {
            LocalTestnetCommand::Seed(args) => CliCommand::LocalTestnetSeed {
                data_dir: args.data_dir,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
pub(super) enum LocalCpuCommand {
    Verify(LocalCpuVerifyArgs),
}

impl LocalCpuCommand {
    pub(super) fn into_command(self) -> CliCommand {
        match self {
            LocalCpuCommand::Verify(args) => CliCommand::LocalCpuVerify {
                data_dir: args.data_dir,
                json: args.json,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct StakeArgs {
    #[arg(long)]
    stake: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct DataDirArgs {
    #[arg(long)]
    data_dir: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct MinerStartArgs {
    #[arg(long)]
    wallet: String,
    #[arg(long)]
    device: String,
    #[arg(long)]
    node: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct MinerRunArgs {
    #[arg(long)]
    wallet: String,
    #[arg(long)]
    device: String,
    #[arg(long)]
    node: String,
    #[arg(long)]
    listen: String,
    #[arg(long)]
    p2p_listen: String,
    #[arg(long)]
    data_dir: String,
    #[arg(long, value_parser = parse_hash_value)]
    identity_seed: Option<Hash>,
    #[arg(long)]
    auth_token: String,
    #[arg(long)]
    max_requests: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ValidatorStartArgs {
    #[arg(long)]
    wallet: String,
    #[arg(long)]
    node: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ValidatorRunArgs {
    #[arg(long)]
    wallet: String,
    #[arg(long)]
    node: String,
    #[arg(long)]
    listen: String,
    #[arg(long)]
    p2p_listen: String,
    #[arg(long)]
    data_dir: String,
    #[arg(long, value_parser = parse_hash_value)]
    identity_seed: Option<Hash>,
    #[arg(long)]
    auth_token: String,
    #[arg(long)]
    max_requests: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ServicePeerAddArgs {
    #[arg(long)]
    data_dir: String,
    #[arg(long)]
    peer_id: String,
    #[arg(long)]
    address: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ServiceReadinessArgs {
    #[arg(long)]
    p2p_listen: String,
    #[arg(long)]
    data_dir: String,
    #[arg(long, value_parser = parse_hash_value)]
    identity_seed: Option<Hash>,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ServiceServeArgs {
    #[arg(long)]
    listen: String,
    #[arg(long)]
    p2p_listen: String,
    #[arg(long)]
    data_dir: String,
    #[arg(long, value_parser = parse_hash_value)]
    identity_seed: Option<Hash>,
    #[arg(long)]
    auth_token: String,
    #[arg(long)]
    max_requests: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ServiceBlockArgs {
    #[arg(long)]
    data_dir: String,
    #[arg(long)]
    height: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct LocalCpuVerifyArgs {
    #[arg(long)]
    data_dir: String,
    #[arg(long)]
    json: bool,
}
