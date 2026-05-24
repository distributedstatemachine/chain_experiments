use super::CliCommand;
use super::publication_evidence::{auditor_record_evidence_line, publication_evidence_lines};
use crate::error::Result;

pub(super) fn execute_public_evidence_publication_command(
    command: &CliCommand,
) -> Option<Result<String>> {
    match command {
        CliCommand::PublicEvidencePublication {
            bundle_id,
            public_uri,
            manifest_signer,
            manifest_signature_count,
            independent_auditor_count,
        } => Some(publication_evidence_lines(
            *bundle_id,
            public_uri,
            *manifest_signer,
            *manifest_signature_count,
            *independent_auditor_count,
        )),
        CliCommand::PublicEvidenceAuditorRecord {
            bundle_id,
            public_uri,
            auditor_id,
            audit_uri,
            observed_at_unix_seconds,
        } => Some(auditor_record_evidence_line(
            *bundle_id,
            public_uri,
            *auditor_id,
            audit_uri,
            *observed_at_unix_seconds,
        )),
        _ => None,
    }
}
