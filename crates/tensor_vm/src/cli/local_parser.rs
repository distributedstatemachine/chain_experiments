use super::parser_values::parse_hash_value;
use crate::types::Hash;
use clap::{Args, Subcommand};

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

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum LocalTestnetCommand {
    Seed(DataDirArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum LocalCpuCommand {
    Verify(LocalCpuVerifyArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct StakeArgs {
    #[arg(long)]
    pub stake: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct DataDirArgs {
    #[arg(long)]
    pub data_dir: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct MinerStartArgs {
    #[arg(long)]
    pub wallet: String,
    #[arg(long)]
    pub device: String,
    #[arg(long)]
    pub node: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct MinerRunArgs {
    #[arg(long)]
    pub wallet: String,
    #[arg(long)]
    pub device: String,
    #[arg(long)]
    pub node: String,
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
pub struct ValidatorStartArgs {
    #[arg(long)]
    pub wallet: String,
    #[arg(long)]
    pub node: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ValidatorRunArgs {
    #[arg(long)]
    pub wallet: String,
    #[arg(long)]
    pub node: String,
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

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct LocalCpuVerifyArgs {
    #[arg(long)]
    pub data_dir: String,
    #[arg(long)]
    pub json: bool,
}
