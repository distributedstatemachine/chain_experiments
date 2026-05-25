use super::public_evidence_block_window_commands::BlockHeightWindowArgs;
use super::public_evidence_observation_commands::ObservationTimestampArgs;
use super::public_evidence_operator_commands::OperatorIdArgs;
use super::value_types::AddressArg;
use crate::testnet::PublicNodeRole;
use clap::{Args, Subcommand, ValueEnum, ValueHint};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub(crate) enum EvidenceNodeCommand {
    #[command(about = "Generate public node heartbeat evidence.")]
    Heartbeat(NodeHeartbeatArgs),
    #[command(about = "Generate public node heartbeat evidence from a file.")]
    HeartbeatFile(NodeHeartbeatFromFileArgs),
    #[command(about = "Generate public operator identity attestation evidence.")]
    OperatorAttestation(OperatorAttestationArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NodeHeartbeatArgs {
    #[command(flatten)]
    pub(crate) node: PublicNodeIdentityArgs,
    #[command(flatten)]
    pub(crate) window: BlockHeightWindowArgs,
    #[arg(
        long,
        value_name = "N",
        help = "Heartbeat records observed in the window."
    )]
    pub(crate) heartbeat_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NodeHeartbeatFromFileArgs {
    #[command(flatten)]
    pub(crate) node: PublicNodeIdentityArgs,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "File containing heartbeat records."
    )]
    pub(crate) heartbeat_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct OperatorAttestationArgs {
    #[command(flatten)]
    pub(crate) node: PublicNodeIdentityArgs,
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public operator identity URI."
    )]
    pub(crate) identity_uri: String,
    #[command(flatten)]
    pub(crate) observation: ObservationTimestampArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct PublicNodeIdentityArgs {
    #[arg(long, help = "Public node role.")]
    pub(crate) role: PublicNodeRoleArg,
    #[arg(long, value_name = "HEX", help = "Node account address.")]
    pub(crate) address: AddressArg,
    #[command(flatten)]
    pub(crate) operator: OperatorIdArgs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub(crate) enum PublicNodeRoleArg {
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
