use super::CliCommand;
use super::arguments::public_service_kind_tag;

pub(super) fn describe_public_evidence_service_command(command: &CliCommand) -> Option<String> {
    match command {
        CliCommand::PublicEvidenceServiceHealth {
            kind,
            public_url,
            health_path,
            ..
        } => Some(format!(
            "generate {} service health evidence public_url={public_url} health_path={health_path}",
            public_service_kind_tag(*kind)
        )),
        CliCommand::PublicEvidenceServiceHealthFromFile {
            kind,
            public_url,
            health_path,
            observation_file,
            ..
        } => Some(format!(
            "generate {} service health evidence from captured observations observation_file={observation_file} public_url={public_url} health_path={health_path}",
            public_service_kind_tag(*kind)
        )),
        CliCommand::PublicEvidenceServiceContent {
            kind,
            public_url,
            content_path,
            ..
        } => Some(format!(
            "generate {} service content evidence public_url={public_url} content_path={content_path}",
            public_service_kind_tag(*kind)
        )),
        CliCommand::PublicEvidenceServiceContentFromBytes {
            kind,
            public_url,
            content_path,
            ..
        } => Some(format!(
            "generate {} service content evidence from observed bytes public_url={public_url} content_path={content_path}",
            public_service_kind_tag(*kind)
        )),
        CliCommand::PublicEvidenceServiceContentFromFile {
            kind,
            public_url,
            content_path,
            content_file,
            ..
        } => Some(format!(
            "generate {} service content evidence from captured file content_file={content_file} public_url={public_url} content_path={content_path}",
            public_service_kind_tag(*kind)
        )),
        _ => None,
    }
}
