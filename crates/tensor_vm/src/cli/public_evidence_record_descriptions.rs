use super::arguments::public_evidence_record_kind_tag;
use super::commands::EvidenceRecordCommand;
use super::validation::path_argument;

pub(super) fn describe_public_evidence_record_command(command: &EvidenceRecordCommand) -> String {
    match command {
        EvidenceRecordCommand::Summary(args) => format!(
            "generate {} public evidence record summary records={}",
            public_evidence_record_kind_tag(args.kind.into()),
            args.record_count
        ),
        EvidenceRecordCommand::Artifact(args) => format!(
            "generate {} public evidence artifact locator artifact_uri={}",
            public_evidence_record_kind_tag(args.kind.into()),
            args.artifact_uri
        ),
        EvidenceRecordCommand::ArtifactRoots(args) => format!(
            "generate {} public evidence artifact locator from {} roots artifact_uri={}",
            public_evidence_record_kind_tag(args.kind.into()),
            args.record_roots.len(),
            args.artifact_uri
        ),
        EvidenceRecordCommand::ArtifactFile(args) => format!(
            "generate {} public evidence artifact locator from record file record_file={} artifact_uri={}",
            public_evidence_record_kind_tag(args.kind.into()),
            path_argument(&args.record_file),
            args.artifact_uri
        ),
        EvidenceRecordCommand::SummaryRoots(args) => format!(
            "generate {} public evidence record summary from {} roots",
            public_evidence_record_kind_tag(args.kind.into()),
            args.record_roots.len()
        ),
        EvidenceRecordCommand::SummaryFile(args) => format!(
            "generate {} public evidence record summary from record file record_file={}",
            public_evidence_record_kind_tag(args.kind.into()),
            path_argument(&args.record_file)
        ),
    }
}
