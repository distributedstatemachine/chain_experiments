use clap::Parser;
use tensor_vm::{
    TvmdCli, TvmdCommand,
    cli::{
        LocalCpuCommand, LocalTestnetCommand, MinerCommand, ProposerCommand, PublicEvidenceCommand,
        PublicTestnetCommand, ServiceCommand, ServicePeerCommand, ValidatorCommand,
        execute_cli_command, validate_public_evidence_manifest,
        validate_public_testnet_preflight_manifest,
    },
};

#[path = "main/block_status.rs"]
mod block_status;

#[path = "main/commands.rs"]
mod commands;

#[path = "main/miner_role.rs"]
mod miner_role;

#[path = "main/network.rs"]
mod network;

#[path = "main/runtime.rs"]
mod runtime;

#[path = "main/runtime_loop.rs"]
mod runtime_loop;

#[path = "main/runtime_network.rs"]
mod runtime_network;

#[path = "main/runtime_production.rs"]
mod runtime_production;

#[path = "main/runtime_rpc.rs"]
mod runtime_rpc;

#[path = "main/runtime_services.rs"]
mod runtime_services;

#[path = "main/runtime_commands.rs"]
mod runtime_commands;

#[path = "main/runtime_config.rs"]
mod runtime_config;

#[path = "main/runtime_status.rs"]
mod runtime_status;

#[path = "main/runtime_status_snapshot.rs"]
mod runtime_status_snapshot;

#[path = "main/runtime_validator.rs"]
mod runtime_validator;

#[path = "main/shared.rs"]
mod shared;

#[path = "main/status.rs"]
mod status;

#[path = "main/validator_fetch.rs"]
mod validator_fetch;

#[path = "main/validator_role.rs"]
mod validator_role;

use block_status::service_block_status;
use commands::{
    add_service_peer, check_service_readiness, init_service_store, seed_local_testnet,
    verify_local_cpu_store,
};
use runtime::serve_service;
use runtime_commands::{run_miner_service, run_proposer_service, run_validator_service};
use runtime_config::RoleServiceConfig;
use status::service_status;

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
        TvmdCommand::PublicEvidence {
            command: PublicEvidenceCommand::Validate(args),
        } => {
            let contents = std::fs::read_to_string(&args.manifest).map_err(|error| {
                format!(
                    "failed to read evidence manifest {}: {error}",
                    args.manifest
                )
            })?;
            validate_public_evidence_manifest(&contents).map_err(|error| error.to_string())
        }
        TvmdCommand::PublicTestnet {
            command: PublicTestnetCommand::Preflight(args),
        } => {
            let contents = std::fs::read_to_string(&args.manifest).map_err(|error| {
                format!(
                    "failed to read preflight manifest {}: {error}",
                    args.manifest
                )
            })?;
            validate_public_testnet_preflight_manifest(&contents).map_err(|error| error.to_string())
        }
        TvmdCommand::Miner {
            command: MinerCommand::Run(args),
        } => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            let runtime = &args.runtime;
            let service = &runtime.service;
            run_miner_service(RoleServiceConfig {
                wallet: &args.wallet,
                device: Some(&args.device),
                node: &runtime.node,
                listen: &service.listen,
                p2p_listen: &service.p2p_listen,
                data_dir: &service.data_dir,
                identity_seed: service.identity_seed,
                auth_token: &service.auth_token,
                max_requests: service.max_requests,
            })
        }
        TvmdCommand::Validator {
            command: ValidatorCommand::Run(args),
        } => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            let runtime = &args.runtime;
            let service = &runtime.service;
            run_validator_service(RoleServiceConfig {
                wallet: &args.wallet,
                device: None,
                node: &runtime.node,
                listen: &service.listen,
                p2p_listen: &service.p2p_listen,
                data_dir: &service.data_dir,
                identity_seed: service.identity_seed,
                auth_token: &service.auth_token,
                max_requests: service.max_requests,
            })
        }
        TvmdCommand::Proposer {
            command: ProposerCommand::Run(args),
        } => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            let runtime = &args.runtime;
            let service = &runtime.service;
            run_proposer_service(RoleServiceConfig {
                wallet: &args.wallet,
                device: None,
                node: &runtime.node,
                listen: &service.listen,
                p2p_listen: &service.p2p_listen,
                data_dir: &service.data_dir,
                identity_seed: service.identity_seed,
                auth_token: &service.auth_token,
                max_requests: service.max_requests,
            })
        }
        TvmdCommand::Service {
            command: ServiceCommand::Init(args),
        } => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            init_service_store(&args.data_dir)
        }
        TvmdCommand::Service {
            command:
                ServiceCommand::Peer {
                    command: ServicePeerCommand::Add(args),
                },
        } => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            add_service_peer(&args.data_dir, &args.peer_id, &args.address)
        }
        TvmdCommand::Service {
            command: ServiceCommand::Readiness(args),
        } => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            check_service_readiness(&args.p2p_listen, &args.data_dir, args.identity_seed)
        }
        TvmdCommand::Service {
            command: ServiceCommand::Serve(args),
        } => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            let runtime = &args.runtime;
            serve_service(
                &runtime.listen,
                &runtime.p2p_listen,
                &runtime.data_dir,
                runtime.identity_seed,
                &runtime.auth_token,
                runtime.max_requests,
            )
        }
        TvmdCommand::Service {
            command: ServiceCommand::Status(args),
        } => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            service_status(&args.data_dir)
        }
        TvmdCommand::Service {
            command: ServiceCommand::Block(args),
        } => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            service_block_status(&args.data_dir, args.height)
        }
        TvmdCommand::LocalTestnet {
            command: LocalTestnetCommand::Seed(args),
        } => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            seed_local_testnet(&args.data_dir)
        }
        TvmdCommand::LocalCpu {
            command: LocalCpuCommand::Verify(args),
        } => {
            execute_cli_command(command).map_err(|error| error.to_string())?;
            verify_local_cpu_store(&args.data_dir, args.json)
        }
        _ => execute_cli_command(command).map_err(|error| error.to_string()),
    }
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
