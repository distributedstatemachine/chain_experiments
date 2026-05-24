use super::CliCommand;
use super::arguments::public_evidence_record_kind_tag;

pub(super) fn describe_public_evidence_record_command(command: &CliCommand) -> Option<String> {
    match command {
        CliCommand::PublicEvidenceRecordSummary {
            kind, record_count, ..
        } => Some(format!(
            "generate {} public evidence record summary records={record_count}",
            public_evidence_record_kind_tag(*kind)
        )),
        CliCommand::PublicEvidenceRecordArtifact {
            kind, artifact_uri, ..
        } => Some(format!(
            "generate {} public evidence artifact locator artifact_uri={artifact_uri}",
            public_evidence_record_kind_tag(*kind)
        )),
        CliCommand::PublicEvidenceRecordArtifactFromRoots {
            kind,
            artifact_uri,
            record_roots,
            ..
        } => Some(format!(
            "generate {} public evidence artifact locator from {} roots artifact_uri={artifact_uri}",
            public_evidence_record_kind_tag(*kind),
            record_roots.len()
        )),
        CliCommand::PublicEvidenceRecordArtifactFromFile {
            kind,
            artifact_uri,
            record_file,
            ..
        } => Some(format!(
            "generate {} public evidence artifact locator from record file record_file={record_file} artifact_uri={artifact_uri}",
            public_evidence_record_kind_tag(*kind),
        )),
        CliCommand::PublicEvidenceRecordSummaryFromRoots {
            kind, record_roots, ..
        } => Some(format!(
            "generate {} public evidence record summary from {} roots",
            public_evidence_record_kind_tag(*kind),
            record_roots.len()
        )),
        CliCommand::PublicEvidenceRecordSummaryFromFile {
            kind, record_file, ..
        } => Some(format!(
            "generate {} public evidence record summary from record file record_file={record_file}",
            public_evidence_record_kind_tag(*kind),
        )),
        _ => None,
    }
}
