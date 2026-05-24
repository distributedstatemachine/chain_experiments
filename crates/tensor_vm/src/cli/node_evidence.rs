use super::evidence_fields::{
    exact_comma_fields, parse_hash_field, parse_public_node_role, parse_u64_field,
    public_node_role_tag,
};
use crate::error::{Result, TvmError};
use crate::hash::hex;
use crate::testnet::{PublicNodeEvidence, PublicNodeRole, PublicOperatorIdentityAttestation};
use crate::types::{Address, Hash};
use std::collections::BTreeSet;

pub(super) fn node_heartbeat_evidence_line(
    role: PublicNodeRole,
    address: Address,
    operator_id: Hash,
    first_seen_block: u64,
    last_seen_block: u64,
    signed_heartbeat_count: u64,
) -> Result<String> {
    if address == [0; 32] {
        return Err(TvmError::InvalidReceipt("node address argument is empty"));
    }
    if last_seen_block < first_seen_block {
        return Err(TvmError::InvalidReceipt(
            "node heartbeat block range is invalid",
        ));
    }
    let node = match role {
        PublicNodeRole::Miner => PublicNodeEvidence::miner(
            address,
            operator_id,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        ),
        PublicNodeRole::Validator => PublicNodeEvidence::validator(
            address,
            operator_id,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        ),
    };
    if !node.has_external_operator_proof() {
        return Err(TvmError::InvalidReceipt("invalid node heartbeat evidence"));
    }
    Ok(format!(
        "node={},{},{},{},{},{},{}",
        public_node_role_tag(node.role),
        hex(&node.address),
        hex(&node.operator_id),
        node.first_seen_block,
        node.last_seen_block,
        node.signed_heartbeat_count,
        hex(&node.heartbeat_signature)
    ))
}

pub(super) struct NodeHeartbeatObservationSummary {
    pub(super) first_seen_block: u64,
    pub(super) last_seen_block: u64,
    pub(super) signed_heartbeat_count: u64,
}

pub(super) fn node_heartbeat_evidence_line_from_file(
    role: PublicNodeRole,
    address: Address,
    operator_id: Hash,
    heartbeat_file: &str,
) -> Result<String> {
    let contents = std::fs::read_to_string(heartbeat_file)
        .map_err(|_| TvmError::Storage("failed to read node heartbeat observation file"))?;
    let summary =
        node_heartbeat_observation_summary_from_file(role, address, operator_id, &contents)?;
    node_heartbeat_evidence_line(
        role,
        address,
        operator_id,
        summary.first_seen_block,
        summary.last_seen_block,
        summary.signed_heartbeat_count,
    )
}

pub(super) fn node_heartbeat_observation_summary_from_file(
    expected_role: PublicNodeRole,
    expected_address: Address,
    expected_operator_id: Hash,
    contents: &str,
) -> Result<NodeHeartbeatObservationSummary> {
    let mut observed_blocks = BTreeSet::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line != raw_line {
            return Err(TvmError::InvalidReceipt(
                "node heartbeat observation line has leading or trailing whitespace",
            ));
        }
        let (role, address, operator_id, block) = parse_node_heartbeat_observation_line(line)?;
        if role != expected_role
            || address != expected_address
            || operator_id != expected_operator_id
        {
            return Err(TvmError::InvalidReceipt(
                "node heartbeat observation identity mismatch",
            ));
        }
        if !observed_blocks.insert(block) {
            return Err(TvmError::InvalidReceipt(
                "duplicate node heartbeat observation block",
            ));
        }
    }
    let Some(first_seen_block) = observed_blocks.iter().next().copied() else {
        return Err(TvmError::InvalidReceipt(
            "node heartbeat observation file has no observations",
        ));
    };
    let last_seen_block = observed_blocks.iter().next_back().copied().unwrap();
    let signed_heartbeat_count = observed_blocks.len() as u64;
    let expected_heartbeat_count = last_seen_block
        .checked_sub(first_seen_block)
        .and_then(|span| span.checked_add(1))
        .ok_or(TvmError::InvalidReceipt(
            "node heartbeat observation block range is invalid",
        ))?;
    if signed_heartbeat_count != expected_heartbeat_count {
        return Err(TvmError::InvalidReceipt(
            "node heartbeat observation blocks must be contiguous",
        ));
    }
    Ok(NodeHeartbeatObservationSummary {
        first_seen_block,
        last_seen_block,
        signed_heartbeat_count,
    })
}

fn parse_node_heartbeat_observation_line(
    line: &str,
) -> Result<(PublicNodeRole, Address, Hash, u64)> {
    let record =
        line.strip_prefix("node_heartbeat_observation=")
            .ok_or(TvmError::InvalidReceipt(
                "unsupported node heartbeat observation line",
            ))?;
    let fields = exact_comma_fields(record, 4, "malformed node heartbeat observation")?;
    Ok((
        parse_public_node_role(fields[0])?,
        parse_hash_field(fields[1])?,
        parse_hash_field(fields[2])?,
        parse_u64_field(fields[3])?,
    ))
}

pub(super) fn operator_identity_attestation_evidence_line(
    role: PublicNodeRole,
    address: Address,
    operator_id: Hash,
    identity_uri: &str,
    observed_at_unix_seconds: u64,
) -> Result<String> {
    let attestation = PublicOperatorIdentityAttestation::new(
        role,
        address,
        operator_id,
        identity_uri.to_owned(),
        observed_at_unix_seconds,
    );
    if !attestation.has_external_identity_proof() {
        return Err(TvmError::InvalidReceipt(
            "invalid operator identity attestation",
        ));
    }
    Ok(format!(
        "operator={},{},{},{},{},{}",
        public_node_role_tag(attestation.role),
        hex(&attestation.address),
        hex(&attestation.operator_id),
        attestation.identity_uri,
        attestation.observed_at_unix_seconds,
        hex(&attestation.operator_signature)
    ))
}
