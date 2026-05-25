use super::local_runtime_args::{DataDirArgs, IdentitySeedArgs, NodeRuntimeArgs, P2pListenArgs};
use clap::{Args, Subcommand};
use libp2p::{Multiaddr, PeerId};

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
    #[command(flatten)]
    pub data_dir: DataDirArgs,
    #[command(flatten)]
    pub bootstrap_peer: BootstrapPeerArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct BootstrapPeerArgs {
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

impl BootstrapPeerArgs {
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    pub fn address(&self) -> &Multiaddr {
        &self.address
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeCheckArgs {
    #[command(flatten)]
    pub p2p_listen: P2pListenArgs,
    #[command(flatten)]
    pub data_dir: DataDirArgs,
    #[command(flatten)]
    pub identity_seed: IdentitySeedArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeServeArgs {
    #[command(flatten)]
    pub runtime: NodeRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeBlockArgs {
    #[command(flatten)]
    pub data_dir: DataDirArgs,
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "Block height to load from the node store."
    )]
    pub height: u64,
}
