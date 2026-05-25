use super::public_evidence_bundle_commands::EvidenceBundleIdArgs;
use super::public_evidence_signing_commands::ManifestSignerArgs;
use crate::types::{Address, Hash};
use clap::{Args, Subcommand, ValueHint};
use std::path::{Path, PathBuf};

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

impl RunWindowArgs {
    pub fn bundle_id(&self) -> Hash {
        self.context.bundle_id()
    }

    pub fn manifest_signer(&self) -> Address {
        self.context.manifest_signer()
    }

    pub fn started_at(&self) -> u64 {
        self.started_at
    }

    pub fn ended_at(&self) -> u64 {
        self.ended_at
    }

    pub fn observed_blocks(&self) -> u64 {
        self.observed_blocks
    }
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

impl RunWindowFromFileArgs {
    pub fn bundle_id(&self) -> Hash {
        self.context.bundle_id()
    }

    pub fn manifest_signer(&self) -> Address {
        self.context.manifest_signer()
    }

    pub fn block_observation_file(&self) -> &Path {
        &self.block_observation_file
    }
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
