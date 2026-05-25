use super::evidence_fields::{
    exact_comma_fields, parse_hash_field, parse_u64_field, public_evidence_record_kind_tag,
};
use super::network_evidence::network_observation_root_from_record_line;
use crate::error::{Result, TvmError};
use crate::testnet::PublicEvidenceRecordKind;
use crate::types::{Hash, hash_bytes};
use std::collections::BTreeSet;

pub(super) fn aggregate_public_evidence_record_roots(
    kind: PublicEvidenceRecordKind,
    record_roots: &[Hash],
) -> Result<Hash> {
    if record_roots.is_empty() {
        return Err(TvmError::InvalidReceipt("record roots argument is empty"));
    }
    if record_roots.contains(&[0; 32]) {
        return Err(TvmError::InvalidReceipt("record root argument is empty"));
    }
    let mut unique_roots = BTreeSet::new();
    if record_roots.iter().any(|root| !unique_roots.insert(*root)) {
        return Err(TvmError::InvalidReceipt("duplicate record root argument"));
    }
    let record_count = (record_roots.len() as u64).to_le_bytes();
    let mut encoded_roots = Vec::with_capacity(record_roots.len() * 32);
    for root in record_roots {
        encoded_roots.extend_from_slice(root);
    }
    Ok(hash_bytes(
        b"tensor-vm-public-evidence-record-root-aggregation-v1",
        &[
            public_evidence_record_kind_tag(kind).as_bytes(),
            &record_count,
            &encoded_roots,
        ],
    ))
}

pub(super) fn public_evidence_record_roots_from_file(
    kind: PublicEvidenceRecordKind,
    record_file: &str,
) -> Result<Vec<Hash>> {
    let contents = std::fs::read_to_string(record_file)
        .map_err(|_| TvmError::Storage("failed to read public evidence record file"))?;
    let mut roots = Vec::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line != raw_line {
            return Err(TvmError::InvalidReceipt(
                "public evidence record line has leading or trailing whitespace",
            ));
        }
        roots.push(public_evidence_record_root_from_line(kind, line)?);
    }
    if roots.is_empty() {
        return Err(TvmError::InvalidReceipt("record file has no roots"));
    }
    Ok(roots)
}

pub(super) fn public_evidence_record_root_from_line(
    kind: PublicEvidenceRecordKind,
    line: &str,
) -> Result<Hash> {
    if let Some(root) = line.strip_prefix("record_root=") {
        return parse_record_file_root(root);
    }
    if kind == PublicEvidenceRecordKind::NetworkRuntimeObservations
        && let Some(record) = line.strip_prefix("network_runtime_observation=")
    {
        return network_observation_root_from_record_line(record);
    }
    if let Some(prefix) = supporting_record_line_prefix(kind)
        && line.starts_with(prefix)
    {
        return supporting_record_root_from_line(kind, line, prefix);
    }
    Err(TvmError::InvalidReceipt(
        "unsupported public evidence record line",
    ))
}

pub(super) fn supporting_record_line_prefix(
    kind: PublicEvidenceRecordKind,
) -> Option<&'static str> {
    match kind {
        PublicEvidenceRecordKind::BlockHistory => Some("block_history_record="),
        PublicEvidenceRecordKind::FinalityHistory => Some("finality_history_record="),
        PublicEvidenceRecordKind::NetworkRuntimeObservations => None,
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => {
            Some("data_availability_measurement=")
        }
        PublicEvidenceRecordKind::InvalidWorkRejections => Some("invalid_work_rejection="),
        PublicEvidenceRecordKind::RewardSettlements => Some("reward_settlement="),
    }
}

pub(super) fn supporting_record_root_from_line(
    kind: PublicEvidenceRecordKind,
    line: &str,
    prefix: &str,
) -> Result<Hash> {
    let payload = line.strip_prefix(prefix).ok_or(TvmError::InvalidReceipt(
        "unsupported public evidence record line",
    ))?;
    if payload.is_empty() || payload.trim() != payload {
        return Err(TvmError::InvalidReceipt(
            "invalid public evidence supporting record line",
        ));
    }
    validate_supporting_record_payload(kind, payload)?;
    Ok(hash_bytes(
        b"tensor-vm-public-evidence-supporting-record-root-v1",
        &[
            public_evidence_record_kind_tag(kind).as_bytes(),
            line.as_bytes(),
        ],
    ))
}

pub(super) fn validate_supporting_record_payload(
    kind: PublicEvidenceRecordKind,
    payload: &str,
) -> Result<()> {
    const INVALID_SUPPORTING_RECORD: &str = "invalid public evidence supporting record line";
    match kind {
        PublicEvidenceRecordKind::BlockHistory => {
            let fields = exact_comma_fields(payload, 2, INVALID_SUPPORTING_RECORD)?;
            parse_u64_field(fields[0])?;
            parse_hash_field(fields[1])?;
        }
        PublicEvidenceRecordKind::FinalityHistory => {
            let fields = exact_comma_fields(payload, 3, INVALID_SUPPORTING_RECORD)?;
            parse_u64_field(fields[0])?;
            parse_hash_field(fields[1])?;
            require_supporting_record_status(fields[2], &["finalized", "unfinalized"])?;
        }
        PublicEvidenceRecordKind::NetworkRuntimeObservations => {
            return Err(TvmError::InvalidReceipt(INVALID_SUPPORTING_RECORD));
        }
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => {
            let fields = exact_comma_fields(payload, 3, INVALID_SUPPORTING_RECORD)?;
            parse_hash_field(fields[0])?;
            require_supporting_record_status(fields[1], &["available", "unavailable"])?;
            parse_u64_field(fields[2])?;
        }
        PublicEvidenceRecordKind::InvalidWorkRejections => {
            let fields = exact_comma_fields(payload, 3, INVALID_SUPPORTING_RECORD)?;
            parse_hash_field(fields[0])?;
            require_supporting_record_status(fields[1], &["rejected"])?;
            parse_u64_field(fields[2])?;
        }
        PublicEvidenceRecordKind::RewardSettlements => {
            let fields = exact_comma_fields(payload, 4, INVALID_SUPPORTING_RECORD)?;
            parse_hash_field(fields[0])?;
            parse_hash_field(fields[1])?;
            parse_hash_field(fields[2])?;
            parse_u64_field(fields[3])?;
        }
    }
    Ok(())
}

fn require_supporting_record_status(status: &str, allowed: &[&str]) -> Result<()> {
    if !allowed.contains(&status) {
        return Err(TvmError::InvalidReceipt(
            "invalid public evidence supporting record line",
        ));
    }
    Ok(())
}

fn parse_record_file_root(root: &str) -> Result<Hash> {
    if root.trim() != root {
        return Err(TvmError::InvalidReceipt("invalid record root file line"));
    }
    parse_hash_field(root)
}
