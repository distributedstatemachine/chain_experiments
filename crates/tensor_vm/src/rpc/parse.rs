use crate::error::{Result, TvmError};
use crate::types::{Address, Hash, parse_hash_hex};

pub(super) fn parse_hash(value: &str) -> Result<Hash> {
    if value.len() != 64 {
        return Err(TvmError::InvalidReceipt("invalid hash length"));
    }
    parse_hash_hex(value).map_err(|_| TvmError::InvalidReceipt("invalid hex"))
}

pub(super) fn parse_address(value: &str) -> Result<Address> {
    parse_hash(value)
}
