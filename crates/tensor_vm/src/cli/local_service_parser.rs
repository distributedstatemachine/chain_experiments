use super::local_parser::DataDirArgs;
use super::local_role_parser::ServiceRuntimeArgs;
use super::parser_values::{
    DEFAULT_DATA_DIR, DEFAULT_P2P_LISTEN_ADDR, parse_hash_value, parse_multiaddr_value,
};
use crate::types::Hash;
use clap::{Args, Subcommand, ValueHint};

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ServiceCommand {
    Init(DataDirArgs),
    Peer {
        #[command(subcommand)]
        command: ServicePeerCommand,
    },
    Readiness(ServiceReadinessArgs),
    Serve(ServiceServeArgs),
    Status(DataDirArgs),
    Block(ServiceBlockArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ServicePeerCommand {
    Add(ServicePeerAddArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServicePeerAddArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_hint = ValueHint::DirPath)]
    pub data_dir: String,
    #[arg(long)]
    pub peer_id: String,
    #[arg(long, value_parser = parse_multiaddr_value)]
    pub address: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceReadinessArgs {
    #[arg(long, env = "TVMD_P2P_LISTEN", default_value = DEFAULT_P2P_LISTEN_ADDR, value_parser = parse_multiaddr_value)]
    pub p2p_listen: String,
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_hint = ValueHint::DirPath)]
    pub data_dir: String,
    #[arg(long, value_parser = parse_hash_value)]
    pub identity_seed: Option<Hash>,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceServeArgs {
    #[command(flatten)]
    pub runtime: ServiceRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceBlockArgs {
    #[arg(long, env = "TVMD_DATA_DIR", default_value = DEFAULT_DATA_DIR, value_hint = ValueHint::DirPath)]
    pub data_dir: String,
    #[arg(long)]
    pub height: u64,
}
