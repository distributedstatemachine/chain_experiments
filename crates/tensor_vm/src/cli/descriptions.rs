use super::TvmdCommand;
use super::local_descriptions::describe_local_command;
use super::public_evidence_descriptions::describe_public_evidence_command;

pub fn describe_cli_command(command: &TvmdCommand) -> String {
    match command {
        TvmdCommand::Miner { .. }
        | TvmdCommand::Validator { .. }
        | TvmdCommand::Proposer { .. }
        | TvmdCommand::Service { .. }
        | TvmdCommand::LocalTestnet { .. }
        | TvmdCommand::LocalCpu { .. } => describe_local_command(command),
        TvmdCommand::PublicEvidence { .. } | TvmdCommand::PublicTestnet { .. } => {
            describe_public_evidence_command(command)
        }
    }
}
