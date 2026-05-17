use crate::field::{Elem, MODULUS};
use crate::hash::{Sha256, sha256};

#[derive(Clone)]
pub struct OracleRng {
    key: [u8; 32],
    counter: u64,
    block: [u8; 32],
    offset: usize,
}

impl OracleRng {
    pub fn new(label: &[u8], parts: &[&[u8]]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update_len_prefixed(label);
        for part in parts {
            hasher.update_len_prefixed(part);
        }
        Self {
            key: hasher.finalize(),
            counter: 0,
            block: [0; 32],
            offset: 32,
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut out = [0_u8; 8];
        for byte in &mut out {
            *byte = self.next_u8();
        }
        u64::from_le_bytes(out)
    }

    pub fn next_field(&mut self) -> Elem {
        let zone = u64::MAX - (u64::MAX % MODULUS);
        loop {
            let value = self.next_u64();
            if value < zone {
                return value % MODULUS;
            }
        }
    }

    fn next_u8(&mut self) -> u8 {
        if self.offset == self.block.len() {
            self.refill();
        }
        let byte = self.block[self.offset];
        self.offset += 1;
        byte
    }

    fn refill(&mut self) {
        let mut input = [0_u8; 40];
        input[..32].copy_from_slice(&self.key);
        input[32..].copy_from_slice(&self.counter.to_le_bytes());
        self.block = sha256(&input);
        self.counter = self.counter.wrapping_add(1);
        self.offset = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_is_deterministic() {
        let mut a = OracleRng::new(b"test", &[b"seed"]);
        let mut b = OracleRng::new(b"test", &[b"seed"]);
        for _ in 0..64 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }
}
