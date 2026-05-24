use super::node_evidence::{
    node_heartbeat_evidence_line, node_heartbeat_evidence_line_from_file,
    operator_identity_attestation_evidence_line,
};
use super::public_evidence_parser::PublicEvidenceCommand;
use crate::error::Result;

pub(super) fn execute_public_evidence_node_command(
    command: &PublicEvidenceCommand,
) -> Option<Result<String>> {
    match command {
        PublicEvidenceCommand::NodeHeartbeat(args) => Some(node_heartbeat_evidence_line(
            args.role.into(),
            args.address,
            args.operator_id,
            args.first_block,
            args.last_block,
            args.heartbeat_count,
        )),
        PublicEvidenceCommand::NodeHeartbeatFromFile(args) => {
            Some(node_heartbeat_evidence_line_from_file(
                args.role.into(),
                args.address,
                args.operator_id,
                &args.heartbeat_file,
            ))
        }
        PublicEvidenceCommand::OperatorAttestation(args) => {
            Some(operator_identity_attestation_evidence_line(
                args.role.into(),
                args.address,
                args.operator_id,
                &args.identity_uri,
                args.observed_at,
            ))
        }
        _ => None,
    }
}
