use crate::hash::Sha256;

pub type Hash = [u8; 32];
pub type Address = [u8; 32];
pub type Signature = [u8; 32];

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
}
