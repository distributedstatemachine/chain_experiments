use super::CliCommand;
use super::local_parser::{
    LocalCpuCommand, LocalTestnetCommand, MinerCommand, ProposerCommand, ServiceCommand,
    ValidatorCommand,
};
use super::public_evidence_parser::PublicEvidenceCommand;
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
struct ManifestArgs {
    #[arg(long)]
    manifest: String,
}
