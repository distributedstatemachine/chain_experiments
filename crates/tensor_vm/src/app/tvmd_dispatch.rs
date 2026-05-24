use std::path::Path;

use crate::cli::{
    EvidenceCommand, LocalnetCommand, MinerCommand, NodeCommand, NodePeerCommand, ProposerCommand,
    PublicCommand, RoleCommand, TvmdCli, TvmdCommand, ValidatorCommand,
    execute_public_evidence_command, validate_public_evidence_manifest,
    validate_public_testnet_preflight_manifest,
};

use super::operator_checks::{
    check_miner_registration, check_miner_start, check_validator_registration,
    check_validator_start, miner_status, validate_data_dir, validate_miner_runtime,
    validate_role_runtime, validate_service_runtime, validator_status,
};
use super::{
    RoleServiceConfig, add_service_peer, check_service_readiness, init_service_store,
    run_miner_service, run_proposer_service, run_validator_service, seed_local_testnet,
    serve_service, service_block_status, service_status, verify_local_cpu_store,
};

impl TvmdCli {
    pub fn execute(&self) -> std::result::Result<String, String> {
        self.command.execute()
    }
}

impl TvmdCommand {
    pub fn execute(&self) -> std::result::Result<String, String> {
        execute_tvmd_command(self)
    }
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
        TvmdCommand::Role(RoleCommand::Miner(MinerCommand::Register(args))) => {
            check_miner_registration(args.stake)
        }
        TvmdCommand::Role(RoleCommand::Miner(MinerCommand::Check(args))) => check_miner_start(
            &path_arg(&args.wallet),
            &args.device,
            &args.node.to_string(),
        ),
        TvmdCommand::Role(RoleCommand::Miner(MinerCommand::Run(args))) => {
            let runtime = &args.runtime;
            let node_runtime = &runtime.node_runtime;
            let wallet = path_arg(&args.wallet);
            let node = runtime.node.to_string();
            let listen = node_runtime.listen.to_string();
            let p2p_listen = node_runtime.p2p_listen.to_string();
            let data_dir = path_arg(&node_runtime.data_dir);
            validate_miner_runtime(&wallet, &args.device, &data_dir, &node_runtime.auth_token)?;
            run_miner_service(RoleServiceConfig {
                wallet: &wallet,
                device: Some(&args.device),
                node: &node,
                listen: &listen,
                p2p_listen: &p2p_listen,
                data_dir: &data_dir,
                identity_seed: node_runtime.identity_seed,
                auth_token: &node_runtime.auth_token,
                max_requests: node_runtime.max_requests,
            })
        }
        TvmdCommand::Role(RoleCommand::Miner(MinerCommand::Status)) => Ok(miner_status()),
        TvmdCommand::Role(RoleCommand::Validator(ValidatorCommand::Register(args))) => {
            check_validator_registration(args.stake)
        }
        TvmdCommand::Role(RoleCommand::Validator(ValidatorCommand::Check(args))) => {
            check_validator_start(&path_arg(&args.wallet), &args.node.to_string())
        }
        TvmdCommand::Role(RoleCommand::Validator(ValidatorCommand::Run(args))) => {
            let runtime = &args.runtime;
            let node_runtime = &runtime.node_runtime;
            let wallet = path_arg(&args.wallet);
            let node = runtime.node.to_string();
            let listen = node_runtime.listen.to_string();
            let p2p_listen = node_runtime.p2p_listen.to_string();
            let data_dir = path_arg(&node_runtime.data_dir);
            validate_role_runtime(&wallet, &data_dir, &node_runtime.auth_token)?;
            run_validator_service(RoleServiceConfig {
                wallet: &wallet,
                device: None,
                node: &node,
                listen: &listen,
                p2p_listen: &p2p_listen,
                data_dir: &data_dir,
                identity_seed: node_runtime.identity_seed,
                auth_token: &node_runtime.auth_token,
                max_requests: node_runtime.max_requests,
            })
        }
        TvmdCommand::Role(RoleCommand::Validator(ValidatorCommand::Status)) => {
            Ok(validator_status())
        }
        TvmdCommand::Role(RoleCommand::Proposer(ProposerCommand::Run(args))) => {
            let runtime = &args.runtime;
            let node_runtime = &runtime.node_runtime;
            let wallet = path_arg(&args.wallet);
            let node = runtime.node.to_string();
            let listen = node_runtime.listen.to_string();
            let p2p_listen = node_runtime.p2p_listen.to_string();
            let data_dir = path_arg(&node_runtime.data_dir);
            validate_role_runtime(&wallet, &data_dir, &node_runtime.auth_token)?;
            run_proposer_service(RoleServiceConfig {
                wallet: &wallet,
                device: None,
                node: &node,
                listen: &listen,
                p2p_listen: &p2p_listen,
                data_dir: &data_dir,
                identity_seed: node_runtime.identity_seed,
                auth_token: &node_runtime.auth_token,
                max_requests: node_runtime.max_requests,
            })
        }
        TvmdCommand::Node(NodeCommand::Init(args)) => {
            validate_data_dir(&path_arg(&args.data_dir))?;
            init_service_store(&path_arg(&args.data_dir))
        }
        TvmdCommand::Node(NodeCommand::Peer(NodePeerCommand::Add(args))) => {
            validate_data_dir(&path_arg(&args.data_dir))?;
            add_service_peer(
                &path_arg(&args.data_dir),
                &args.peer_id.to_string(),
                &args.address.to_string(),
            )
        }
        TvmdCommand::Node(NodeCommand::Check(args)) => {
            validate_data_dir(&path_arg(&args.data_dir))?;
            check_service_readiness(
                &args.p2p_listen.to_string(),
                &path_arg(&args.data_dir),
                args.identity_seed,
            )
        }
        TvmdCommand::Node(NodeCommand::Serve(args)) => {
            let runtime = &args.runtime;
            let listen = runtime.listen.to_string();
            let p2p_listen = runtime.p2p_listen.to_string();
            let data_dir = path_arg(&runtime.data_dir);
            validate_service_runtime(&data_dir, &runtime.auth_token)?;
            serve_service(
                &listen,
                &p2p_listen,
                &data_dir,
                runtime.identity_seed,
                &runtime.auth_token,
                runtime.max_requests,
            )
        }
        TvmdCommand::Node(NodeCommand::Status(args)) => {
            validate_data_dir(&path_arg(&args.data_dir))?;
            service_status(&path_arg(&args.data_dir))
        }
        TvmdCommand::Node(NodeCommand::Block(args)) => {
            validate_data_dir(&path_arg(&args.data_dir))?;
            service_block_status(&path_arg(&args.data_dir), args.height)
        }
        TvmdCommand::Localnet(LocalnetCommand::Seed(args)) => {
            validate_data_dir(&path_arg(&args.data_dir))?;
            seed_local_testnet(&path_arg(&args.data_dir))
        }
        TvmdCommand::Localnet(LocalnetCommand::Verify(args)) => {
            validate_data_dir(&path_arg(&args.data_dir))?;
            verify_local_cpu_store(&path_arg(&args.data_dir), args.json)
        }
        TvmdCommand::Public(PublicCommand::Evidence(command)) => {
            execute_public_evidence_command(command).map_err(|error| error.to_string())
        }
    }
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
