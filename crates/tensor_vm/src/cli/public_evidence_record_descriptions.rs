use super::arguments::public_evidence_record_kind_tag;
use super::public_evidence_parser::PublicEvidenceCommand;

pub(super) fn describe_public_evidence_record_command(
    command: &PublicEvidenceCommand,
) -> Option<String> {
    match command {
        PublicEvidenceCommand::RecordSummary(args) => Some(format!(
            "generate {} public evidence record summary records={}",
            public_evidence_record_kind_tag(args.kind.into()),
            args.record_count
        )),
        PublicEvidenceCommand::RecordArtifact(args) => Some(format!(
            "generate {} public evidence artifact locator artifact_uri={}",
            public_evidence_record_kind_tag(args.kind.into()),
            args.artifact_uri
        )),
        PublicEvidenceCommand::RecordArtifactFromRoots(args) => Some(format!(
            "generate {} public evidence artifact locator from {} roots artifact_uri={}",
            public_evidence_record_kind_tag(args.kind.into()),
            args.record_roots.0.len(),
            args.artifact_uri
        )),
        PublicEvidenceCommand::RecordArtifactFromFile(args) => Some(format!(
            "generate {} public evidence artifact locator from record file record_file={} artifact_uri={}",
            public_evidence_record_kind_tag(args.kind.into()),
            args.record_file,
            args.artifact_uri
        )),
        PublicEvidenceCommand::RecordSummaryFromRoots(args) => Some(format!(
            "generate {} public evidence record summary from {} roots",
            public_evidence_record_kind_tag(args.kind.into()),
            args.record_roots.0.len()
        )),
        PublicEvidenceCommand::RecordSummaryFromFile(args) => Some(format!(
            "generate {} public evidence record summary from record file record_file={}",
            public_evidence_record_kind_tag(args.kind.into()),
            args.record_file
        )),
        _ => None,
    }
}
