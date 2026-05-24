use super::CliCommand;
use super::parser_values::parse_hash_value;
use crate::types::{Address, Hash};
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct RunWindowArgs {
    #[arg(long, value_parser = parse_hash_value)]
    bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    manifest_signer: Address,
    #[arg(long)]
    started_at: u64,
    #[arg(long)]
    ended_at: u64,
    #[arg(long)]
    observed_blocks: u64,
}

impl RunWindowArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceRunWindow {
            bundle_id: self.bundle_id,
            manifest_signer: self.manifest_signer,
            run_started_at_unix_seconds: self.started_at,
            run_ended_at_unix_seconds: self.ended_at,
            observed_blocks: self.observed_blocks,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct RunWindowFromFileArgs {
    #[arg(long, value_parser = parse_hash_value)]
    bundle_id: Hash,
    #[arg(long, value_parser = parse_hash_value)]
    manifest_signer: Address,
    #[arg(long)]
    block_observation_file: String,
}

impl RunWindowFromFileArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceRunWindowFromFile {
            bundle_id: self.bundle_id,
            manifest_signer: self.manifest_signer,
            block_observation_file: self.block_observation_file,
        }
    }
}
