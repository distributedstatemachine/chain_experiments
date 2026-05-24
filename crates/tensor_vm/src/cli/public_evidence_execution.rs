use super::CliCommand;
use super::descriptions::describe_command;
use super::network_evidence::{
    NetworkObservationEvidenceLine, network_observation_evidence_line,
    network_observation_evidence_line_from_service_log,
};
use super::node_evidence::{
    node_heartbeat_evidence_line, node_heartbeat_evidence_line_from_file,
    operator_identity_attestation_evidence_line,
};
use super::public_evidence_record_execution::execute_public_evidence_record_command;
use super::public_evidence_service_execution::execute_public_evidence_service_command;
use super::publication_evidence::{auditor_record_evidence_line, publication_evidence_lines};
use super::run_window_evidence::{run_window_evidence_line, run_window_evidence_line_from_file};
use crate::error::{Result, TvmError};

pub(super) fn execute_public_evidence_cli_command(command: &CliCommand) -> Result<String> {
    if let Some(output) = execute_public_evidence_service_command(command) {
        return output;
    }
    if let Some(output) = execute_public_evidence_record_command(command) {
        return output;
    }

    match command {
        CliCommand::PublicEvidenceNetworkObservation {
            operator_id,
            peer_id,
            listen_address,
            observed_at_unix_seconds,
            gossip_topic_count,
            request_response_protocol_count,
            bootstrap_peer_count,
            max_transmit_bytes,
            request_timeout_seconds,
            max_concurrent_streams,
            idle_connection_timeout_seconds,
        } => network_observation_evidence_line(NetworkObservationEvidenceLine {
            operator_id: *operator_id,
            peer_id,
            listen_address,
            observed_at_unix_seconds: *observed_at_unix_seconds,
            gossip_topic_count: *gossip_topic_count,
            request_response_protocol_count: *request_response_protocol_count,
            bootstrap_peer_count: *bootstrap_peer_count,
            max_transmit_bytes: *max_transmit_bytes,
            request_timeout_seconds: *request_timeout_seconds,
            max_concurrent_streams: *max_concurrent_streams,
            idle_connection_timeout_seconds: *idle_connection_timeout_seconds,
        }),
        CliCommand::PublicEvidenceNetworkObservationFromServiceLog {
            operator_id,
            listen_address,
            observed_at_unix_seconds,
            service_log,
        } => {
            let log_contents = std::fs::read_to_string(service_log)
                .map_err(|_| TvmError::Storage("failed to read service log file"))?;
            network_observation_evidence_line_from_service_log(
                *operator_id,
                listen_address,
                *observed_at_unix_seconds,
                &log_contents,
            )
        }
        CliCommand::PublicEvidencePublication {
            bundle_id,
            public_uri,
            manifest_signer,
            manifest_signature_count,
            independent_auditor_count,
        } => publication_evidence_lines(
            *bundle_id,
            public_uri,
            *manifest_signer,
            *manifest_signature_count,
            *independent_auditor_count,
        ),
        CliCommand::PublicEvidenceAuditorRecord {
            bundle_id,
            public_uri,
            auditor_id,
            audit_uri,
            observed_at_unix_seconds,
        } => auditor_record_evidence_line(
            *bundle_id,
            public_uri,
            *auditor_id,
            audit_uri,
            *observed_at_unix_seconds,
        ),
        CliCommand::PublicEvidenceRunWindow {
            bundle_id,
            manifest_signer,
            run_started_at_unix_seconds,
            run_ended_at_unix_seconds,
            observed_blocks,
        } => run_window_evidence_line(
            *bundle_id,
            *manifest_signer,
            *run_started_at_unix_seconds,
            *run_ended_at_unix_seconds,
            *observed_blocks,
        ),
        CliCommand::PublicEvidenceRunWindowFromFile {
            bundle_id,
            manifest_signer,
            block_observation_file,
        } => {
            run_window_evidence_line_from_file(*bundle_id, *manifest_signer, block_observation_file)
        }
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
