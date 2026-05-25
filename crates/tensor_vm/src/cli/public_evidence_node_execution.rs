use super::node_evidence::{
    node_heartbeat_evidence_line, node_heartbeat_evidence_line_from_file,
    operator_identity_attestation_evidence_line,
};
use super::public_evidence_commands::EvidenceNodeCommand;
use super::public_evidence_node_commands::PublicNodeIdentityArgs;
use super::validation::path_argument;
use crate::error::Result;
use crate::testnet::PublicNodeRole;
use crate::types::{Address, Hash};

pub(super) fn execute_public_evidence_node_command(
    command: &EvidenceNodeCommand,
) -> Result<String> {
    match command {
        EvidenceNodeCommand::Heartbeat(args) => {
            let node = public_node_identity(&args.node);
            node_heartbeat_evidence_line(
                node.role,
                node.address,
                node.operator_id,
                args.window.first_block,
                args.window.last_block,
                args.heartbeat_count,
            )
        }
        EvidenceNodeCommand::HeartbeatFile(args) => {
            let node = public_node_identity(&args.node);
            node_heartbeat_evidence_line_from_file(
                node.role,
                node.address,
                node.operator_id,
                &path_argument(&args.heartbeat_file),
            )
        }
        EvidenceNodeCommand::OperatorAttestation(args) => {
            let node = public_node_identity(&args.node);
            operator_identity_attestation_evidence_line(
                node.role,
                node.address,
                node.operator_id,
                &args.identity_uri,
                args.observation.observed_at,
            )
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PublicNodeIdentityContext {
    role: PublicNodeRole,
    address: Address,
    operator_id: Hash,
}

fn public_node_identity(args: &PublicNodeIdentityArgs) -> PublicNodeIdentityContext {
    PublicNodeIdentityContext {
        role: args.role.into(),
        address: args.address.into_address(),
        operator_id: args.operator.operator_id.into_hash(),
    }
}
