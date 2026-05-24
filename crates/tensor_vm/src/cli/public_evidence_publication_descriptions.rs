use super::public_evidence_parser::PublicEvidenceCommand;
use crate::hash::hex;

pub(super) fn describe_public_evidence_publication_command(
    command: &PublicEvidenceCommand,
) -> Option<String> {
    match command {
        PublicEvidenceCommand::Publication(args) => Some(format!(
            "generate public evidence publication signature public_uri={}",
            args.public_uri
        )),
        PublicEvidenceCommand::AuditorRecord(args) => Some(format!(
            "generate public evidence auditor record auditor_id={} audit_uri={}",
            hex(&args.auditor_id),
            args.audit_uri
        )),
        _ => None,
    }
}
