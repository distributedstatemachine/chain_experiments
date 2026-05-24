use super::commands::EvidenceCommand;
use super::publication_evidence::{auditor_record_evidence_line, publication_evidence_lines};
use crate::error::Result;

pub(super) fn execute_public_evidence_publication_command(
    command: &EvidenceCommand,
) -> Result<String> {
    match command {
        EvidenceCommand::Publish(args) => publication_evidence_lines(
            args.bundle_id.into_hash(),
            &args.public_uri,
            args.manifest_signer.into_address(),
            args.manifest_signature_count,
            args.independent_auditor_count,
        ),
        EvidenceCommand::Audit(args) => auditor_record_evidence_line(
            args.bundle_id.into_hash(),
            &args.public_uri,
            args.auditor_id.into_address(),
            &args.audit_uri,
            args.observed_at,
        ),
        _ => unreachable!("non-publication evidence commands are routed before this executor"),
    }
}
