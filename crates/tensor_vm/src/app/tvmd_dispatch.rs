use std::path::Path;

use crate::cli::{
    EvidenceCommand, MinerCommand, ProposerCommand, PublicCommand, RoleRuntimeArgs, TvmdCli,
    TvmdCommand, ValidatorCommand, execute_public_evidence_command,
    validate_public_evidence_manifest, validate_public_testnet_preflight_manifest,
};

use super::operator_checks::{
    check_miner_registration, check_miner_start, check_validator_registration,
    check_validator_start, miner_status, validator_status,
};
use super::operator_validation::{validate_miner_runtime, validate_role_runtime};
use super::tvmd_node_dispatch::{execute_localnet_command, execute_node_command};
use super::tvmd_path::path_arg;
use super::{RoleServiceConfig, run_miner_service, run_proposer_service, run_validator_service};

pub fn run(cli: TvmdCli) -> std::result::Result<String, String> {
    execute_tvmd_command(&cli.command)
}

pub fn execute_tvmd_command(command: &TvmdCommand) -> std::result::Result<String, String> {
    match command {
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Validate(args))) => {
            let contents = std::fs::read_to_string(&args.manifest).map_err(|error| {
                format!(
                    "failed to read evidence manifest {}: {error}",
                    path_arg(&args.manifest)
                )
            })?;
            validate_public_evidence_manifest(&contents).map_err(|error| error.to_string())
        }
        TvmdCommand::Public(PublicCommand::Preflight(args)) => {
            let contents = std::fs::read_to_string(&args.manifest).map_err(|error| {
                format!(
                    "failed to read preflight manifest {}: {error}",
                    path_arg(&args.manifest)
                )
            })?;
            validate_public_testnet_preflight_manifest(&contents).map_err(|error| error.to_string())
        }
        TvmdCommand::Miner(MinerCommand::Register(args)) => check_miner_registration(args.stake),
        TvmdCommand::Miner(MinerCommand::Check(args)) => check_miner_start(
            &path_arg(args.wallet.path()),
            args.device.as_str(),
            &args.node.multiaddr().to_string(),
        ),
        TvmdCommand::Miner(MinerCommand::Run(args)) => {
            let config = RoleServiceDispatchConfig::from_args(args.wallet.path(), &args.runtime);
            validate_miner_runtime(
                &config.wallet,
                args.device.as_str(),
                &config.data_dir,
                &config.auth_token,
            )?;
            run_miner_service(config.as_role_service_config(Some(args.device.as_str())))
        }
        TvmdCommand::Miner(MinerCommand::Status) => Ok(miner_status()),
        TvmdCommand::Validator(ValidatorCommand::Register(args)) => {
            check_validator_registration(args.stake)
        }
        TvmdCommand::Validator(ValidatorCommand::Check(args)) => check_validator_start(
            &path_arg(args.wallet.path()),
            &args.node.multiaddr().to_string(),
        ),
        TvmdCommand::Validator(ValidatorCommand::Run(args)) => {
            let config = RoleServiceDispatchConfig::from_args(args.wallet.path(), &args.runtime);
            validate_role_runtime(&config.wallet, &config.data_dir, &config.auth_token)?;
            run_validator_service(config.as_role_service_config(None))
        }
        TvmdCommand::Validator(ValidatorCommand::Status) => Ok(validator_status()),
        TvmdCommand::Proposer(ProposerCommand::Run(args)) => {
            let config = RoleServiceDispatchConfig::from_args(args.wallet.path(), &args.runtime);
            validate_role_runtime(&config.wallet, &config.data_dir, &config.auth_token)?;
            run_proposer_service(config.as_role_service_config(None))
        }
        TvmdCommand::Node(command) => execute_node_command(command),
        TvmdCommand::Localnet(command) => execute_localnet_command(command),
        TvmdCommand::Public(PublicCommand::Evidence(command)) => {
            execute_public_evidence_command(command).map_err(|error| error.to_string())
        }
    }
}

struct RoleServiceDispatchConfig {
    wallet: String,
    node: String,
    listen: String,
    p2p_listen: String,
    data_dir: String,
    identity_seed: Option<[u8; 32]>,
    auth_token: String,
    max_requests: usize,
}

impl RoleServiceDispatchConfig {
    fn from_args(wallet: &Path, runtime: &RoleRuntimeArgs) -> Self {
        let node_runtime = &runtime.node_runtime;
        Self {
            wallet: path_arg(wallet),
            node: runtime.node.multiaddr().to_string(),
            listen: node_runtime.listen.to_string(),
            p2p_listen: node_runtime.p2p_listen.to_string(),
            data_dir: path_arg(&node_runtime.data_dir),
            identity_seed: node_runtime.identity_seed.map(|seed| seed.into_hash()),
            auth_token: node_runtime.auth_token.clone(),
            max_requests: node_runtime.max_requests,
        }
    }

    fn as_role_service_config<'a>(&'a self, device: Option<&'a str>) -> RoleServiceConfig<'a> {
        RoleServiceConfig {
            wallet: &self.wallet,
            device,
            node: &self.node,
            listen: &self.listen,
            p2p_listen: &self.p2p_listen,
            data_dir: &self.data_dir,
            identity_seed: self.identity_seed,
            auth_token: &self.auth_token,
            max_requests: self.max_requests,
        }
    }
}
