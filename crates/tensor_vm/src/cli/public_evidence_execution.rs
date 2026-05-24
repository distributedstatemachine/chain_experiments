use super::TvmdCommand;
use super::commands::PublicEvidenceCommand;
use super::commands::PublicTestnetCommand;
use super::descriptions::describe_cli_command;
use super::public_evidence_network_execution::execute_public_evidence_network_command;
use super::public_evidence_node_execution::execute_public_evidence_node_command;
use super::public_evidence_publication_execution::execute_public_evidence_publication_command;
use super::public_evidence_record_execution::execute_public_evidence_record_command;
use super::public_evidence_run_window_execution::execute_public_evidence_run_window_command;
use super::public_evidence_service_execution::execute_public_evidence_service_command;
use crate::error::Result;

pub(super) fn execute_public_evidence_cli_command(command: &TvmdCommand) -> Result<String> {
    let TvmdCommand::PublicEvidence {
        command: public_command,
    } = command
    else {
        return match command {
            TvmdCommand::PublicTestnet {
                command: PublicTestnetCommand::Preflight(_),
            } => Ok(describe_cli_command(command)),
            _ => unreachable!("local commands are handled by cli::local_execution"),
        };
    };

    if let Some(output) = execute_public_evidence_service_command(public_command) {
        return output;
    }
    if let Some(output) = execute_public_evidence_record_command(public_command) {
        return output;
    }
    if let Some(output) = execute_public_evidence_network_command(public_command) {
        return output;
    }
    if let Some(output) = execute_public_evidence_publication_command(public_command) {
        return output;
    }
    if let Some(output) = execute_public_evidence_run_window_command(public_command) {
        return output;
    }
    if let Some(output) = execute_public_evidence_node_command(public_command) {
        return output;
    }

    match public_command {
        PublicEvidenceCommand::Validate(_) => Ok(describe_cli_command(command)),
        _ => unreachable!("public evidence subcommands are handled by family executors"),
    }
}
