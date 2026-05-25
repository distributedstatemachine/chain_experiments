use super::local_commands::{
    LocalnetCommand, MinerCommand, NodeCommand, ProposerCommand, ValidatorCommand,
};
use super::public_evidence_commands::PublicCommand;
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
    command: TvmdCommand,
}

impl TvmdCli {
    pub(crate) fn tvmd_command(&self) -> &TvmdCommand {
        &self.command
    }

    #[cfg(test)]
    pub(crate) fn into_command(self) -> TvmdCommand {
        self.command
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub(crate) enum TvmdCommand {
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
