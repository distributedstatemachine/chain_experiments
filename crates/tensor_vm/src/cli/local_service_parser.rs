use super::local_parser::DataDirArgs;
use super::parser_values::parse_hash_value;
use crate::types::Hash;
use clap::{Args, Subcommand};

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
    #[arg(long)]
    pub data_dir: String,
    #[arg(long)]
    pub peer_id: String,
    #[arg(long)]
    pub address: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceReadinessArgs {
    #[arg(long)]
    pub p2p_listen: String,
    #[arg(long)]
    pub data_dir: String,
    #[arg(long, value_parser = parse_hash_value)]
    pub identity_seed: Option<Hash>,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceServeArgs {
    #[arg(long)]
    pub listen: String,
    #[arg(long)]
    pub p2p_listen: String,
    #[arg(long)]
    pub data_dir: String,
    #[arg(long, value_parser = parse_hash_value)]
    pub identity_seed: Option<Hash>,
    #[arg(long)]
    pub auth_token: String,
    #[arg(long)]
    pub max_requests: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ServiceBlockArgs {
    #[arg(long)]
    pub data_dir: String,
    #[arg(long)]
    pub height: u64,
}
