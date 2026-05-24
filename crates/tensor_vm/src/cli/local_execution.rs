use super::TvmdCommand;
use super::commands::{LocalnetCommand, RoleCommand};
use super::local_role_execution::{
    execute_miner_command, execute_proposer_command, execute_validator_command,
};
use super::local_service_execution::execute_node_command;
use super::validation::{ensure_data_dir, json_escape, path_argument};
use crate::chain::ChainParams;
use crate::error::Result;

pub(super) fn execute_local_cli_command(command: &TvmdCommand) -> Result<String> {
    let params = ChainParams::default();
    match command {
        TvmdCommand::Role(RoleCommand::Miner(command)) => execute_miner_command(command, &params),
        TvmdCommand::Role(RoleCommand::Validator(command)) => {
            execute_validator_command(command, &params)
        }
        TvmdCommand::Role(RoleCommand::Proposer(command)) => execute_proposer_command(command),
        TvmdCommand::Node(command) => execute_node_command(command),
        TvmdCommand::Localnet(command) => execute_localnet_command(command),
        TvmdCommand::Public(_) => unreachable!("public commands are handled by cli::execution"),
    }
}

fn execute_localnet_command(command: &LocalnetCommand) -> Result<String> {
    match command {
        LocalnetCommand::Seed(args) => {
            ensure_data_dir(&args.data_dir)?;
            let data_dir = path_argument(&args.data_dir);
            Ok(format!(
                "command=local_testnet_seed\ndata_dir={}\nlocal_cpu_seed_ready=true",
                data_dir
            ))
        }
        LocalnetCommand::Verify(args) => {
            ensure_data_dir(&args.data_dir)?;
            let data_dir = path_argument(&args.data_dir);
            if args.json {
                Ok(format!(
                    "{{\"command\":\"local_cpu_verify\",\"data_dir\":\"{}\",\"structured_verifier_ready\":true}}",
                    json_escape(&data_dir)
                ))
            } else {
                Ok(format!(
                    "command=local_cpu_verify\ndata_dir={}\nstructured_verifier_ready=true",
                    data_dir
                ))
            }
        }
    }
}
