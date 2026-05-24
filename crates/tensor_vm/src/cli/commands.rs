use super::local_parser::{LocalCpuCommand, LocalTestnetCommand};
use super::local_role_parser::{MinerCommand, ProposerCommand, ValidatorCommand};
use super::local_service_parser::ServiceCommand;
use super::public_evidence_parser::PublicEvidenceCommand;
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum CliCommand {
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

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum PublicTestnetCommand {
    Preflight(ManifestArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ManifestArgs {
    #[arg(long)]
    pub manifest: String,
}
