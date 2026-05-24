use super::CliCommand;
use super::descriptions::describe_command;
use super::public_evidence_network_execution::execute_public_evidence_network_command;
use super::public_evidence_node_execution::execute_public_evidence_node_command;
use super::public_evidence_publication_execution::execute_public_evidence_publication_command;
use super::public_evidence_record_execution::execute_public_evidence_record_command;
use super::public_evidence_run_window_execution::execute_public_evidence_run_window_command;
use super::public_evidence_service_execution::execute_public_evidence_service_command;
use crate::error::Result;

pub(super) fn execute_public_evidence_cli_command(command: &CliCommand) -> Result<String> {
    if let Some(output) = execute_public_evidence_service_command(command) {
        return output;
    }
    if let Some(output) = execute_public_evidence_record_command(command) {
        return output;
    }
    if let Some(output) = execute_public_evidence_network_command(command) {
        return output;
    }
    if let Some(output) = execute_public_evidence_publication_command(command) {
        return output;
    }
    if let Some(output) = execute_public_evidence_run_window_command(command) {
        return output;
    }
    if let Some(output) = execute_public_evidence_node_command(command) {
        return output;
    }

    match command {
        CliCommand::PublicEvidenceValidate { .. } | CliCommand::PublicTestnetPreflight { .. } => {
            Ok(describe_command(command))
        }
        _ => unreachable!("local commands are handled by cli::local_execution"),
    }
}
