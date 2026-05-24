use super::local_commands::{
    DEFAULT_DATA_DIR, DataDirArgs, NodeRuntimeArgs, default_p2p_listen_addr,
};
use super::value_types::HashArg;
use clap::{Args, Subcommand, ValueHint};
use libp2p::{Multiaddr, PeerId};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum NodeCommand {
    #[command(about = "Initialize the service node store.")]
    Init(DataDirArgs),
    #[command(about = "Manage libp2p peers.")]
    #[command(subcommand)]
    Peer(NodePeerCommand),
    #[command(about = "Check libp2p and node-store readiness.")]
    Check(NodeCheckArgs),
    #[command(about = "Serve RPC, explorer, faucet, telemetry, and libp2p.")]
    Serve(NodeServeArgs),
    #[command(about = "Show node-store status.")]
    Status(DataDirArgs),
    #[command(about = "Show one persisted block from the node store.")]
    Block(NodeBlockArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum NodePeerCommand {
    #[command(about = "Add a libp2p bootstrap peer to the node store.")]
    Add(NodePeerAddArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodePeerAddArgs {
    #[arg(
        long,
        env = "TVMD_DATA_DIR",
        default_value = DEFAULT_DATA_DIR,
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        help = "Node store directory."
    )]
    pub data_dir: PathBuf,
    #[arg(
        long,
        value_name = "PEER_ID",
        help = "Peer ID to persist as a bootstrap peer."
    )]
    pub peer_id: PeerId,
    #[arg(
        long,
        value_name = "MULTIADDR",
        help = "Reachable multiaddress for the peer."
    )]
    pub address: Multiaddr,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeCheckArgs {
    #[arg(
        long,
        env = "TVMD_P2P_LISTEN",
        default_value_t = default_p2p_listen_addr(),
        value_name = "MULTIADDR",
        help = "libp2p listen multiaddress to validate."
    )]
    pub p2p_listen: Multiaddr,
    #[arg(
        long,
        env = "TVMD_DATA_DIR",
        default_value = DEFAULT_DATA_DIR,
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        help = "Node store directory."
    )]
    pub data_dir: PathBuf,
    #[arg(
        long,
        value_name = "HEX",
        help = "Deterministic 32-byte seed for the libp2p identity."
    )]
    pub identity_seed: Option<HashArg>,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeServeArgs {
    #[command(flatten)]
    pub runtime: NodeRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeBlockArgs {
    #[arg(
        long,
        env = "TVMD_DATA_DIR",
        default_value = DEFAULT_DATA_DIR,
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        help = "Node store directory."
    )]
    pub data_dir: PathBuf,
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "Block height to load from the node store."
    )]
    pub height: u64,
}
