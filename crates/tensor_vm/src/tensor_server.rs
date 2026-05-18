use crate::error::{Result, TvmError};
use crate::tensor::{DEFAULT_CHUNK_SIZE, Tensor, TensorDescriptor, TensorOpening};
use crate::types::Hash;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct TensorServer {
    tensors: BTreeMap<Hash, StoredTensor>,
}

#[derive(Clone, Debug)]
struct StoredTensor {
    tensor: Tensor,
    retain_until_block: u64,
}

impl TensorServer {
    pub fn insert(&mut self, tensor: Tensor) -> Hash {
        self.insert_with_retention(tensor, u64::MAX)
    }

    pub fn insert_with_retention(&mut self, tensor: Tensor, retain_until_block: u64) -> Hash {
        let id = tensor.tensor_id();
        self.tensors.insert(
            id,
            StoredTensor {
                tensor,
                retain_until_block,
            },
        );
        id
    }

    pub fn get(&self, tensor_id: &Hash) -> Option<&Tensor> {
        self.tensors.get(tensor_id).map(|stored| &stored.tensor)
    }

    pub fn get_by_commitment_root(&self, commitment_root: &Hash) -> Option<&Tensor> {
        self.tensors
            .values()
            .map(|stored| &stored.tensor)
            .find(|tensor| tensor.commitment_root() == *commitment_root)
    }

    pub fn contains_commitment_root(&self, commitment_root: &Hash) -> bool {
        self.get_by_commitment_root(commitment_root).is_some()
    }

    pub fn retention_deadline(&self, tensor_id: &Hash) -> Option<u64> {
        self.tensors
            .get(tensor_id)
            .map(|stored| stored.retain_until_block)
    }

    pub fn extend_retention(&mut self, tensor_id: &Hash, retain_until_block: u64) -> Result<()> {
        let stored = self
            .tensors
            .get_mut(tensor_id)
            .ok_or(TvmError::InvalidReceipt("tensor not found"))?;
        stored.retain_until_block = stored.retain_until_block.max(retain_until_block);
        Ok(())
    }

    pub fn prune_expired(&mut self, current_block: u64) -> usize {
        let before = self.tensors.len();
        self.tensors
            .retain(|_, stored| stored.retain_until_block >= current_block);
        before.saturating_sub(self.tensors.len())
    }

    pub fn descriptor(&self, tensor_id: &Hash) -> Result<TensorDescriptor> {
        self.get(tensor_id)
            .map(Tensor::descriptor)
            .ok_or(TvmError::InvalidReceipt("tensor not found"))
    }

    pub fn opening(&self, tensor_id: &Hash, chunk_index: u64) -> Result<TensorOpening> {
        self.get(tensor_id)
            .ok_or(TvmError::InvalidReceipt("tensor not found"))?
            .opening(chunk_index, DEFAULT_CHUNK_SIZE)
    }

    pub fn row(&self, tensor_id: &Hash, row_index: usize) -> Result<Vec<u64>> {
        Ok(self
            .get(tensor_id)
            .ok_or(TvmError::InvalidReceipt("tensor not found"))?
            .row(row_index)?
            .to_vec())
    }

    pub fn len(&self) -> usize {
        self.tensors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tensors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::{DType, Tensor};

    #[test]
    fn tensor_server_serves_rows_descriptors_and_openings() {
        let tensor = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let mut server = TensorServer::default();
        let id = server.insert(tensor);
        assert_eq!(server.len(), 1);
        assert_eq!(server.row(&id, 1).unwrap(), vec![3, 4]);
        let descriptor = server.descriptor(&id).unwrap();
        let opening = server.opening(&id, 0).unwrap();
        assert!(opening.verify(&descriptor));
        assert!(server.contains_commitment_root(&descriptor.commitment.root));
        assert_eq!(
            server
                .get_by_commitment_root(&descriptor.commitment.root)
                .unwrap()
                .tensor_id(),
            id
        );
    }

    #[test]
    fn tensor_server_retains_through_deadline_and_prunes_afterward() {
        let tensor = Tensor::from_vec(vec![1, 2], DType::FieldElement, vec![7, 8]).unwrap();
        let mut server = TensorServer::default();
        let id = server.insert_with_retention(tensor, 10);
        assert_eq!(server.retention_deadline(&id), Some(10));
        assert_eq!(server.prune_expired(10), 0);
        assert!(server.get(&id).is_some());
        assert_eq!(server.prune_expired(11), 1);
        assert!(server.get(&id).is_none());
    }

    #[test]
    fn tensor_server_extends_retention_only_forward() {
        let tensor = Tensor::from_vec(vec![1, 1], DType::FieldElement, vec![9]).unwrap();
        let mut server = TensorServer::default();
        let id = server.insert_with_retention(tensor, 5);
        server.extend_retention(&id, 3).unwrap();
        assert_eq!(server.retention_deadline(&id), Some(5));
        server.extend_retention(&id, 8).unwrap();
        assert_eq!(server.retention_deadline(&id), Some(8));
    }

    #[test]
    fn tensor_server_reports_missing_tensors() {
        let missing = crate::types::hash_bytes(b"test", &[b"missing-tensor"]);
        let mut server = TensorServer::default();
        assert!(server.is_empty());
        assert_eq!(
            server.descriptor(&missing),
            Err(TvmError::InvalidReceipt("tensor not found"))
        );
        assert_eq!(
            server.opening(&missing, 0),
            Err(TvmError::InvalidReceipt("tensor not found"))
        );
        assert_eq!(
            server.row(&missing, 0),
            Err(TvmError::InvalidReceipt("tensor not found"))
        );
        assert_eq!(
            server.extend_retention(&missing, 1),
            Err(TvmError::InvalidReceipt("tensor not found"))
        );
        assert_eq!(server.prune_expired(1), 0);
    }
}
