use super::arguments::parse_hex_bytes_argument;
use super::commands::PublicEvidenceCommand;
use super::service_evidence::{
    ServiceHealthEvidenceLine, service_content_evidence_line,
    service_content_evidence_line_from_bytes, service_health_evidence_line,
    service_health_evidence_line_from_file,
};
use super::validation::path_argument;
use crate::error::{Result, TvmError};

pub(super) fn execute_public_evidence_service_command(
    command: &PublicEvidenceCommand,
) -> Option<Result<String>> {
    match command {
        PublicEvidenceCommand::ServiceHealth(args) => {
            Some(service_health_evidence_line(ServiceHealthEvidenceLine {
                kind: args.kind.into(),
                endpoint_id: args.endpoint_id,
                public_url: &args.public_url,
                health_path: &args.health_path,
                first_seen_block: args.first_block,
                last_seen_block: args.last_block,
                reachable_observation_count: args.reachable_count,
                signed_health_check_count: args.signed_health_check_count,
            }))
        }
        PublicEvidenceCommand::ServiceHealthFromFile(args) => {
            Some(service_health_evidence_line_from_file(
                args.kind.into(),
                args.endpoint_id,
                &args.public_url,
                &args.health_path,
                &path_argument(&args.observation_file),
            ))
        }
        PublicEvidenceCommand::ServiceContent(args) => Some(service_content_evidence_line(
            args.kind.into(),
            args.endpoint_id,
            &args.public_url,
            &args.content_path,
            args.content_root,
            args.observed_at,
            args.min_content_bytes,
        )),
        PublicEvidenceCommand::ServiceContentFromBytes(args) => Some(
            parse_hex_bytes_argument(&args.content_hex).and_then(|content_bytes| {
                service_content_evidence_line_from_bytes(
                    args.kind.into(),
                    args.endpoint_id,
                    &args.public_url,
                    &args.content_path,
                    args.observed_at,
                    &content_bytes,
                )
            }),
        ),
        PublicEvidenceCommand::ServiceContentFromFile(args) => Some(
            std::fs::read(&args.content_file)
                .map_err(|_| TvmError::Storage("failed to read service content file"))
                .and_then(|content_bytes| {
                    service_content_evidence_line_from_bytes(
                        args.kind.into(),
                        args.endpoint_id,
                        &args.public_url,
                        &args.content_path,
                        args.observed_at,
                        &content_bytes,
                    )
                }),
        ),
        _ => None,
    }
}
