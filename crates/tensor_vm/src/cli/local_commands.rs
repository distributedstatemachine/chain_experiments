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

fn default_p2p_listen_addr() -> Multiaddr {
    DEFAULT_P2P_LISTEN_ADDR
        .parse()
        .expect("default p2p listen address must be a multiaddr")
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum MinerCommand {
    #[command(about = "Check miner registration stake requirements.")]
    Register(StakeArgs),
    #[command(about = "Check miner runtime inputs without running the role.")]
    Check(MinerCheckArgs),
    #[command(about = "Run a miner service.")]
    Run(MinerRunArgs),
    #[command(about = "Show miner readiness metadata.")]
    Status,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum ValidatorCommand {
    #[command(about = "Check validator registration stake requirements.")]
    Register(StakeArgs),
    #[command(about = "Check validator runtime inputs without running the role.")]
    Check(ValidatorCheckArgs),
    #[command(about = "Run a validator service.")]
    Run(ValidatorRunArgs),
    #[command(about = "Show validator readiness metadata.")]
    Status,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum ProposerCommand {
    #[command(about = "Run a proposer service.")]
    Run(ValidatorRunArgs),
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
pub struct StakeArgs {
    #[arg(long, value_name = "TOKENS")]
    pub stake: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct MinerCheckArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub wallet: PathBuf,
    #[arg(long, default_value = "cpu", value_name = "DEVICE")]
    pub device: String,
    #[arg(long, default_value_t = default_p2p_listen_addr(), value_name = "MULTIADDR")]
    pub node: Multiaddr,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct MinerRunArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub wallet: PathBuf,
    #[arg(long, default_value = "cpu", value_name = "DEVICE")]
    pub device: String,
    #[command(flatten)]
    pub runtime: RoleRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ValidatorCheckArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub wallet: PathBuf,
    #[arg(long, default_value_t = default_p2p_listen_addr(), value_name = "MULTIADDR")]
    pub node: Multiaddr,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ValidatorRunArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub wallet: PathBuf,
    #[command(flatten)]
    pub runtime: RoleRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RoleRuntimeArgs {
    #[arg(long, default_value_t = default_p2p_listen_addr(), value_name = "MULTIADDR")]
    pub node: Multiaddr,
    #[command(flatten)]
    pub node_runtime: NodeRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeRuntimeArgs {
    #[arg(long, env = "TVMD_LISTEN", default_value_t = default_listen_addr(), value_name = "ADDR")]
    pub listen: SocketAddr,
    #[arg(long, env = "TVMD_P2P_LISTEN", default_value_t = default_p2p_listen_addr(), value_name = "MULTIADDR")]
    pub p2p_listen: Multiaddr,
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: PathBuf,
    #[arg(long, value_name = "HEX")]
    pub identity_seed: Option<HashArg>,
    #[arg(long, env = "TVMD_AUTH_TOKEN", value_name = "TOKEN")]
    pub auth_token: String,
    #[arg(long, env = "TVMD_MAX_REQUESTS", default_value_t = DEFAULT_MAX_REQUESTS, value_name = "N")]
    pub max_requests: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct DataDirArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct LocalCpuVerifyArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodePeerAddArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: PathBuf,
    #[arg(long, value_name = "PEER_ID")]
    pub peer_id: PeerId,
    #[arg(long, value_name = "MULTIADDR")]
    pub address: Multiaddr,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeCheckArgs {
    #[arg(long, env = "TVMD_P2P_LISTEN", default_value_t = default_p2p_listen_addr(), value_name = "MULTIADDR")]
    pub p2p_listen: Multiaddr,
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: PathBuf,
    #[arg(long, value_name = "HEX")]
    pub identity_seed: Option<HashArg>,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeServeArgs {
    #[command(flatten)]
    pub runtime: NodeRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeBlockArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: PathBuf,
    #[arg(long, value_name = "HEIGHT")]
    pub height: u64,
}
