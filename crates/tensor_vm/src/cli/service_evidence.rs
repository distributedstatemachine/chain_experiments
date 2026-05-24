use super::arguments::{parse_u64, public_service_kind_tag};
use crate::error::{Result, TvmError};
use crate::hash::hex;
use crate::testnet::{
    PublicServiceContentEvidence, PublicServiceEndpoint, PublicServiceEvidence, PublicServiceKind,
};
use crate::types::{Hash, hash_bytes};
use std::collections::BTreeSet;

pub(super) struct ServiceHealthEvidenceLine<'a> {
    pub(super) kind: PublicServiceKind,
    pub(super) endpoint_id: Hash,
    pub(super) public_url: &'a str,
    pub(super) health_path: &'a str,
    pub(super) first_seen_block: u64,
    pub(super) last_seen_block: u64,
    pub(super) reachable_observation_count: u64,
    pub(super) signed_health_check_count: u64,
}

pub(super) fn service_health_evidence_line(input: ServiceHealthEvidenceLine<'_>) -> Result<String> {
    if input.last_seen_block < input.first_seen_block {
        return Err(TvmError::InvalidReceipt(
            "service health block range is invalid",
        ));
    }
    let evidence = PublicServiceEvidence::new(
        input.kind,
        PublicServiceEndpoint::new(input.endpoint_id, input.public_url, input.health_path),
        input.first_seen_block,
        input.last_seen_block,
        input.reachable_observation_count,
        input.signed_health_check_count,
    );
    if !evidence.has_reachable_endpoint_proof() {
        return Err(TvmError::InvalidReceipt("invalid service health evidence"));
    }
    Ok(format!(
        "service={},{},{},{},{},{},{},{},{}",
        public_service_kind_tag(input.kind),
        hex(&evidence.endpoint_id),
        evidence.public_url,
        evidence.health_path,
        evidence.first_seen_block,
        evidence.last_seen_block,
        evidence.reachable_observation_count,
        evidence.signed_health_check_count,
        hex(&evidence.health_check_signature)
    ))
}

pub(super) struct ServiceHealthObservationSummary {
    pub(super) first_seen_block: u64,
    pub(super) last_seen_block: u64,
    pub(super) reachable_observation_count: u64,
    pub(super) signed_health_check_count: u64,
}

pub(super) fn service_health_evidence_line_from_file(
    kind: PublicServiceKind,
    endpoint_id: Hash,
    public_url: &str,
    health_path: &str,
    observation_file: &str,
) -> Result<String> {
    let contents = std::fs::read_to_string(observation_file)
        .map_err(|_| TvmError::Storage("failed to read service health observation file"))?;
    let summary = service_health_observation_summary_from_file(&contents)?;
    service_health_evidence_line(ServiceHealthEvidenceLine {
        kind,
        endpoint_id,
        public_url,
        health_path,
        first_seen_block: summary.first_seen_block,
        last_seen_block: summary.last_seen_block,
        reachable_observation_count: summary.reachable_observation_count,
        signed_health_check_count: summary.signed_health_check_count,
    })
}

pub(super) fn service_health_observation_summary_from_file(
    contents: &str,
) -> Result<ServiceHealthObservationSummary> {
    let mut observed_blocks = BTreeSet::new();
    let mut reachable_observation_count = 0_u64;
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line != raw_line {
            return Err(TvmError::InvalidReceipt(
                "service health observation line has leading or trailing whitespace",
            ));
        }
        let (block, reachable) = parse_service_health_observation_line(line)?;
        if !observed_blocks.insert(block) {
            return Err(TvmError::InvalidReceipt(
                "duplicate service health observation block",
            ));
        }
        if reachable {
            reachable_observation_count = reachable_observation_count.saturating_add(1);
        }
    }
    let Some(first_seen_block) = observed_blocks.iter().next().copied() else {
        return Err(TvmError::InvalidReceipt(
            "service health observation file has no observations",
        ));
    };
    let last_seen_block = observed_blocks.iter().next_back().copied().unwrap();
    let signed_health_check_count = observed_blocks.len() as u64;
    let expected_observation_count = last_seen_block
        .checked_sub(first_seen_block)
        .and_then(|span| span.checked_add(1))
        .ok_or(TvmError::InvalidReceipt(
            "service health observation block range is invalid",
        ))?;
    if signed_health_check_count != expected_observation_count {
        return Err(TvmError::InvalidReceipt(
            "service health observation blocks must be contiguous",
        ));
    }
    Ok(ServiceHealthObservationSummary {
        first_seen_block,
        last_seen_block,
        reachable_observation_count,
        signed_health_check_count,
    })
}

fn parse_service_health_observation_line(line: &str) -> Result<(u64, bool)> {
    let record =
        line.strip_prefix("service_health_observation=")
            .ok_or(TvmError::InvalidReceipt(
                "unsupported service health observation line",
            ))?;
    let fields: Vec<&str> = record.split(',').collect();
    if fields.len() != 2 {
        return Err(TvmError::InvalidReceipt(
            "malformed service health observation",
        ));
    }
    let block = parse_u64(fields[0])?;
    let reachable = match fields[1] {
        "reachable" => true,
        "unreachable" => false,
        _ => {
            return Err(TvmError::InvalidReceipt(
                "invalid service health observation status",
            ));
        }
    };
    Ok((block, reachable))
}

pub(super) fn service_content_evidence_line(
    kind: PublicServiceKind,
    endpoint_id: Hash,
    public_url: &str,
    content_path: &str,
    content_root: Hash,
    observed_at_unix_seconds: u64,
    min_content_bytes: u64,
) -> Result<String> {
    let evidence = PublicServiceContentEvidence::new(
        kind,
        endpoint_id,
        public_url,
        content_path,
        content_root,
        observed_at_unix_seconds,
        min_content_bytes,
    );
    if !evidence.has_external_content_proof() {
        return Err(TvmError::InvalidReceipt("invalid service content evidence"));
    }
    Ok(format!(
        "service_content={},{},{},{},{},{},{},{}",
        public_service_kind_tag(kind),
        hex(&evidence.endpoint_id),
        evidence.public_url,
        evidence.content_path,
        hex(&evidence.content_root),
        evidence.observed_at_unix_seconds,
        evidence.min_content_bytes,
        hex(&evidence.content_signature)
    ))
}

pub(super) fn public_service_content_root(content_bytes: &[u8]) -> Hash {
    hash_bytes(
        b"tensor-vm-public-service-content-root-v1",
        &[content_bytes],
    )
}

pub(super) fn service_content_evidence_line_from_bytes(
    kind: PublicServiceKind,
    endpoint_id: Hash,
    public_url: &str,
    content_path: &str,
    observed_at_unix_seconds: u64,
    content_bytes: &[u8],
) -> Result<String> {
    service_content_evidence_line(
        kind,
        endpoint_id,
        public_url,
        content_path,
        public_service_content_root(content_bytes),
        observed_at_unix_seconds,
        content_bytes.len() as u64,
    )
}
