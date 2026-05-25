use crate::types::{
    Address, Hash, HashHexParseError, HexBytesParseError, parse_hash_hex, parse_hex_bytes,
};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HashArg(Hash);

impl HashArg {
    #[cfg(test)]
    pub(crate) fn new(value: Hash) -> Self {
        Self(value)
    }

    pub fn into_hash(self) -> Hash {
        self.0
    }
}

impl FromStr for HashArg {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        parse_hash_hex(value)
            .map(Self)
            .map_err(|error| hash_parse_error("hash", error))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AddressArg(Address);

impl AddressArg {
    #[cfg(test)]
    pub(crate) fn new(value: Address) -> Self {
        Self(value)
    }

    pub fn into_address(self) -> Address {
        self.0
    }
}

impl FromStr for AddressArg {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        parse_hash_hex(value)
            .map(Self)
            .map_err(|error| hash_parse_error("address", error))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HexBytesArg {
    bytes: Vec<u8>,
}

impl HexBytesArg {
    #[cfg(test)]
    pub(crate) fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }
}

impl FromStr for HexBytesArg {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        parse_hex_bytes(value)
            .map(|bytes| Self { bytes })
            .map_err(hex_bytes_parse_error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinerDeviceArg(String);

impl MinerDeviceArg {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for MinerDeviceArg {
    fn default() -> Self {
        Self("cpu".to_owned())
    }
}

impl fmt::Display for MinerDeviceArg {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for MinerDeviceArg {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        if value != value.trim() || value.is_empty() {
            return Err(miner_device_parse_error());
        }
        if value == "cpu" {
            return Ok(Self(value.to_owned()));
        }

        let Some(cuda_index) = value.strip_prefix("cuda:") else {
            return Err(miner_device_parse_error());
        };
        if cuda_index.is_empty() || cuda_index.parse::<u32>().is_err() {
            return Err(miner_device_parse_error());
        }
        Ok(Self(value.to_owned()))
    }
}

fn hash_parse_error(kind: &str, error: HashHexParseError) -> String {
    match error {
        HashHexParseError::InvalidLength => {
            format!("{kind} must be exactly 32 bytes of hex")
        }
        HashHexParseError::InvalidHex => format!("{kind} contains non-hex characters"),
    }
}

fn hex_bytes_parse_error(error: HexBytesParseError) -> String {
    match error {
        HexBytesParseError::Empty => "hex bytes must not be empty".to_owned(),
        HexBytesParseError::OddLength => {
            "hex bytes must contain an even number of digits".to_owned()
        }
        HexBytesParseError::InvalidHex => "hex bytes contain non-hex characters".to_owned(),
    }
}

fn miner_device_parse_error() -> String {
    "miner device must be cpu or cuda:N".to_owned()
}
