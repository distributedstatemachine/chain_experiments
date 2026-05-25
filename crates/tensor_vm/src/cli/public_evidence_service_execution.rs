use super::commands::EvidenceServiceCommand;
use super::service_evidence::{
    ServiceHealthEvidenceLine, service_content_evidence_line,
    service_content_evidence_line_from_bytes, service_health_evidence_line,
    service_health_evidence_line_from_file,
};
use super::validation::path_argument;
use crate::error::{Result, TvmError};

pub(super) fn execute_public_evidence_service_command(
    command: &EvidenceServiceCommand,
) -> Result<String> {
    match command {
        EvidenceServiceCommand::Health(args) => {
            service_health_evidence_line(ServiceHealthEvidenceLine {
                kind: args.endpoint.kind(),
                endpoint_id: args.endpoint.endpoint_id(),
                public_url: args.endpoint.public_url(),
                health_path: args.health.path(),
                first_seen_block: args.window.first_block(),
                last_seen_block: args.window.last_block(),
                reachable_observation_count: args.reachable_count,
                signed_health_check_count: args.signed_health_check_count,
            })
        }
        EvidenceServiceCommand::HealthFile(args) => service_health_evidence_line_from_file(
            args.endpoint.kind(),
            args.endpoint.endpoint_id(),
            args.endpoint.public_url(),
            args.health.path(),
            &path_argument(&args.observation_file),
        ),
        EvidenceServiceCommand::Content(args) => service_content_evidence_line(
            args.target.kind(),
            args.target.endpoint_id(),
            args.target.public_url(),
            args.target.content_path(),
            args.content_root.into_hash(),
            args.target.observation.observed_at(),
            args.min_content_bytes,
        ),
        EvidenceServiceCommand::ContentBytes(args) => service_content_evidence_line_from_bytes(
            args.target.kind(),
            args.target.endpoint_id(),
            args.target.public_url(),
            args.target.content_path(),
            args.target.observation.observed_at(),
            args.content.as_slice(),
        ),
        EvidenceServiceCommand::ContentFile(args) => std::fs::read(&args.content_file)
            .map_err(|_| TvmError::Storage("failed to read service content file"))
            .and_then(|content_bytes| {
                service_content_evidence_line_from_bytes(
                    args.target.kind(),
                    args.target.endpoint_id(),
                    args.target.public_url(),
                    args.target.content_path(),
                    args.target.observation.observed_at(),
                    &content_bytes,
                )
            }),
    }
}
