use crate::error::{Result, TvmError};
use crate::types::{Address, Hash};

pub(super) fn parse_hash(value: &str) -> Result<Hash> {
    if value.len() != 64 {
        return Err(TvmError::InvalidReceipt("invalid hash length"));
    }
    let mut out = [0_u8; 32];
    for (i, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_value(chunk[0])?;
        let low = hex_value(chunk[1])?;
        out[i] = (high << 4) | low;
    }
    Ok(out)
}

pub(super) fn parse_address(value: &str) -> Result<Address> {
    parse_hash(value)
}

fn hex_value(value: u8) -> Result<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(TvmError::InvalidReceipt("invalid hex")),
    }
}
