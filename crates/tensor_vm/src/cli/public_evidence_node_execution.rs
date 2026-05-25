use super::commands::EvidenceNodeCommand;
use super::node_evidence::{
    node_heartbeat_evidence_line, node_heartbeat_evidence_line_from_file,
    operator_identity_attestation_evidence_line,
};
use super::validation::path_argument;
use crate::error::Result;

pub(super) fn execute_public_evidence_node_command(
    command: &EvidenceNodeCommand,
) -> Result<String> {
    match command {
        EvidenceNodeCommand::Heartbeat(args) => node_heartbeat_evidence_line(
            args.node.role(),
            args.node.address(),
            args.node.operator_id(),
            args.window.first_block(),
            args.window.last_block(),
            args.heartbeat_count(),
        ),
        EvidenceNodeCommand::HeartbeatFile(args) => node_heartbeat_evidence_line_from_file(
            args.node.role(),
            args.node.address(),
            args.node.operator_id(),
            &path_argument(args.heartbeat_file()),
        ),
        EvidenceNodeCommand::OperatorAttestation(args) => {
            operator_identity_attestation_evidence_line(
                args.node.role(),
                args.node.address(),
                args.node.operator_id(),
                args.identity_uri(),
                args.observed_at(),
            )
        }
    }
}
