use super::local_runtime_args::DataDirArgs;
use clap::{Args, Subcommand};

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
    #[command(flatten)]
    pub data_dir: DataDirArgs,
    #[arg(long, help = "Emit the verification report as JSON.")]
    pub json: bool,
}

impl LocalCpuVerifyArgs {
    pub fn data_dir(&self) -> &DataDirArgs {
        &self.data_dir
    }

    pub fn emit_json(&self) -> bool {
        self.json
    }
}
