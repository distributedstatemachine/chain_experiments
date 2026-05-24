use super::value_types::{AddressArg, HashArg};
use crate::testnet::PublicNodeRole;
use clap::{Args, Subcommand, ValueEnum, ValueHint};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum EvidenceNodeCommand {
    #[command(about = "Generate public node heartbeat evidence.")]
    Heartbeat(NodeHeartbeatArgs),
    #[command(about = "Generate public node heartbeat evidence from a file.")]
    HeartbeatFile(NodeHeartbeatFromFileArgs),
    #[command(about = "Generate public operator identity attestation evidence.")]
    OperatorAttestation(OperatorAttestationArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeHeartbeatArgs {
    #[arg(long, help = "Public node role being observed.")]
    pub role: PublicNodeRoleArg,
    #[arg(long, value_name = "HEX", help = "Node account address.")]
    pub address: AddressArg,
    #[arg(long, value_name = "HEX", help = "Operator identifier for the node.")]
    pub operator_id: HashArg,
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "First block height covered by the heartbeat window."
    )]
    pub first_block: u64,
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "Last block height covered by the heartbeat window."
    )]
    pub last_block: u64,
    #[arg(
        long,
        value_name = "N",
        help = "Heartbeat records observed in the window."
    )]
    pub heartbeat_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeHeartbeatFromFileArgs {
    #[arg(long, help = "Public node role being observed.")]
    pub role: PublicNodeRoleArg,
    #[arg(long, value_name = "HEX", help = "Node account address.")]
    pub address: AddressArg,
    #[arg(long, value_name = "HEX", help = "Operator identifier for the node.")]
    pub operator_id: HashArg,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "File containing heartbeat records."
    )]
    pub heartbeat_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct OperatorAttestationArgs {
    #[arg(long, help = "Public node role being attested.")]
    pub role: PublicNodeRoleArg,
    #[arg(long, value_name = "HEX", help = "Node account address.")]
    pub address: AddressArg,
    #[arg(long, value_name = "HEX", help = "Operator identifier for the node.")]
    pub operator_id: HashArg,
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public operator identity URI."
    )]
    pub identity_uri: String,
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp for the attestation observation."
    )]
    pub observed_at: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum PublicNodeRoleArg {
    Miner,
    Validator,
}

impl From<PublicNodeRoleArg> for PublicNodeRole {
    fn from(role: PublicNodeRoleArg) -> Self {
        match role {
            PublicNodeRoleArg::Miner => Self::Miner,
            PublicNodeRoleArg::Validator => Self::Validator,
        }
    }
}
