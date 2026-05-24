use super::PublicServiceKind;
use crate::error::{Result, TvmError};
use crate::record_fields::exact_comma_fields as exact_record_fields;
use crate::types::{Hash, HashHexParseError, parse_hash_hex as parse_typed_hash_hex};
use std::collections::BTreeSet;

pub(super) struct ManifestEntry<'a> {
    pub(super) key: &'a str,
    pub(super) value: &'a str,
}

pub(super) fn parse_manifest_entries<'a>(
    input: &'a str,
    allows_repeated: impl Fn(&str) -> bool,
    malformed_line_error: &'static str,
    duplicate_field_error: &'static str,
) -> Result<Vec<ManifestEntry<'a>>> {
    let mut scalar_fields = BTreeSet::new();
    let mut entries = Vec::new();
    for raw_line in input.lines() {
        let line = raw_line.trim_start();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = raw_line
            .split_once('=')
            .ok_or(TvmError::InvalidReceipt(malformed_line_error))?;
        reject_manifest_key_whitespace(key)?;
        let key = key.trim();
        if !allows_repeated(key) && !scalar_fields.insert(key.to_owned()) {
            return Err(TvmError::InvalidReceipt(duplicate_field_error));
        }
        entries.push(ManifestEntry { key, value });
    }
    Ok(entries)
}

pub(super) fn reject_manifest_key_whitespace(key: &str) -> Result<()> {
    if key.trim() != key {
        return Err(TvmError::InvalidReceipt("malformed manifest field key"));
    }
    Ok(())
}

pub(super) fn exact_manifest_record_fields<'a>(
    value: &'a str,
    expected_fields: usize,
    error: &'static str,
) -> Result<Vec<&'a str>> {
    exact_record_fields(value, expected_fields).ok_or(TvmError::InvalidReceipt(error))
}

pub(super) fn exact_manifest_scalar(value: &str) -> Result<&str> {
    if value.trim() != value {
        return Err(TvmError::InvalidReceipt("malformed manifest scalar value"));
    }
    Ok(value)
}

pub(super) fn parse_service_kind(value: &str) -> Result<PublicServiceKind> {
    match value {
        "rpc" => Ok(PublicServiceKind::Rpc),
        "explorer" => Ok(PublicServiceKind::Explorer),
        "faucet" => Ok(PublicServiceKind::Faucet),
        "telemetry" => Ok(PublicServiceKind::Telemetry),
        _ => Err(TvmError::InvalidReceipt("unknown service evidence kind")),
    }
}

pub(super) fn parse_hash_hex(value: &str) -> Result<Hash> {
    match parse_typed_hash_hex(value) {
        Ok(hash) => Ok(hash),
        Err(HashHexParseError::InvalidLength) => {
            Err(TvmError::InvalidReceipt("invalid evidence hash length"))
        }
        Err(HashHexParseError::InvalidHex) => {
            Err(TvmError::InvalidReceipt("invalid evidence hash hex"))
        }
    }
}

pub(super) fn parse_manifest_u64(value: &str) -> Result<u64> {
    value
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid evidence manifest number"))
}

pub(super) fn parse_manifest_usize(value: &str) -> Result<usize> {
    value
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid evidence manifest number"))
}

pub(super) fn parse_manifest_bool(value: &str) -> Result<bool> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(TvmError::InvalidReceipt(
            "invalid evidence manifest boolean",
        )),
    }
}

pub(super) fn required_u64(value: Option<u64>) -> Result<u64> {
    value.ok_or(TvmError::InvalidReceipt("missing evidence manifest number"))
}

pub(super) fn required_usize(value: Option<usize>) -> Result<usize> {
    value.ok_or(TvmError::InvalidReceipt("missing evidence manifest number"))
}

pub(super) fn required_bool(value: Option<bool>) -> Result<bool> {
    value.ok_or(TvmError::InvalidReceipt(
        "missing evidence manifest boolean",
    ))
}

pub(super) fn required_hash(value: Option<Hash>) -> Result<Hash> {
    value.ok_or(TvmError::InvalidReceipt("missing evidence manifest hash"))
}

pub(super) fn required_string(value: Option<String>) -> Result<String> {
    value.ok_or(TvmError::InvalidReceipt("missing evidence manifest string"))
}
