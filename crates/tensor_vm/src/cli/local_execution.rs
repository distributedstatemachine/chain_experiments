use super::CliCommand;
use super::local_parser::{LocalCpuCommand, LocalTestnetCommand};
use super::local_role_execution::{
    execute_miner_command, execute_proposer_command, execute_validator_command,
};
use super::local_service_execution::execute_service_command;
use super::validation::{ensure_data_dir, json_escape};
use crate::chain::ChainParams;
use crate::error::Result;

pub(super) fn execute_local_cli_command(command: &CliCommand) -> Result<String> {
    let params = ChainParams::default();
    match command {
        CliCommand::Miner { command } => execute_miner_command(command, &params),
        CliCommand::Validator { command } => execute_validator_command(command, &params),
        CliCommand::Proposer { command } => execute_proposer_command(command),
        CliCommand::Service { command } => execute_service_command(command),
        CliCommand::LocalTestnet { command } => execute_local_testnet_command(command),
        CliCommand::LocalCpu { command } => execute_local_cpu_command(command),
        _ => unreachable!("public evidence commands are handled by cli::execution"),
    }
}

fn execute_local_testnet_command(command: &LocalTestnetCommand) -> Result<String> {
    match command {
        LocalTestnetCommand::Seed(args) => {
            ensure_data_dir(&args.data_dir)?;
            Ok(format!(
                "command=local_testnet_seed\ndata_dir={}\nlocal_cpu_seed_ready=true",
                args.data_dir
            ))
        }
    }
}

fn execute_local_cpu_command(command: &LocalCpuCommand) -> Result<String> {
    match command {
        LocalCpuCommand::Verify(args) => {
            ensure_data_dir(&args.data_dir)?;
            if args.json {
                Ok(format!(
                    "{{\"command\":\"local_cpu_verify\",\"data_dir\":\"{}\",\"structured_verifier_ready\":true}}",
                    json_escape(&args.data_dir)
                ))
            } else {
                Ok(format!(
                    "command=local_cpu_verify\ndata_dir={}\nstructured_verifier_ready=true",
                    args.data_dir
                ))
            }
        }
    }
}
