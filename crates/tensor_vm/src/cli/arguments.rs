use crate::error::{Result, TvmError};
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use crate::types::Hash;

pub(super) fn parse_u64(value: &str) -> Result<u64> {
    value
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid numeric argument"))
}

pub(super) fn parse_usize(value: &str) -> Result<usize> {
    value
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid numeric argument"))
}

pub(super) fn parse_public_service_kind(value: &str) -> Result<PublicServiceKind> {
    match value {
        "rpc" => Ok(PublicServiceKind::Rpc),
        "explorer" => Ok(PublicServiceKind::Explorer),
        "faucet" => Ok(PublicServiceKind::Faucet),
        "telemetry" => Ok(PublicServiceKind::Telemetry),
        _ => Err(TvmError::InvalidReceipt("invalid public service kind")),
    }
}

pub(super) fn public_service_kind_tag(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "rpc",
        PublicServiceKind::Explorer => "explorer",
        PublicServiceKind::Faucet => "faucet",
        PublicServiceKind::Telemetry => "telemetry",
    }
}

pub(super) fn parse_public_node_role(value: &str) -> Result<PublicNodeRole> {
    match value {
        "miner" => Ok(PublicNodeRole::Miner),
        "validator" => Ok(PublicNodeRole::Validator),
        _ => Err(TvmError::InvalidReceipt("invalid public node role")),
    }
}

pub(super) fn public_node_role_tag(role: PublicNodeRole) -> &'static str {
    match role {
        PublicNodeRole::Miner => "miner",
        PublicNodeRole::Validator => "validator",
    }
}

pub(super) fn parse_public_evidence_record_kind(value: &str) -> Result<PublicEvidenceRecordKind> {
    match value {
        "block-history" => Ok(PublicEvidenceRecordKind::BlockHistory),
        "finality-history" => Ok(PublicEvidenceRecordKind::FinalityHistory),
        "network-runtime" => Ok(PublicEvidenceRecordKind::NetworkRuntimeObservations),
        "data-availability" => Ok(PublicEvidenceRecordKind::DataAvailabilityMeasurements),
        "invalid-work" => Ok(PublicEvidenceRecordKind::InvalidWorkRejections),
        "reward-settlement" => Ok(PublicEvidenceRecordKind::RewardSettlements),
        _ => Err(TvmError::InvalidReceipt(
            "invalid public evidence record kind",
        )),
    }
}

pub(super) fn public_evidence_record_kind_tag(kind: PublicEvidenceRecordKind) -> &'static str {
    match kind {
        PublicEvidenceRecordKind::BlockHistory => "block-history",
        PublicEvidenceRecordKind::FinalityHistory => "finality-history",
        PublicEvidenceRecordKind::NetworkRuntimeObservations => "network-runtime",
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => "data-availability",
        PublicEvidenceRecordKind::InvalidWorkRejections => "invalid-work",
        PublicEvidenceRecordKind::RewardSettlements => "reward-settlement",
    }
}

pub(super) fn public_evidence_record_field_prefix(kind: PublicEvidenceRecordKind) -> &'static str {
    match kind {
        PublicEvidenceRecordKind::BlockHistory => "block_history",
        PublicEvidenceRecordKind::FinalityHistory => "finality_history",
        PublicEvidenceRecordKind::NetworkRuntimeObservations => "network_runtime_observation",
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => "data_availability_measurement",
        PublicEvidenceRecordKind::InvalidWorkRejections => "invalid_work_rejection",
        PublicEvidenceRecordKind::RewardSettlements => "reward_settlement",
    }
}

pub(super) fn parse_hash_argument(value: &str) -> Result<Hash> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.len() != 64 {
        return Err(TvmError::InvalidReceipt("invalid hash argument"));
    }
    let mut out = [0u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        let high = parse_hash_nibble(value.as_bytes()[index * 2])?;
        let low = parse_hash_nibble(value.as_bytes()[index * 2 + 1])?;
        *byte = (high << 4) | low;
    }
    Ok(out)
}

pub(super) fn parse_hash_list_argument(value: &str) -> Result<Vec<Hash>> {
    if value.trim().is_empty() {
        return Err(TvmError::InvalidReceipt("empty hash list argument"));
    }
    let mut hashes = Vec::new();
    for part in value.split(',') {
        if part.is_empty() || part.trim() != part {
            return Err(TvmError::InvalidReceipt("invalid hash list argument"));
        }
        hashes.push(parse_hash_argument(part)?);
    }
    Ok(hashes)
}

pub(super) fn parse_hex_bytes_argument(value: &str) -> Result<Vec<u8>> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.is_empty() || !value.len().is_multiple_of(2) {
        return Err(TvmError::InvalidReceipt("invalid hex bytes argument"));
    }
    let mut out = Vec::with_capacity(value.len() / 2);
    for chunk in value.as_bytes().chunks_exact(2) {
        let high = parse_hex_nibble(chunk[0])?;
        let low = parse_hex_nibble(chunk[1])?;
        out.push((high << 4) | low);
    }
    Ok(out)
}

fn parse_hash_nibble(value: u8) -> Result<u8> {
    parse_hex_nibble(value).map_err(|_| TvmError::InvalidReceipt("invalid hash argument"))
}

fn parse_hex_nibble(value: u8) -> Result<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(TvmError::InvalidReceipt("invalid hex bytes argument")),
    }
}
