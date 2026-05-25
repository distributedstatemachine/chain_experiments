use super::local_runtime_args::DataDirArgs;
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub(crate) enum LocalnetCommand {
    #[command(about = "Seed local CPU testnet data.")]
    Seed(DataDirArgs),
    #[command(about = "Verify local CPU testnet state.")]
    Verify(LocalCpuVerifyArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct LocalCpuVerifyArgs {
    #[command(flatten)]
    pub(crate) data_dir: DataDirArgs,
    #[arg(long, help = "Emit the verification report as JSON.")]
    pub(crate) json: bool,
}
