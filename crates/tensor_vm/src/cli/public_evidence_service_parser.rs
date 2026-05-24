use super::CliCommand;
use super::parser_values::{PublicServiceKindArg, parse_hash_value};
use crate::types::Hash;
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ServiceHealthArgs {
    #[arg(long)]
    kind: PublicServiceKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    endpoint_id: Hash,
    #[arg(long)]
    public_url: String,
    #[arg(long)]
    health_path: String,
    #[arg(long)]
    first_block: u64,
    #[arg(long)]
    last_block: u64,
    #[arg(long)]
    reachable_count: u64,
    #[arg(long)]
    signed_health_check_count: u64,
}

impl ServiceHealthArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceServiceHealth {
            kind: self.kind.into(),
            endpoint_id: self.endpoint_id,
            public_url: self.public_url,
            health_path: self.health_path,
            first_seen_block: self.first_block,
            last_seen_block: self.last_block,
            reachable_observation_count: self.reachable_count,
            signed_health_check_count: self.signed_health_check_count,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ServiceHealthFromFileArgs {
    #[arg(long)]
    kind: PublicServiceKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    endpoint_id: Hash,
    #[arg(long)]
    public_url: String,
    #[arg(long)]
    health_path: String,
    #[arg(long)]
    observation_file: String,
}

impl ServiceHealthFromFileArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceServiceHealthFromFile {
            kind: self.kind.into(),
            endpoint_id: self.endpoint_id,
            public_url: self.public_url,
            health_path: self.health_path,
            observation_file: self.observation_file,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ServiceContentArgs {
    #[arg(long)]
    kind: PublicServiceKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    endpoint_id: Hash,
    #[arg(long)]
    public_url: String,
    #[arg(long)]
    content_path: String,
    #[arg(long, value_parser = parse_hash_value)]
    content_root: Hash,
    #[arg(long)]
    observed_at: u64,
    #[arg(long)]
    min_content_bytes: u64,
}

impl ServiceContentArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceServiceContent {
            kind: self.kind.into(),
            endpoint_id: self.endpoint_id,
            public_url: self.public_url,
            content_path: self.content_path,
            content_root: self.content_root,
            observed_at_unix_seconds: self.observed_at,
            min_content_bytes: self.min_content_bytes,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ServiceContentFromBytesArgs {
    #[arg(long)]
    kind: PublicServiceKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    endpoint_id: Hash,
    #[arg(long)]
    public_url: String,
    #[arg(long)]
    content_path: String,
    #[arg(long)]
    observed_at: u64,
    #[arg(long)]
    content_hex: String,
}

impl ServiceContentFromBytesArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceServiceContentFromBytes {
            kind: self.kind.into(),
            endpoint_id: self.endpoint_id,
            public_url: self.public_url,
            content_path: self.content_path,
            observed_at_unix_seconds: self.observed_at,
            content_hex: self.content_hex,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct ServiceContentFromFileArgs {
    #[arg(long)]
    kind: PublicServiceKindArg,
    #[arg(long, value_parser = parse_hash_value)]
    endpoint_id: Hash,
    #[arg(long)]
    public_url: String,
    #[arg(long)]
    content_path: String,
    #[arg(long)]
    observed_at: u64,
    #[arg(long)]
    content_file: String,
}

impl ServiceContentFromFileArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceServiceContentFromFile {
            kind: self.kind.into(),
            endpoint_id: self.endpoint_id,
            public_url: self.public_url,
            content_path: self.content_path,
            observed_at_unix_seconds: self.observed_at,
            content_file: self.content_file,
        }
    }
}
