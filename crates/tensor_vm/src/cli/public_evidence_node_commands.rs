use super::public_evidence_block_window_commands::BlockHeightWindowArgs;
use super::public_evidence_observation_commands::ObservationTimestampArgs;
use super::public_evidence_operator_commands::OperatorIdArgs;
use super::value_types::AddressArg;
use crate::testnet::PublicNodeRole;
use crate::types::{Address, Hash};
use clap::{Args, Subcommand, ValueEnum, ValueHint};
use std::path::{Path, PathBuf};

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
    #[command(flatten)]
    node: PublicNodeIdentityArgs,
    #[command(flatten)]
    window: BlockHeightWindowArgs,
    #[arg(
        long,
        value_name = "N",
        help = "Heartbeat records observed in the window."
    )]
    heartbeat_count: u64,
}

impl NodeHeartbeatArgs {
    #[cfg(test)]
    pub(crate) fn new(
        node: PublicNodeIdentityArgs,
        window: BlockHeightWindowArgs,
        heartbeat_count: u64,
    ) -> Self {
        Self {
            node,
            window,
            heartbeat_count,
        }
    }

    pub fn role(&self) -> PublicNodeRole {
        self.node.role()
    }

    pub fn address(&self) -> Address {
        self.node.address()
    }

    pub fn operator_id(&self) -> Hash {
        self.node.operator_id()
    }

    pub fn first_block(&self) -> u64 {
        self.window.first_block()
    }

    pub fn last_block(&self) -> u64 {
        self.window.last_block()
    }

    pub fn heartbeat_count(&self) -> u64 {
        self.heartbeat_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeHeartbeatFromFileArgs {
    #[command(flatten)]
    node: PublicNodeIdentityArgs,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "File containing heartbeat records."
    )]
    heartbeat_file: PathBuf,
}

impl NodeHeartbeatFromFileArgs {
    #[cfg(test)]
    pub(crate) fn new(node: PublicNodeIdentityArgs, heartbeat_file: PathBuf) -> Self {
        Self {
            node,
            heartbeat_file,
        }
    }

    pub fn role(&self) -> PublicNodeRole {
        self.node.role()
    }

    pub fn address(&self) -> Address {
        self.node.address()
    }

    pub fn operator_id(&self) -> Hash {
        self.node.operator_id()
    }

    pub fn heartbeat_file(&self) -> &Path {
        &self.heartbeat_file
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct OperatorAttestationArgs {
    #[command(flatten)]
    node: PublicNodeIdentityArgs,
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public operator identity URI."
    )]
    identity_uri: String,
    #[command(flatten)]
    observation: ObservationTimestampArgs,
}

impl OperatorAttestationArgs {
    #[cfg(test)]
    pub(crate) fn new(
        node: PublicNodeIdentityArgs,
        identity_uri: impl Into<String>,
        observation: ObservationTimestampArgs,
    ) -> Self {
        Self {
            node,
            identity_uri: identity_uri.into(),
            observation,
        }
    }

    pub fn role(&self) -> PublicNodeRole {
        self.node.role()
    }

    pub fn address(&self) -> Address {
        self.node.address()
    }

    pub fn operator_id(&self) -> Hash {
        self.node.operator_id()
    }

    pub fn identity_uri(&self) -> &str {
        &self.identity_uri
    }

    pub fn observed_at(&self) -> u64 {
        self.observation.observed_at()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicNodeIdentityArgs {
    #[arg(long, help = "Public node role.")]
    role: PublicNodeRoleArg,
    #[arg(long, value_name = "HEX", help = "Node account address.")]
    address: AddressArg,
    #[command(flatten)]
    operator: OperatorIdArgs,
}

impl PublicNodeIdentityArgs {
    #[cfg(test)]
    pub(crate) fn new(
        role: PublicNodeRoleArg,
        address: AddressArg,
        operator: OperatorIdArgs,
    ) -> Self {
        Self {
            role,
            address,
            operator,
        }
    }

    pub fn role(&self) -> PublicNodeRole {
        self.role.into()
    }

    pub fn address(&self) -> Address {
        self.address.into_address()
    }

    pub fn operator_id(&self) -> Hash {
        self.operator.id()
    }
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
