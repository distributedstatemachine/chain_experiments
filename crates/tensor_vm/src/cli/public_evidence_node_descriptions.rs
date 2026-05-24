use super::arguments::public_node_role_tag;
use super::commands::PublicEvidenceCommand;
use super::validation::path_argument;
use crate::hash::hex;

pub(super) fn describe_public_evidence_node_command(
    command: &PublicEvidenceCommand,
) -> Option<String> {
    match command {
        PublicEvidenceCommand::NodeHeartbeat(args) => Some(format!(
            "generate {} node heartbeat evidence address={}",
            public_node_role_tag(args.role.into()),
            hex(&args.address)
        )),
        PublicEvidenceCommand::NodeHeartbeatFromFile(args) => Some(format!(
            "generate {} node heartbeat evidence from captured observations heartbeat_file={} address={}",
            public_node_role_tag(args.role.into()),
            path_argument(&args.heartbeat_file),
            hex(&args.address)
        )),
        PublicEvidenceCommand::OperatorAttestation(args) => Some(format!(
            "generate {} operator identity attestation address={} identity_uri={}",
            public_node_role_tag(args.role.into()),
            hex(&args.address),
            args.identity_uri
        )),
        _ => None,
    }
}
