use super::commands::EvidenceCommand;
use crate::hash::hex;

pub(super) fn describe_public_evidence_publication_command(command: &EvidenceCommand) -> String {
    match command {
        EvidenceCommand::Publish(args) => format!(
            "generate public evidence publication signature public_uri={}",
            args.public_uri
        ),
        EvidenceCommand::Audit(args) => format!(
            "generate public evidence auditor record auditor_id={} audit_uri={}",
            hex(&args.auditor_id),
            args.audit_uri
        ),
        _ => unreachable!("non-publication evidence commands are routed before this descriptor"),
    }
}
