use super::public_evidence_bundle_commands::EvidenceBundleIdArgs;
use super::public_evidence_signing_commands::ManifestSignerArgs;
use crate::types::{Address, Hash};
use clap::{Args, Subcommand, ValueHint};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum EvidenceRunCommand {
    #[command(about = "Generate signed run-window evidence.")]
    Window(RunWindowArgs),
    #[command(about = "Generate signed run-window evidence from block observations.")]
    WindowFile(RunWindowFromFileArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RunWindowArgs {
    #[command(flatten)]
    pub context: RunWindowContextArgs,
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp at run-window start."
    )]
    pub started_at: u64,
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp at run-window end."
    )]
    pub ended_at: u64,
    #[arg(
        long,
        value_name = "N",
        help = "Blocks observed during the run window."
    )]
    pub observed_blocks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RunWindowFromFileArgs {
    #[command(flatten)]
    pub context: RunWindowContextArgs,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "File containing observed block records."
    )]
    pub block_observation_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RunWindowContextArgs {
    #[command(flatten)]
    pub bundle: EvidenceBundleIdArgs,
    #[command(flatten)]
    pub signer: ManifestSignerArgs,
}

impl RunWindowContextArgs {
    pub fn bundle_id(&self) -> Hash {
        self.bundle.id()
    }

    pub fn manifest_signer(&self) -> Address {
        self.signer.signer()
    }
}
