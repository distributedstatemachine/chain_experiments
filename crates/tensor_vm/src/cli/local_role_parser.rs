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

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct StakeArgs {
    #[arg(long)]
    pub stake: u64,
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
