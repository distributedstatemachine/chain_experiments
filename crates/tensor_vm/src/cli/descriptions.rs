use super::TvmdCommand;
use super::local_descriptions::describe_local_command;
use super::public_evidence_descriptions::describe_public_evidence_command;

pub fn describe_cli_command(command: &TvmdCommand) -> String {
    match command {
        TvmdCommand::Miner(_)
        | TvmdCommand::Validator(_)
        | TvmdCommand::Proposer(_)
        | TvmdCommand::Service(_)
        | TvmdCommand::Testnet(_) => describe_local_command(command),
        TvmdCommand::Evidence(_) => describe_public_evidence_command(command),
    }
}
