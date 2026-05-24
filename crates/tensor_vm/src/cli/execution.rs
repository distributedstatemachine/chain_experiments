use super::CliCommand;
use super::arguments::parse_hex_bytes_argument;
use super::descriptions::describe_command;
use super::local_execution::execute_local_cli_command;
use super::network_evidence::{
    NetworkObservationEvidenceLine, network_observation_evidence_line,
    network_observation_evidence_line_from_service_log,
};
use super::node_evidence::{
    node_heartbeat_evidence_line, node_heartbeat_evidence_line_from_file,
    operator_identity_attestation_evidence_line,
};
use super::publication_evidence::{auditor_record_evidence_line, publication_evidence_lines};
use super::record_evidence::{
    aggregate_public_evidence_record_roots, public_evidence_record_roots_from_file,
    record_artifact_evidence_line, record_summary_evidence_lines,
};
use super::run_window_evidence::{run_window_evidence_line, run_window_evidence_line_from_file};
use super::service_evidence::{
    ServiceHealthEvidenceLine, service_content_evidence_line,
    service_content_evidence_line_from_bytes, service_health_evidence_line,
    service_health_evidence_line_from_file,
};
use crate::error::{Result, TvmError};

pub fn execute_reference_cli_command(command: &CliCommand) -> Result<String> {
    match command {
        CliCommand::MinerRegister { .. }
        | CliCommand::MinerStart { .. }
        | CliCommand::MinerRun { .. }
        | CliCommand::MinerStatus
        | CliCommand::ValidatorRegister { .. }
        | CliCommand::ValidatorStart { .. }
        | CliCommand::ValidatorRun { .. }
        | CliCommand::ValidatorStatus
        | CliCommand::ProposerRun { .. }
        | CliCommand::ServiceInit { .. }
        | CliCommand::ServicePeerAdd { .. }
        | CliCommand::ServiceReadiness { .. }
        | CliCommand::ServiceServe { .. }
        | CliCommand::ServiceStatus { .. }
        | CliCommand::ServiceBlock { .. }
        | CliCommand::LocalTestnetSeed { .. }
        | CliCommand::LocalCpuVerify { .. } => execute_local_cli_command(command),
        CliCommand::PublicEvidenceServiceHealth {
            kind,
            endpoint_id,
            public_url,
            health_path,
            first_seen_block,
            last_seen_block,
            reachable_observation_count,
            signed_health_check_count,
        } => service_health_evidence_line(ServiceHealthEvidenceLine {
            kind: *kind,
            endpoint_id: *endpoint_id,
            public_url,
            health_path,
            first_seen_block: *first_seen_block,
            last_seen_block: *last_seen_block,
            reachable_observation_count: *reachable_observation_count,
            signed_health_check_count: *signed_health_check_count,
        }),
        CliCommand::PublicEvidenceServiceHealthFromFile {
            kind,
            endpoint_id,
            public_url,
            health_path,
            observation_file,
        } => service_health_evidence_line_from_file(
            *kind,
            *endpoint_id,
            public_url,
            health_path,
            observation_file,
        ),
        CliCommand::PublicEvidenceServiceContent {
            kind,
            endpoint_id,
            public_url,
            content_path,
            content_root,
            observed_at_unix_seconds,
            min_content_bytes,
        } => service_content_evidence_line(
            *kind,
            *endpoint_id,
            public_url,
            content_path,
            *content_root,
            *observed_at_unix_seconds,
            *min_content_bytes,
        ),
        CliCommand::PublicEvidenceServiceContentFromBytes {
            kind,
            endpoint_id,
            public_url,
            content_path,
            observed_at_unix_seconds,
            content_hex,
        } => {
            let content_bytes = parse_hex_bytes_argument(content_hex)?;
            service_content_evidence_line_from_bytes(
                *kind,
                *endpoint_id,
                public_url,
                content_path,
                *observed_at_unix_seconds,
                &content_bytes,
            )
        }
        CliCommand::PublicEvidenceServiceContentFromFile {
            kind,
            endpoint_id,
            public_url,
            content_path,
            observed_at_unix_seconds,
            content_file,
        } => {
            let content_bytes = std::fs::read(content_file)
                .map_err(|_| TvmError::Storage("failed to read service content file"))?;
            service_content_evidence_line_from_bytes(
                *kind,
                *endpoint_id,
                public_url,
                content_path,
                *observed_at_unix_seconds,
                &content_bytes,
            )
        }
        CliCommand::PublicEvidenceRecordSummary {
            kind,
            bundle_id,
            manifest_signer,
            record_root,
            record_count,
        } => record_summary_evidence_lines(
            *kind,
            *bundle_id,
            *manifest_signer,
            *record_root,
            *record_count,
        ),
        CliCommand::PublicEvidenceRecordArtifact {
            kind,
            bundle_id,
            manifest_signer,
            artifact_uri,
            record_root,
            record_count,
        } => record_artifact_evidence_line(
            *kind,
            *bundle_id,
            *manifest_signer,
            artifact_uri,
            *record_root,
            *record_count,
        ),
        CliCommand::PublicEvidenceRecordArtifactFromRoots {
            kind,
            bundle_id,
            manifest_signer,
            artifact_uri,
            record_roots,
        } => {
            let record_root = aggregate_public_evidence_record_roots(*kind, record_roots)?;
            record_artifact_evidence_line(
                *kind,
                *bundle_id,
                *manifest_signer,
                artifact_uri,
                record_root,
                record_roots.len() as u64,
            )
        }
        CliCommand::PublicEvidenceRecordArtifactFromFile {
            kind,
            bundle_id,
            manifest_signer,
            artifact_uri,
            record_file,
        } => {
            let record_roots = public_evidence_record_roots_from_file(*kind, record_file)?;
            let record_root = aggregate_public_evidence_record_roots(*kind, &record_roots)?;
            record_artifact_evidence_line(
                *kind,
                *bundle_id,
                *manifest_signer,
                artifact_uri,
                record_root,
                record_roots.len() as u64,
            )
        }
        CliCommand::PublicEvidenceRecordSummaryFromRoots {
            kind,
            bundle_id,
            manifest_signer,
            record_roots,
        } => {
            let record_root = aggregate_public_evidence_record_roots(*kind, record_roots)?;
            record_summary_evidence_lines(
                *kind,
                *bundle_id,
                *manifest_signer,
                record_root,
                record_roots.len() as u64,
            )
        }
        CliCommand::PublicEvidenceRecordSummaryFromFile {
            kind,
            bundle_id,
            manifest_signer,
            record_file,
        } => {
            let record_roots = public_evidence_record_roots_from_file(*kind, record_file)?;
            let record_root = aggregate_public_evidence_record_roots(*kind, &record_roots)?;
            record_summary_evidence_lines(
                *kind,
                *bundle_id,
                *manifest_signer,
                record_root,
                record_roots.len() as u64,
            )
        }
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
    }
}
