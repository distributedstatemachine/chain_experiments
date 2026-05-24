use super::CliCommand;
use super::parser_values::parse_hash_value;
use super::public_evidence_parser::PublicEvidenceCommand;
use crate::types::Hash;
use clap::{Args, Parser, Subcommand};

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
#[command(
    name = "tvmd",
    version,
    about = "Run TensorVM nodes and produce public-testnet evidence."
)]
pub struct Cli {
    #[command(subcommand)]
    command: TopLevelCommand,
}

impl Cli {
    pub fn into_command(self) -> CliCommand {
        self.command.into_command()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
enum TopLevelCommand {
    Miner {
        #[command(subcommand)]
        command: MinerCommand,
    },
    Validator {
        #[command(subcommand)]
        command: ValidatorCommand,
    },
    Proposer {
        #[command(subcommand)]
        command: ProposerCommand,
    },
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
    LocalTestnet {
        #[command(subcommand)]
        command: LocalTestnetCommand,
    },
    LocalCpu {
        #[command(subcommand)]
        command: LocalCpuCommand,
    },
    PublicEvidence {
        #[command(subcommand)]
        command: PublicEvidenceCommand,
    },
    PublicTestnet {
        #[command(subcommand)]
        command: PublicTestnetCommand,
    },
}

impl TopLevelCommand {
    fn into_command(self) -> CliCommand {
        match self {
            TopLevelCommand::Miner { command } => command.into_command(),
            TopLevelCommand::Validator { command } => command.into_command(),
            TopLevelCommand::Proposer { command } => command.into_command(),
            TopLevelCommand::Service { command } => command.into_command(),
            TopLevelCommand::LocalTestnet { command } => command.into_command(),
            TopLevelCommand::LocalCpu { command } => command.into_command(),
            TopLevelCommand::PublicEvidence { command } => command.into_command(),
            TopLevelCommand::PublicTestnet { command } => command.into_command(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
enum MinerCommand {
    Register(StakeArgs),
    Start(MinerStartArgs),
    Run(MinerRunArgs),
    Status,
}

impl MinerCommand {
    fn into_command(self) -> CliCommand {
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
enum ValidatorCommand {
    Register(StakeArgs),
    Start(ValidatorStartArgs),
    Run(ValidatorRunArgs),
    Status,
}

impl ValidatorCommand {
    fn into_command(self) -> CliCommand {
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
enum ProposerCommand {
    Run(ValidatorRunArgs),
}

impl ProposerCommand {
    fn into_command(self) -> CliCommand {
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
enum ServiceCommand {
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
    fn into_command(self) -> CliCommand {
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
enum ServicePeerCommand {
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
enum LocalTestnetCommand {
    Seed(DataDirArgs),
}

impl LocalTestnetCommand {
    fn into_command(self) -> CliCommand {
        match self {
            LocalTestnetCommand::Seed(args) => CliCommand::LocalTestnetSeed {
                data_dir: args.data_dir,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
enum LocalCpuCommand {
    Verify(LocalCpuVerifyArgs),
}

impl LocalCpuCommand {
    fn into_command(self) -> CliCommand {
        match self {
            LocalCpuCommand::Verify(args) => CliCommand::LocalCpuVerify {
                data_dir: args.data_dir,
                json: args.json,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
enum PublicTestnetCommand {
    Preflight(ManifestArgs),
}

impl PublicTestnetCommand {
    fn into_command(self) -> CliCommand {
        match self {
            PublicTestnetCommand::Preflight(args) => CliCommand::PublicTestnetPreflight {
                manifest: args.manifest,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct StakeArgs {
    #[arg(long)]
    stake: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct DataDirArgs {
    #[arg(long)]
    data_dir: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct ManifestArgs {
    #[arg(long)]
    manifest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct MinerStartArgs {
    #[arg(long)]
    wallet: String,
    #[arg(long)]
    device: String,
    #[arg(long)]
    node: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct MinerRunArgs {
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
struct ValidatorStartArgs {
    #[arg(long)]
    wallet: String,
    #[arg(long)]
    node: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct ValidatorRunArgs {
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
struct ServicePeerAddArgs {
    #[arg(long)]
    data_dir: String,
    #[arg(long)]
    peer_id: String,
    #[arg(long)]
    address: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct ServiceReadinessArgs {
    #[arg(long)]
    p2p_listen: String,
    #[arg(long)]
    data_dir: String,
    #[arg(long, value_parser = parse_hash_value)]
    identity_seed: Option<Hash>,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct ServiceServeArgs {
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
struct ServiceBlockArgs {
    #[arg(long)]
    data_dir: String,
    #[arg(long)]
    height: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
struct LocalCpuVerifyArgs {
    #[arg(long)]
    data_dir: String,
    #[arg(long)]
    json: bool,
}
