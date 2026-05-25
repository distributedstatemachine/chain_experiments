use crate::hash::hex;
use crate::testnet::{PublicNodeEvidence, PublicNodeRole, PublicOperatorIdentityAttestation};
use crate::types::{Hash, address, hash_bytes};

pub(super) fn manifest_node_signature(
    role: PublicNodeRole,
    address_label: &[u8],
    operator_label: &[u8],
) -> String {
    let node_address = address(address_label);
    let operator_id = hash_bytes(b"test", &[operator_label]);
    let node = match role {
        PublicNodeRole::Miner => PublicNodeEvidence::miner(node_address, operator_id, 0, 9, 10),
        PublicNodeRole::Validator => {
            PublicNodeEvidence::validator(node_address, operator_id, 0, 9, 10)
        }
    };
    hex(&node.heartbeat_signature)
}

pub(super) fn manifest_operator_identity_uri(operator_id: &Hash) -> String {
    format!("https://operators.tensorvm.net/{}", hex(operator_id))
}

pub(super) fn manifest_operator_signature(
    role: PublicNodeRole,
    address_label: &[u8],
    operator_label: &[u8],
) -> String {
    let node_address = address(address_label);
    let operator_id = hash_bytes(b"test", &[operator_label]);
    let attestation = PublicOperatorIdentityAttestation::new(
        role,
        node_address,
        operator_id,
        manifest_operator_identity_uri(&operator_id),
        1_700_000_000,
    );
    hex(&attestation.operator_signature)
}
