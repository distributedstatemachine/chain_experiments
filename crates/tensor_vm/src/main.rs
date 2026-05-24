use clap::Parser;
use tensor_vm::{
    TvmdCli, TvmdCommand,
    app::{
        RoleServiceConfig, add_service_peer, check_service_readiness, init_service_store,
        seed_local_testnet, service_block_status, service_status, verify_local_cpu_store,
    },
    cli::{
        EvidenceCommand, MinerCommand, ProposerCommand, ServiceCommand, ServicePeerCommand,
        TestnetCommand, ValidatorCommand, execute_cli_command, validate_public_evidence_manifest,
        validate_public_testnet_preflight_manifest,
    },
};

#[path = "main/miner_role.rs"]
mod miner_role;

#[path = "main/runtime.rs"]
mod runtime;

#[path = "main/runtime_loop.rs"]
mod runtime_loop;

#[path = "main/runtime_commands.rs"]
mod runtime_commands;

#[path = "main/runtime_validator.rs"]
mod runtime_validator;

use runtime::serve_service;
use runtime_commands::{run_miner_service, run_proposer_service, run_validator_service};

fn path_arg(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

fn main() {
    let command = TvmdCli::parse().command;
    match execute_command(&command) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn execute_command(command: &TvmdCommand) -> std::result::Result<String, String> {
    match command {
        TvmdCommand::Evidence(EvidenceCommand::Validate(args)) => {
            let contents = std::fs::read_to_string(&args.manifest).map_err(|error| {
                format!(
                    "failed to read evidence manifest {}: {error}",
                    path_arg(&args.manifest)
                )
            })?;
            validate_public_evidence_manifest(&contents).map_err(|error| error.to_string())
        }
        TvmdCommand::Testnet(TestnetCommand::Preflight(args)) => {
            let contents = std::fs::read_to_string(&args.manifest).map_err(|error| {
                format!(
                    "failed to read preflight manifest {}: {error}",
                    path_arg(&args.manifest)
                )
            })?;
            validate_public_testnet_preflight_manifest(&contents).map_err(|error| error.to_string())
        }
        TvmdCommand::Miner(MinerCommand::Run(args)) => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            let runtime = &args.runtime;
            let service = &runtime.service;
            let wallet = path_arg(&args.wallet);
            let node = runtime.node.to_string();
            let listen = service.listen.to_string();
            let p2p_listen = service.p2p_listen.to_string();
            let data_dir = path_arg(&service.data_dir);
            run_miner_service(RoleServiceConfig {
                wallet: &wallet,
                device: Some(&args.device),
                node: &node,
                listen: &listen,
                p2p_listen: &p2p_listen,
                data_dir: &data_dir,
                identity_seed: service.identity_seed,
                auth_token: &service.auth_token,
                max_requests: service.max_requests,
            })
        }
        TvmdCommand::Validator(ValidatorCommand::Run(args)) => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            let runtime = &args.runtime;
            let service = &runtime.service;
            let wallet = path_arg(&args.wallet);
            let node = runtime.node.to_string();
            let listen = service.listen.to_string();
            let p2p_listen = service.p2p_listen.to_string();
            let data_dir = path_arg(&service.data_dir);
            run_validator_service(RoleServiceConfig {
                wallet: &wallet,
                device: None,
                node: &node,
                listen: &listen,
                p2p_listen: &p2p_listen,
                data_dir: &data_dir,
                identity_seed: service.identity_seed,
                auth_token: &service.auth_token,
                max_requests: service.max_requests,
            })
        }
        TvmdCommand::Proposer(ProposerCommand::Run(args)) => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            let runtime = &args.runtime;
            let service = &runtime.service;
            let wallet = path_arg(&args.wallet);
            let node = runtime.node.to_string();
            let listen = service.listen.to_string();
            let p2p_listen = service.p2p_listen.to_string();
            let data_dir = path_arg(&service.data_dir);
            run_proposer_service(RoleServiceConfig {
                wallet: &wallet,
                device: None,
                node: &node,
                listen: &listen,
                p2p_listen: &p2p_listen,
                data_dir: &data_dir,
                identity_seed: service.identity_seed,
                auth_token: &service.auth_token,
                max_requests: service.max_requests,
            })
        }
        TvmdCommand::Service(ServiceCommand::Init(args)) => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            init_service_store(&path_arg(&args.data_dir))
        }
        TvmdCommand::Service(ServiceCommand::Peer(ServicePeerCommand::Add(args))) => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            add_service_peer(
                &path_arg(&args.data_dir),
                &args.peer_id.to_string(),
                &args.address.to_string(),
            )
        }
        TvmdCommand::Service(ServiceCommand::Readiness(args)) => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            check_service_readiness(
                &args.p2p_listen.to_string(),
                &path_arg(&args.data_dir),
                args.identity_seed,
            )
        }
        TvmdCommand::Service(ServiceCommand::Serve(args)) => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            let runtime = &args.runtime;
            let listen = runtime.listen.to_string();
            let p2p_listen = runtime.p2p_listen.to_string();
            let data_dir = path_arg(&runtime.data_dir);
            serve_service(
                &listen,
                &p2p_listen,
                &data_dir,
                runtime.identity_seed,
                &runtime.auth_token,
                runtime.max_requests,
            )
        }
        TvmdCommand::Service(ServiceCommand::Status(args)) => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            service_status(&path_arg(&args.data_dir))
        }
        TvmdCommand::Service(ServiceCommand::Block(args)) => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            service_block_status(&path_arg(&args.data_dir), args.height)
        }
        TvmdCommand::Testnet(TestnetCommand::Seed(args)) => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            seed_local_testnet(&path_arg(&args.data_dir))
        }
        TvmdCommand::Testnet(TestnetCommand::VerifyLocalCpu(args)) => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            verify_local_cpu_store(&path_arg(&args.data_dir), args.json)
        }
        _ => execute_cli_command(command).map_err(|error| error.to_string()),
    }
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
