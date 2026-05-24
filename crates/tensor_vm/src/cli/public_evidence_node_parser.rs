use super::CliCommand;
use super::parser_values::{PublicNodeRoleArg, parse_hash_value};
use crate::types::{Address, Hash};
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct NodeHeartbeatArgs {
    #[arg(long)]
    role: PublicNodeRoleArg,
    #[arg(long, value_parser = parse_hash_value)]
    address: Address,
    #[arg(long, value_parser = parse_hash_value)]
    operator_id: Hash,
    #[arg(long)]
    first_block: u64,
    #[arg(long)]
    last_block: u64,
    #[arg(long)]
    heartbeat_count: u64,
}

impl NodeHeartbeatArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceNodeHeartbeat {
            role: self.role.into(),
            address: self.address,
            operator_id: self.operator_id,
            first_seen_block: self.first_block,
            last_seen_block: self.last_block,
            signed_heartbeat_count: self.heartbeat_count,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct NodeHeartbeatFromFileArgs {
    #[arg(long)]
    role: PublicNodeRoleArg,
    #[arg(long, value_parser = parse_hash_value)]
    address: Address,
    #[arg(long, value_parser = parse_hash_value)]
    operator_id: Hash,
    #[arg(long)]
    heartbeat_file: String,
}

impl NodeHeartbeatFromFileArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceNodeHeartbeatFromFile {
            role: self.role.into(),
            address: self.address,
            operator_id: self.operator_id,
            heartbeat_file: self.heartbeat_file,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct OperatorAttestationArgs {
    #[arg(long)]
    role: PublicNodeRoleArg,
    #[arg(long, value_parser = parse_hash_value)]
    address: Address,
    #[arg(long, value_parser = parse_hash_value)]
    operator_id: Hash,
    #[arg(long)]
    identity_uri: String,
    #[arg(long)]
    observed_at: u64,
}

impl OperatorAttestationArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceOperatorAttestation {
            role: self.role.into(),
            address: self.address,
            operator_id: self.operator_id,
            identity_uri: self.identity_uri,
            observed_at_unix_seconds: self.observed_at,
        }
    }
}
