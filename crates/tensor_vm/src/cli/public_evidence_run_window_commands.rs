use super::public_evidence_bundle_commands::EvidenceBundleIdArgs;
use super::public_evidence_signing_commands::ManifestSignerArgs;
use clap::{Args, Subcommand, ValueHint};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub(crate) enum EvidenceRunCommand {
    #[command(about = "Generate signed run-window evidence.")]
    Window(RunWindowArgs),
    #[command(about = "Generate signed run-window evidence from block observations.")]
    WindowFile(RunWindowFromFileArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RunWindowArgs {
    #[command(flatten)]
    pub(crate) context: RunWindowContextArgs,
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp at run-window start."
    )]
    pub(crate) started_at: u64,
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp at run-window end."
    )]
    pub(crate) ended_at: u64,
    #[arg(
        long,
        value_name = "N",
        help = "Blocks observed during the run window."
    )]
    pub(crate) observed_blocks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RunWindowFromFileArgs {
    #[command(flatten)]
    pub(crate) context: RunWindowContextArgs,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "File containing observed block records."
    )]
    pub(crate) block_observation_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RunWindowContextArgs {
    #[command(flatten)]
    pub(crate) bundle: EvidenceBundleIdArgs,
    #[command(flatten)]
    pub(crate) signer: ManifestSignerArgs,
}
