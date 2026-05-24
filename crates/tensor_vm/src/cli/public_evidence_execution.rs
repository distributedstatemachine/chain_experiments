use super::TvmdCommand;
use super::commands::EvidenceCommand;
use super::descriptions::describe_cli_command;
use super::public_evidence_network_execution::execute_public_evidence_network_command;
use super::public_evidence_node_execution::execute_public_evidence_node_command;
use super::public_evidence_publication_execution::execute_public_evidence_publication_command;
use super::public_evidence_record_execution::execute_public_evidence_record_command;
use super::public_evidence_run_window_execution::execute_public_evidence_run_window_command;
use super::public_evidence_service_execution::execute_public_evidence_service_command;
use crate::error::Result;

pub(super) fn execute_public_evidence_cli_command(command: &TvmdCommand) -> Result<String> {
    let TvmdCommand::Evidence(public_command) = command else {
        unreachable!("local commands are handled by cli::local_execution")
    };

    match public_command {
        EvidenceCommand::Validate(_) => Ok(describe_cli_command(command)),
        EvidenceCommand::Publish(_) | EvidenceCommand::Audit(_) => {
            execute_public_evidence_publication_command(public_command)
        }
        EvidenceCommand::Run(command) => execute_public_evidence_run_window_command(command),
        EvidenceCommand::Node(command) => execute_public_evidence_node_command(command),
        EvidenceCommand::Service(command) => execute_public_evidence_service_command(command),
        EvidenceCommand::Network(command) => execute_public_evidence_network_command(command),
        EvidenceCommand::Record(command) => execute_public_evidence_record_command(command),
    }
}
