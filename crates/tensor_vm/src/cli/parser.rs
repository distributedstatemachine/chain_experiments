use super::CliCommand;
use clap::Parser;

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
#[command(
    name = "tvmd",
    version,
    about = "Run TensorVM nodes and produce public-testnet evidence.",
    propagate_version = true,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: CliCommand,
}
