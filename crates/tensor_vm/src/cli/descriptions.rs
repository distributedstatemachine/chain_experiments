use super::CliCommand;
use super::local_descriptions::describe_local_command;
use super::public_evidence_descriptions::describe_public_evidence_command;

pub fn describe_command(command: &CliCommand) -> String {
    match command {
        CliCommand::Miner { .. }
        | CliCommand::Validator { .. }
        | CliCommand::Proposer { .. }
        | CliCommand::Service { .. }
        | CliCommand::LocalTestnet { .. }
        | CliCommand::LocalCpu { .. } => describe_local_command(command),
        CliCommand::PublicEvidence { .. } | CliCommand::PublicTestnet { .. } => {
            describe_public_evidence_command(command)
        }
    }
}
