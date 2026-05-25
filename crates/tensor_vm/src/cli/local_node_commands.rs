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
    data_dir: DataDirArgs,
    #[command(flatten)]
    bootstrap_peer: BootstrapPeerArgs,
}

impl NodePeerAddArgs {
    #[cfg(test)]
    pub(crate) fn new(data_dir: DataDirArgs, bootstrap_peer: BootstrapPeerArgs) -> Self {
        Self {
            data_dir,
            bootstrap_peer,
        }
    }

    pub fn data_dir(&self) -> &DataDirArgs {
        &self.data_dir
    }

    pub fn bootstrap_peer(&self) -> &BootstrapPeerArgs {
        &self.bootstrap_peer
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct BootstrapPeerArgs {
    #[arg(
        long,
        value_name = "PEER_ID",
        help = "Peer ID to persist as a bootstrap peer."
    )]
    peer_id: PeerId,
    #[arg(
        long,
        value_name = "MULTIADDR",
        help = "Reachable multiaddress for the peer."
    )]
    address: Multiaddr,
}

impl BootstrapPeerArgs {
    #[cfg(test)]
    pub(crate) fn new(peer_id: PeerId, address: Multiaddr) -> Self {
        Self { peer_id, address }
    }

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
    p2p_listen: P2pListenArgs,
    #[command(flatten)]
    data_dir: DataDirArgs,
    #[command(flatten)]
    identity_seed: IdentitySeedArgs,
}

impl NodeCheckArgs {
    #[cfg(test)]
    pub(crate) fn new(
        p2p_listen: P2pListenArgs,
        data_dir: DataDirArgs,
        identity_seed: IdentitySeedArgs,
    ) -> Self {
        Self {
            p2p_listen,
            data_dir,
            identity_seed,
        }
    }

    pub fn p2p_listen(&self) -> &P2pListenArgs {
        &self.p2p_listen
    }

    pub fn data_dir(&self) -> &DataDirArgs {
        &self.data_dir
    }

    pub fn identity_seed(&self) -> &IdentitySeedArgs {
        &self.identity_seed
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeServeArgs {
    #[command(flatten)]
    runtime: NodeRuntimeArgs,
}

impl NodeServeArgs {
    #[cfg(test)]
    pub(crate) fn new(runtime: NodeRuntimeArgs) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> &NodeRuntimeArgs {
        &self.runtime
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeBlockArgs {
    #[command(flatten)]
    data_dir: DataDirArgs,
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "Block height to load from the node store."
    )]
    height: u64,
}

impl NodeBlockArgs {
    #[cfg(test)]
    pub(crate) fn new(data_dir: DataDirArgs, height: u64) -> Self {
        Self { data_dir, height }
    }

    pub fn data_dir(&self) -> &DataDirArgs {
        &self.data_dir
    }

    pub fn height(&self) -> u64 {
        self.height
    }
}
