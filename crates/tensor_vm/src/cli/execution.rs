use super::TvmdCommand;
use super::local_execution::execute_local_cli_command;
use super::public_evidence_execution::execute_public_evidence_cli_command;
use crate::error::Result;

pub fn execute_cli_command(command: &TvmdCommand) -> Result<String> {
    match command {
        TvmdCommand::Miner { .. }
        | TvmdCommand::Validator { .. }
        | TvmdCommand::Proposer { .. }
        | TvmdCommand::Service { .. }
        | TvmdCommand::LocalTestnet { .. }
        | TvmdCommand::LocalCpu { .. } => execute_local_cli_command(command),
        TvmdCommand::PublicEvidence { .. } | TvmdCommand::PublicTestnet { .. } => {
            execute_public_evidence_cli_command(command)
        }
    }
}
