use super::parser_values::DEFAULT_DATA_DIR;
use clap::{Args, Subcommand, ValueHint};

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum LocalTestnetCommand {
    Seed(DataDirArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum LocalCpuCommand {
    Verify(LocalCpuVerifyArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct DataDirArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_hint = ValueHint::DirPath)]
    pub data_dir: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct LocalCpuVerifyArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_hint = ValueHint::DirPath)]
    pub data_dir: String,
    #[arg(long)]
    pub json: bool,
}
