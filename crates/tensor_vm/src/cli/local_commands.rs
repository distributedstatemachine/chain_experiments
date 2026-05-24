use super::command_values::{parse_hash_value, parse_multiaddr_value, parse_socket_addr_value};
use crate::types::Hash;
use clap::{Args, Subcommand, ValueHint};

const DEFAULT_DATA_DIR: &str = ".tensorvm";
const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:8545";
const DEFAULT_P2P_LISTEN_ADDR: &str = "/ip4/127.0.0.1/tcp/4001";
const DEFAULT_MAX_REQUESTS: usize = 0;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MinerCommand {
    #[command(about = "Check miner registration stake requirements.")]
    Register(StakeArgs),
    #[command(about = "Check miner startup inputs without running a service.")]
    Start(MinerStartArgs),
    #[command(about = "Run a miner service.")]
    Run(MinerRunArgs),
    #[command(about = "Show miner readiness metadata.")]
    Status,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ValidatorCommand {
    #[command(about = "Check validator registration stake requirements.")]
    Register(StakeArgs),
    #[command(about = "Check validator startup inputs without running a service.")]
    Start(ValidatorStartArgs),
    #[command(about = "Run a validator service.")]
    Run(ValidatorRunArgs),
    #[command(about = "Show validator readiness metadata.")]
    Status,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ProposerCommand {
    #[command(about = "Run a proposer service.")]
    Run(ValidatorRunArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ServiceCommand {
    #[command(about = "Initialize the service node store.")]
    Init(DataDirArgs),
    #[command(about = "Manage libp2p peers.")]
    Peer {
        #[command(subcommand)]
        command: ServicePeerCommand,
    },
    #[command(about = "Check libp2p and node-store readiness.")]
    Readiness(ServiceReadinessArgs),
    #[command(about = "Serve RPC, explorer, faucet, telemetry, and libp2p.")]
    Serve(ServiceServeArgs),
    #[command(about = "Show node-store status.")]
    Status(DataDirArgs),
    #[command(about = "Show one persisted block from the node store.")]
    Block(ServiceBlockArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ServicePeerCommand {
    #[command(about = "Add a libp2p bootstrap peer to the node store.")]
    Add(ServicePeerAddArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum LocalTestnetCommand {
    #[command(about = "Seed local CPU testnet data.")]
    Seed(DataDirArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum LocalCpuCommand {
    #[command(about = "Verify local CPU testnet state.")]
    Verify(LocalCpuVerifyArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct StakeArgs {
    #[arg(long, value_name = "TOKENS")]
    pub stake: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct MinerStartArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub wallet: String,
    #[arg(long, default_value = "cpu", value_name = "DEVICE")]
    pub device: String,
    #[arg(long, default_value = DEFAULT_P2P_LISTEN_ADDR, value_name = "MULTIADDR", value_parser = parse_multiaddr_value)]
    pub node: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct MinerRunArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub wallet: String,
    #[arg(long, default_value = "cpu", value_name = "DEVICE")]
    pub device: String,
    #[command(flatten)]
    pub runtime: RoleRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ValidatorStartArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub wallet: String,
    #[arg(long, default_value = DEFAULT_P2P_LISTEN_ADDR, value_name = "MULTIADDR", value_parser = parse_multiaddr_value)]
    pub node: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ValidatorRunArgs {
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub wallet: String,
    #[command(flatten)]
    pub runtime: RoleRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RoleRuntimeArgs {
    #[arg(long, default_value = DEFAULT_P2P_LISTEN_ADDR, value_name = "MULTIADDR", value_parser = parse_multiaddr_value)]
    pub node: String,
    #[command(flatten)]
    pub service: ServiceRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceRuntimeArgs {
    #[arg(long, env = "TVMD_LISTEN", default_value = DEFAULT_LISTEN_ADDR, value_name = "ADDR", value_parser = parse_socket_addr_value)]
    pub listen: String,
    #[arg(long, env = "TVMD_P2P_LISTEN", default_value = DEFAULT_P2P_LISTEN_ADDR, value_name = "MULTIADDR", value_parser = parse_multiaddr_value)]
    pub p2p_listen: String,
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: String,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub identity_seed: Option<Hash>,
    #[arg(long, env = "TVMD_AUTH_TOKEN", value_name = "TOKEN")]
    pub auth_token: String,
    #[arg(long, env = "TVMD_MAX_REQUESTS", default_value_t = DEFAULT_MAX_REQUESTS, value_name = "N")]
    pub max_requests: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct DataDirArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct LocalCpuVerifyArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServicePeerAddArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: String,
    #[arg(long, value_name = "PEER_ID")]
    pub peer_id: String,
    #[arg(long, value_name = "MULTIADDR", value_parser = parse_multiaddr_value)]
    pub address: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceReadinessArgs {
    #[arg(long, env = "TVMD_P2P_LISTEN", default_value = DEFAULT_P2P_LISTEN_ADDR, value_name = "MULTIADDR", value_parser = parse_multiaddr_value)]
    pub p2p_listen: String,
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: String,
    #[arg(long, value_name = "HEX", value_parser = parse_hash_value)]
    pub identity_seed: Option<Hash>,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceServeArgs {
    #[command(flatten)]
    pub runtime: ServiceRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceBlockArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub data_dir: String,
    #[arg(long, value_name = "HEIGHT")]
    pub height: u64,
}
