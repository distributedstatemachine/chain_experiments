use crate::hash::Sha256;

pub type Hash = [u8; 32];
pub type Address = [u8; 32];
pub type Signature = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HashHexParseError {
    InvalidLength,
    InvalidHex,
}

pub(crate) fn parse_hash_hex(value: &str) -> std::result::Result<Hash, HashHexParseError> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.len() != 64 {
        return Err(HashHexParseError::InvalidLength);
    }
    let mut out = [0_u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        let high = parse_hex_nibble(value.as_bytes()[index * 2])?;
        let low = parse_hex_nibble(value.as_bytes()[index * 2 + 1])?;
        *byte = (high << 4) | low;
    }
    Ok(out)
}

fn parse_hex_nibble(value: u8) -> std::result::Result<u8, HashHexParseError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(HashHexParseError::InvalidHex),
    }
}

pub fn hash_bytes(domain: &[u8], parts: &[&[u8]]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update_len_prefixed(domain);
    for part in parts {
        hasher.update_len_prefixed(part);
    }
    hasher.finalize()
}

pub fn hash_pair(domain: &[u8], left: &Hash, right: &Hash) -> Hash {
    hash_bytes(domain, &[left, right])
}

pub fn u64_bytes(value: u64) -> [u8; 8] {
    value.to_le_bytes()
}

pub fn usize_bytes(value: usize) -> [u8; 8] {
    (value as u64).to_le_bytes()
}

pub fn address(label: &[u8]) -> Address {
    hash_bytes(b"tensor-vm-address-v1", &[label])
}

pub fn sign(address: &Address, message: &Hash) -> Signature {
    hash_bytes(b"tensor-vm-signature-v1", &[address, message])
}

pub fn verify_signature(address: &Address, message: &Hash, signature: &Signature) -> bool {
    sign(address, message) == *signature
}

pub fn hash_to_u128(hash: &Hash) -> u128 {
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    u128::from_le_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_hash_helpers_are_deterministic_and_domain_separated() {
        let left = hash_bytes(b"left", &[b"a"]);
        let right = hash_bytes(b"right", &[b"a"]);
        assert_ne!(left, right);
        assert_eq!(
            hash_pair(b"pair", &left, &right),
            hash_pair(b"pair", &left, &right)
        );
        assert_eq!(u64_bytes(42), 42_u64.to_le_bytes());
        assert_eq!(usize_bytes(42), 42_u64.to_le_bytes());
        assert_eq!(address(b"alice"), address(b"alice"));

        let message = hash_bytes(b"message", &[b"payload"]);
        let signer = address(b"signer");
        let signature = sign(&signer, &message);
        assert!(verify_signature(&signer, &message, &signature));
        assert!(!verify_signature(&address(b"other"), &message, &signature));

        let mut expected = [0_u8; 16];
        expected.copy_from_slice(&left[..16]);
        assert_eq!(hash_to_u128(&left), u128::from_le_bytes(expected));
    }

    #[test]
    fn hash_hex_parser_accepts_hash_text_and_reports_edges() {
        let hash = hash_bytes(b"hash-hex", &[b"value"]);
        assert_eq!(parse_hash_hex(&crate::hash::hex(&hash)).unwrap(), hash);
        assert_eq!(
            parse_hash_hex(&format!("0x{}", crate::hash::hex(&hash).to_uppercase())).unwrap(),
            hash
        );
        assert_eq!(parse_hash_hex("12"), Err(HashHexParseError::InvalidLength));
        assert_eq!(
            parse_hash_hex(&format!("z{}", "0".repeat(63))),
            Err(HashHexParseError::InvalidHex)
        );
    }
}
