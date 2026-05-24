use super::TvmdCommand;
use super::commands::TestnetCommand;
use super::local_role_execution::{
    execute_miner_command, execute_proposer_command, execute_validator_command,
};
use super::local_service_execution::execute_service_command;
use super::validation::{ensure_data_dir, json_escape, path_argument};
use crate::chain::ChainParams;
use crate::error::Result;

pub(super) fn execute_local_cli_command(command: &TvmdCommand) -> Result<String> {
    let params = ChainParams::default();
    match command {
        TvmdCommand::Miner(command) => execute_miner_command(command, &params),
        TvmdCommand::Validator(command) => execute_validator_command(command, &params),
        TvmdCommand::Proposer(command) => execute_proposer_command(command),
        TvmdCommand::Service(command) => execute_service_command(command),
        TvmdCommand::Testnet(command) => execute_testnet_command(command),
        _ => unreachable!("public evidence commands are handled by cli::execution"),
    }
}

fn execute_testnet_command(command: &TestnetCommand) -> Result<String> {
    match command {
        TestnetCommand::Seed(args) => {
            ensure_data_dir(&args.data_dir)?;
            let data_dir = path_argument(&args.data_dir);
            Ok(format!(
                "command=local_testnet_seed\ndata_dir={}\nlocal_cpu_seed_ready=true",
                data_dir
            ))
        }
        TestnetCommand::VerifyLocalCpu(args) => {
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
        TestnetCommand::Preflight(args) => Ok(format!(
            "run public testnet preflight manifest {}",
            path_argument(&args.manifest)
        )),
    }
}
