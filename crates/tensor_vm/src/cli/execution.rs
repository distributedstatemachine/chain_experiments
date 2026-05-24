use super::TvmdCommand;
use super::local_execution::execute_local_cli_command;
use super::public_evidence_execution::execute_public_evidence_cli_command;
use crate::error::Result;

pub fn execute_cli_command(command: &TvmdCommand) -> Result<String> {
    match command {
        TvmdCommand::Miner(_)
        | TvmdCommand::Validator(_)
        | TvmdCommand::Proposer(_)
        | TvmdCommand::Service(_)
        | TvmdCommand::Testnet(_) => execute_local_cli_command(command),
        TvmdCommand::Evidence(_) => execute_public_evidence_cli_command(command),
    }
}
