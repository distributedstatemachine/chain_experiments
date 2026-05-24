use super::local_commands::{DEFAULT_DATA_DIR, DataDirArgs};
use clap::{Args, Subcommand, ValueHint};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum LocalnetCommand {
    #[command(about = "Seed local CPU testnet data.")]
    Seed(DataDirArgs),
    #[command(about = "Verify local CPU testnet state.")]
    Verify(LocalCpuVerifyArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct LocalCpuVerifyArgs {
    #[arg(
        long,
        env = "TVMD_DATA_DIR",
        default_value = DEFAULT_DATA_DIR,
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        help = "Node store directory."
    )]
    pub data_dir: PathBuf,
    #[arg(long, help = "Emit the verification report as JSON.")]
    pub json: bool,
}
