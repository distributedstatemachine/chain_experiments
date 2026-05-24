pub use super::local_role_commands::{
    MinerCheckArgs, MinerCommand, MinerRunArgs, ProposerCommand, RoleRuntimeArgs, StakeArgs,
    ValidatorCheckArgs, ValidatorCommand, ValidatorRunArgs,
};
use super::value_types::HashArg;
use clap::{Args, Subcommand, ValueHint};
use libp2p::{Multiaddr, PeerId};
use std::net::SocketAddr;
use std::path::PathBuf;

const DEFAULT_DATA_DIR: &str = ".tensorvm";
const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:8545";
const DEFAULT_P2P_LISTEN_ADDR: &str = "/ip4/127.0.0.1/tcp/4001";
const DEFAULT_MAX_REQUESTS: usize = 0;

fn default_listen_addr() -> SocketAddr {
    DEFAULT_LISTEN_ADDR
        .parse()
        .expect("default service listen address must be a socket address")
}

pub(super) fn default_p2p_listen_addr() -> Multiaddr {
    DEFAULT_P2P_LISTEN_ADDR
        .parse()
        .expect("default p2p listen address must be a multiaddr")
}

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

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum LocalnetCommand {
    #[command(about = "Seed local CPU testnet data.")]
    Seed(DataDirArgs),
    #[command(about = "Verify local CPU testnet state.")]
    Verify(LocalCpuVerifyArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeRuntimeArgs {
    #[arg(
        long,
        env = "TVMD_LISTEN",
        default_value_t = default_listen_addr(),
        value_name = "ADDR",
        help = "RPC and service listen address."
    )]
    pub listen: SocketAddr,
    #[arg(
        long,
        env = "TVMD_P2P_LISTEN",
        default_value_t = default_p2p_listen_addr(),
        value_name = "MULTIADDR",
        help = "libp2p listen multiaddress."
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
    #[arg(
        long,
        env = "TVMD_AUTH_TOKEN",
        value_name = "TOKEN",
        hide_env_values = true,
        help = "Bearer token required by local RPC, explorer, faucet, and telemetry endpoints."
    )]
    pub auth_token: String,
    #[arg(
        long,
        env = "TVMD_MAX_REQUESTS",
        default_value_t = DEFAULT_MAX_REQUESTS,
        value_name = "N",
        help = "Maximum RPC requests before the service exits; 0 keeps serving."
    )]
    pub max_requests: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct DataDirArgs {
    #[arg(
        long,
        env = "TVMD_DATA_DIR",
        default_value = DEFAULT_DATA_DIR,
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        help = "Node store directory."
    )]
    pub data_dir: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct LocalCpuVerifyArgs {
    #[arg(
        long,
        env = "TVMD_DATA_DIR",
        default_value = DEFAULT_DATA_DIR,
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        help = "Node store directory."
    )]
    pub data_dir: PathBuf,
    #[arg(long, help = "Emit the verification report as JSON.")]
    pub json: bool,
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
