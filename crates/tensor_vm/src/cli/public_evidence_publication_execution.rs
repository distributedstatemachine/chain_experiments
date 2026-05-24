use super::public_evidence_parser::PublicEvidenceCommand;
use super::publication_evidence::{auditor_record_evidence_line, publication_evidence_lines};
use crate::error::Result;

pub(super) fn execute_public_evidence_publication_command(
    command: &PublicEvidenceCommand,
) -> Option<Result<String>> {
    match command {
        PublicEvidenceCommand::Publication(args) => Some(publication_evidence_lines(
            args.bundle_id,
            &args.public_uri,
            args.manifest_signer,
            args.manifest_signature_count,
            args.independent_auditor_count,
        )),
        PublicEvidenceCommand::AuditorRecord(args) => Some(auditor_record_evidence_line(
            args.bundle_id,
            &args.public_uri,
            args.auditor_id,
            &args.audit_uri,
            args.observed_at,
        )),
        _ => None,
    }
}
