use super::public_evidence_commands::EvidenceCommand;
use super::public_evidence_network_execution::execute_public_evidence_network_command;
use super::public_evidence_node_execution::execute_public_evidence_node_command;
use super::public_evidence_publication_execution::execute_public_evidence_publication_command;
use super::public_evidence_record_execution::execute_public_evidence_record_command;
use super::public_evidence_run_window_execution::execute_public_evidence_run_window_command;
use super::public_evidence_service_execution::execute_public_evidence_service_command;
use crate::error::{Result, TvmError};

pub(crate) fn execute_public_evidence_command(command: &EvidenceCommand) -> Result<String> {
    match command {
        EvidenceCommand::Validate(_) => Err(TvmError::InvalidReceipt(
            "evidence validate is handled by the app dispatcher",
        )),
        EvidenceCommand::Publish(_) | EvidenceCommand::Audit(_) => {
            execute_public_evidence_publication_command(command)
        }
        EvidenceCommand::Run(command) => execute_public_evidence_run_window_command(command),
        EvidenceCommand::Node(command) => execute_public_evidence_node_command(command),
        EvidenceCommand::Service(command) => execute_public_evidence_service_command(command),
        EvidenceCommand::Network(command) => execute_public_evidence_network_command(command),
        EvidenceCommand::Record(command) => execute_public_evidence_record_command(command),
    }
}
