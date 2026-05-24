use super::CliCommand;
use super::descriptions::describe_command;
use super::node_evidence::{
    node_heartbeat_evidence_line, node_heartbeat_evidence_line_from_file,
    operator_identity_attestation_evidence_line,
};
use super::public_evidence_network_execution::execute_public_evidence_network_command;
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

    match command {
        CliCommand::PublicEvidenceNodeHeartbeat {
            role,
            address,
            operator_id,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        } => node_heartbeat_evidence_line(
            *role,
            *address,
            *operator_id,
            *first_seen_block,
            *last_seen_block,
            *signed_heartbeat_count,
        ),
        CliCommand::PublicEvidenceNodeHeartbeatFromFile {
            role,
            address,
            operator_id,
            heartbeat_file,
        } => node_heartbeat_evidence_line_from_file(*role, *address, *operator_id, heartbeat_file),
        CliCommand::PublicEvidenceOperatorAttestation {
            role,
            address,
            operator_id,
            identity_uri,
            observed_at_unix_seconds,
        } => operator_identity_attestation_evidence_line(
            *role,
            *address,
            *operator_id,
            identity_uri,
            *observed_at_unix_seconds,
        ),
        CliCommand::PublicEvidenceValidate { .. } | CliCommand::PublicTestnetPreflight { .. } => {
            Ok(describe_command(command))
        }
        _ => unreachable!("local commands are handled by cli::local_execution"),
    }
}
