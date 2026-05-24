use super::parser_values::{
    DEFAULT_DATA_DIR, DEFAULT_LISTEN_ADDR, DEFAULT_MAX_REQUESTS, DEFAULT_P2P_LISTEN_ADDR,
    parse_hash_value, parse_multiaddr_value, parse_socket_addr_value,
};
use crate::types::Hash;
use clap::{Args, Subcommand, ValueHint};

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MinerCommand {
    Register(StakeArgs),
    Start(MinerStartArgs),
    Run(MinerRunArgs),
    Status,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ValidatorCommand {
    Register(StakeArgs),
    Start(ValidatorStartArgs),
    Run(ValidatorRunArgs),
    Status,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ProposerCommand {
    Run(ValidatorRunArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct StakeArgs {
    #[arg(long)]
    pub stake: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct MinerStartArgs {
    #[arg(long)]
    pub wallet: String,
    #[arg(long, default_value = "cpu")]
    pub device: String,
    #[arg(long, default_value = DEFAULT_P2P_LISTEN_ADDR, value_parser = parse_multiaddr_value)]
    pub node: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct MinerRunArgs {
    #[arg(long)]
    pub wallet: String,
    #[arg(long, default_value = "cpu")]
    pub device: String,
    #[command(flatten)]
    pub runtime: RoleRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ValidatorStartArgs {
    #[arg(long)]
    pub wallet: String,
    #[arg(long, default_value = DEFAULT_P2P_LISTEN_ADDR, value_parser = parse_multiaddr_value)]
    pub node: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ValidatorRunArgs {
    #[arg(long)]
    pub wallet: String,
    #[command(flatten)]
    pub runtime: RoleRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct RoleRuntimeArgs {
    #[arg(long, default_value = DEFAULT_P2P_LISTEN_ADDR, value_parser = parse_multiaddr_value)]
    pub node: String,
    #[command(flatten)]
    pub service: ServiceRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceRuntimeArgs {
    #[arg(long, env = "TVMD_LISTEN", default_value = DEFAULT_LISTEN_ADDR, value_parser = parse_socket_addr_value)]
    pub listen: String,
    #[arg(long, env = "TVMD_P2P_LISTEN", default_value = DEFAULT_P2P_LISTEN_ADDR, value_parser = parse_multiaddr_value)]
    pub p2p_listen: String,
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_hint = ValueHint::DirPath)]
    pub data_dir: String,
    #[arg(long, value_parser = parse_hash_value)]
    pub identity_seed: Option<Hash>,
    #[arg(long, env = "TVMD_AUTH_TOKEN")]
    pub auth_token: String,
    #[arg(long, env = "TVMD_MAX_REQUESTS", default_value_t = DEFAULT_MAX_REQUESTS)]
    pub max_requests: usize,
}
