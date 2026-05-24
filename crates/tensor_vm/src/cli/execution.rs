use super::CliCommand;
use super::local_execution::execute_local_cli_command;
use super::public_evidence_execution::execute_public_evidence_cli_command;
use crate::error::Result;

pub fn execute_reference_cli_command(command: &CliCommand) -> Result<String> {
    match command {
        CliCommand::Miner { .. }
        | CliCommand::Validator { .. }
        | CliCommand::Proposer { .. }
        | CliCommand::Service { .. }
        | CliCommand::LocalTestnet { .. }
        | CliCommand::LocalCpu { .. } => execute_local_cli_command(command),
        CliCommand::PublicEvidence { .. } | CliCommand::PublicTestnet { .. } => {
            execute_public_evidence_cli_command(command)
        }
    }
}
