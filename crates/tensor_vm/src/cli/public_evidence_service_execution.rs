use super::CliCommand;
use super::arguments::parse_hex_bytes_argument;
use super::service_evidence::{
    ServiceHealthEvidenceLine, service_content_evidence_line,
    service_content_evidence_line_from_bytes, service_health_evidence_line,
    service_health_evidence_line_from_file,
};
use crate::error::{Result, TvmError};

pub(super) fn execute_public_evidence_service_command(
    command: &CliCommand,
) -> Option<Result<String>> {
    match command {
        CliCommand::PublicEvidenceServiceHealth {
            kind,
            endpoint_id,
            public_url,
            health_path,
            first_seen_block,
            last_seen_block,
            reachable_observation_count,
            signed_health_check_count,
        } => Some(service_health_evidence_line(ServiceHealthEvidenceLine {
            kind: *kind,
            endpoint_id: *endpoint_id,
            public_url,
            health_path,
            first_seen_block: *first_seen_block,
            last_seen_block: *last_seen_block,
            reachable_observation_count: *reachable_observation_count,
            signed_health_check_count: *signed_health_check_count,
        })),
        CliCommand::PublicEvidenceServiceHealthFromFile {
            kind,
            endpoint_id,
            public_url,
            health_path,
            observation_file,
        } => Some(service_health_evidence_line_from_file(
            *kind,
            *endpoint_id,
            public_url,
            health_path,
            observation_file,
        )),
        CliCommand::PublicEvidenceServiceContent {
            kind,
            endpoint_id,
            public_url,
            content_path,
            content_root,
            observed_at_unix_seconds,
            min_content_bytes,
        } => Some(service_content_evidence_line(
            *kind,
            *endpoint_id,
            public_url,
            content_path,
            *content_root,
            *observed_at_unix_seconds,
            *min_content_bytes,
        )),
        CliCommand::PublicEvidenceServiceContentFromBytes {
            kind,
            endpoint_id,
            public_url,
            content_path,
            observed_at_unix_seconds,
            content_hex,
        } => Some(
            parse_hex_bytes_argument(content_hex).and_then(|content_bytes| {
                service_content_evidence_line_from_bytes(
                    *kind,
                    *endpoint_id,
                    public_url,
                    content_path,
                    *observed_at_unix_seconds,
                    &content_bytes,
                )
            }),
        ),
        CliCommand::PublicEvidenceServiceContentFromFile {
            kind,
            endpoint_id,
            public_url,
            content_path,
            observed_at_unix_seconds,
            content_file,
        } => Some(
            std::fs::read(content_file)
                .map_err(|_| TvmError::Storage("failed to read service content file"))
                .and_then(|content_bytes| {
                    service_content_evidence_line_from_bytes(
                        *kind,
                        *endpoint_id,
                        public_url,
                        content_path,
                        *observed_at_unix_seconds,
                        &content_bytes,
                    )
                }),
        ),
        _ => None,
    }
}
