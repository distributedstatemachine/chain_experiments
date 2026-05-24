use super::arguments::public_service_kind_tag;
use super::commands::PublicEvidenceCommand;
use super::validation::path_argument;

pub(super) fn describe_public_evidence_service_command(
    command: &PublicEvidenceCommand,
) -> Option<String> {
    match command {
        PublicEvidenceCommand::ServiceHealth(args) => Some(format!(
            "generate {} service health evidence public_url={} health_path={}",
            public_service_kind_tag(args.kind.into()),
            args.public_url,
            args.health_path
        )),
        PublicEvidenceCommand::ServiceHealthFromFile(args) => Some(format!(
            "generate {} service health evidence from captured observations observation_file={} public_url={} health_path={}",
            public_service_kind_tag(args.kind.into()),
            path_argument(&args.observation_file),
            args.public_url,
            args.health_path
        )),
        PublicEvidenceCommand::ServiceContent(args) => Some(format!(
            "generate {} service content evidence public_url={} content_path={}",
            public_service_kind_tag(args.kind.into()),
            args.public_url,
            args.content_path
        )),
        PublicEvidenceCommand::ServiceContentFromBytes(args) => Some(format!(
            "generate {} service content evidence from observed bytes public_url={} content_path={}",
            public_service_kind_tag(args.kind.into()),
            args.public_url,
            args.content_path
        )),
        PublicEvidenceCommand::ServiceContentFromFile(args) => Some(format!(
            "generate {} service content evidence from captured file content_file={} public_url={} content_path={}",
            public_service_kind_tag(args.kind.into()),
            path_argument(&args.content_file),
            args.public_url,
            args.content_path
        )),
        _ => None,
    }
}
