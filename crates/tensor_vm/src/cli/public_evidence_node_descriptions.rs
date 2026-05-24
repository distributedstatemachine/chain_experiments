use super::arguments::public_node_role_tag;
use super::commands::EvidenceNodeCommand;
use super::validation::path_argument;
use crate::hash::hex;

pub(super) fn describe_public_evidence_node_command(command: &EvidenceNodeCommand) -> String {
    match command {
        EvidenceNodeCommand::Heartbeat(args) => format!(
            "generate {} node heartbeat evidence address={}",
            public_node_role_tag(args.role.into()),
            hex(&args.address)
        ),
        EvidenceNodeCommand::HeartbeatFile(args) => format!(
            "generate {} node heartbeat evidence from captured observations heartbeat_file={} address={}",
            public_node_role_tag(args.role.into()),
            path_argument(&args.heartbeat_file),
            hex(&args.address)
        ),
        EvidenceNodeCommand::OperatorAttestation(args) => format!(
            "generate {} operator identity attestation address={} identity_uri={}",
            public_node_role_tag(args.role.into()),
            hex(&args.address),
            args.identity_uri
        ),
    }
}
