use crate::error::{Result, TvmError};
use crate::types::{Hash, hash_bytes, hash_pair};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerkleCommitment {
    pub root: Hash,
    pub leaf_count: u64,
    pub chunk_size: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerkleProof {
    pub leaf_index: u64,
    pub siblings: Vec<Hash>,
}

pub fn leaf_hash(tensor_id: &Hash, chunk_index: u64, chunk: &[u8]) -> Hash {
    hash_bytes(
        b"tensor-vm-merkle-leaf-v1",
        &[tensor_id, &chunk_index.to_le_bytes(), chunk],
    )
}

pub fn merkle_root(leaves: &[Hash]) -> Hash {
    if leaves.is_empty() {
        return hash_bytes(b"tensor-vm-empty-merkle-v1", &[]);
    }

    let mut level = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            let right = if pair.len() == 2 { &pair[1] } else { &pair[0] };
            next.push(hash_pair(b"tensor-vm-merkle-node-v1", &pair[0], right));
        }
        level = next;
    }
    level[0]
}

pub fn build_proof(leaves: &[Hash], leaf_index: u64) -> Result<MerkleProof> {
    let mut index = leaf_index as usize;
    if index >= leaves.len() {
        return Err(TvmError::InvalidChunk {
            chunk_index: leaf_index,
        });
    }

    let mut level = leaves.to_vec();
    let mut siblings = Vec::new();
    while level.len() > 1 {
        let sibling_index = if index.is_multiple_of(2) {
            (index + 1).min(level.len() - 1)
        } else {
            index - 1
        };
        siblings.push(level[sibling_index]);

        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            let right = if pair.len() == 2 { &pair[1] } else { &pair[0] };
            next.push(hash_pair(b"tensor-vm-merkle-node-v1", &pair[0], right));
        }
        index /= 2;
        level = next;
    }

    Ok(MerkleProof {
        leaf_index,
        siblings,
    })
}

pub fn verify_proof(root: &Hash, leaf: Hash, proof: &MerkleProof) -> bool {
    let mut index = proof.leaf_index;
    let mut current = leaf;
    for sibling in &proof.siblings {
        current = if index.is_multiple_of(2) {
            hash_pair(b"tensor-vm-merkle-node-v1", &current, sibling)
        } else {
            hash_pair(b"tensor-vm-merkle-node-v1", sibling, &current)
        };
        index /= 2;
    }
    &current == root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merkle_proofs_verify_each_leaf() {
        let tensor_id = hash_bytes(b"test", &[b"tensor"]);
        let leaves: Vec<_> = (0..5)
            .map(|i| leaf_hash(&tensor_id, i, &[i as u8, 42]))
            .collect();
        let root = merkle_root(&leaves);
        for i in 0..leaves.len() {
            let proof = build_proof(&leaves, i as u64).unwrap();
            assert!(verify_proof(&root, leaves[i], &proof));
        }
    }

    #[test]
    fn merkle_proof_rejects_tampering() {
        let tensor_id = hash_bytes(b"test", &[b"tensor"]);
        let leaves: Vec<_> = (0..3)
            .map(|i| leaf_hash(&tensor_id, i, &[i as u8]))
            .collect();
        let root = merkle_root(&leaves);
        let proof = build_proof(&leaves, 1).unwrap();
        let bad_leaf = leaf_hash(&tensor_id, 1, b"bad");
        assert!(!verify_proof(&root, bad_leaf, &proof));
    }

    #[test]
    fn merkle_empty_root_and_out_of_range_proof_are_explicit() {
        let empty = merkle_root(&[]);
        assert_eq!(empty, hash_bytes(b"tensor-vm-empty-merkle-v1", &[]));
        assert_eq!(
            build_proof(&[], 0),
            Err(TvmError::InvalidChunk { chunk_index: 0 })
        );
    }
}
