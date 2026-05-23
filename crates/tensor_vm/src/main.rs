use tensor_vm::{
    CliCommand,
    cli::{
        execute_reference_cli_command, validate_public_evidence_manifest,
        validate_public_testnet_preflight_manifest,
    },
    parse_cli_args,
};

#[path = "main/block_status.rs"]
mod block_status;

#[path = "main/commands.rs"]
mod commands;

#[path = "main/network.rs"]
mod network;

#[path = "main/roles.rs"]
mod roles;

#[path = "main/runtime.rs"]
mod runtime;

#[path = "main/runtime_config.rs"]
mod runtime_config;

#[path = "main/shared.rs"]
mod shared;

#[path = "main/status.rs"]
mod status;

use block_status::service_block_status;
use commands::{
    add_service_peer, check_service_readiness, init_service_store, seed_local_testnet,
    verify_local_cpu_store,
};
use runtime::{run_miner_service, run_proposer_service, run_validator_service, serve_service};
use runtime_config::RoleServiceConfig;
use status::service_status;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_cli_args(&args) {
        Ok(command) => match execute_command(&command) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}

fn execute_command(command: &CliCommand) -> std::result::Result<String, String> {
    match command {
        CliCommand::PublicEvidenceValidate { manifest } => {
            let contents = std::fs::read_to_string(manifest)
                .map_err(|error| format!("failed to read evidence manifest {manifest}: {error}"))?;
            validate_public_evidence_manifest(&contents).map_err(|error| error.to_string())
        }
        CliCommand::PublicTestnetPreflight { manifest } => {
            let contents = std::fs::read_to_string(manifest).map_err(|error| {
                format!("failed to read preflight manifest {manifest}: {error}")
            })?;
            validate_public_testnet_preflight_manifest(&contents).map_err(|error| error.to_string())
        }
        CliCommand::MinerRun {
            wallet,
            device,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            run_miner_service(RoleServiceConfig {
                wallet,
                device: Some(device),
                node,
                listen,
                p2p_listen,
                data_dir,
                identity_seed: *identity_seed,
                auth_token,
                max_requests: *max_requests,
            })
        }
        CliCommand::ValidatorRun {
            wallet,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            run_validator_service(RoleServiceConfig {
                wallet,
                device: None,
                node,
                listen,
                p2p_listen,
                data_dir,
                identity_seed: *identity_seed,
                auth_token,
                max_requests: *max_requests,
            })
        }
        CliCommand::ProposerRun {
            wallet,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            run_proposer_service(RoleServiceConfig {
                wallet,
                device: None,
                node,
                listen,
                p2p_listen,
                data_dir,
                identity_seed: *identity_seed,
                auth_token,
                max_requests: *max_requests,
            })
        }
        CliCommand::ServiceInit { data_dir } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            init_service_store(data_dir)
        }
        CliCommand::ServicePeerAdd {
            data_dir,
            peer_id,
            address,
        } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            add_service_peer(data_dir, peer_id, address)
        }
        CliCommand::ServiceReadiness {
            p2p_listen,
            data_dir,
            identity_seed,
        } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            check_service_readiness(p2p_listen, data_dir, *identity_seed)
        }
        CliCommand::ServiceServe {
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            serve_service(
                listen,
                p2p_listen,
                data_dir,
                *identity_seed,
                auth_token,
                *max_requests,
            )
        }
        CliCommand::ServiceStatus { data_dir } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            service_status(data_dir)
        }
        CliCommand::ServiceBlock { data_dir, height } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            service_block_status(data_dir, *height)
        }
        CliCommand::LocalTestnetSeed { data_dir } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            seed_local_testnet(data_dir)
        }
        CliCommand::LocalCpuVerify { data_dir, json } => {
            execute_reference_cli_command(command).map_err(|error| error.to_string())?;
            verify_local_cpu_store(data_dir, *json)
        }
        _ => execute_reference_cli_command(command).map_err(|error| error.to_string()),
    }
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
