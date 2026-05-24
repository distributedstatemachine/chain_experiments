use super::render::{json_u64_array, json_usize_array};
use super::{RpcNode, RpcResponse, parse_hash};
use crate::hash::hex;
use crate::tensor::{DEFAULT_CHUNK_SIZE, Tensor};

impl RpcNode {
    pub(super) fn tensor_descriptor(&self, tensor_id: &str) -> RpcResponse {
        let Some(tensor) = self.lookup_tensor(tensor_id) else {
            return self.not_found("tensor not found");
        };
        let descriptor = tensor.descriptor();
        self.ok(format!(
            "{{\"tensor_id\":\"{}\",\"shape\":{},\"byte_size\":{},\"root\":\"{}\"}}",
            hex(&descriptor.tensor_id),
            json_usize_array(&descriptor.shape),
            descriptor.byte_size,
            hex(&descriptor.commitment.root)
        ))
    }

    pub(super) fn tensor_chunk(&self, tensor_id: &str, chunk_index: &str) -> RpcResponse {
        let Some(tensor) = self.lookup_tensor(tensor_id) else {
            return self.not_found("tensor not found");
        };
        let Ok(chunk_index) = chunk_index.parse::<u64>() else {
            return self.bad_request("invalid chunk index");
        };
        match tensor.opening(chunk_index, DEFAULT_CHUNK_SIZE) {
            Ok(opening) => self.ok(format!(
                "{{\"tensor_id\":\"{}\",\"chunk_index\":{},\"bytes\":\"{}\"}}",
                hex(&opening.tensor_id),
                opening.chunk_index,
                hex(&opening.chunk_bytes)
            )),
            Err(_) => self.not_found("chunk not found"),
        }
    }

    pub(super) fn tensor_row(&self, tensor_id: &str, row_index: &str) -> RpcResponse {
        let Some(tensor) = self.lookup_tensor(tensor_id) else {
            return self.not_found("tensor not found");
        };
        let Ok(row_index) = row_index.parse::<usize>() else {
            return self.bad_request("invalid row index");
        };
        match tensor.row(row_index) {
            Ok(row) => self.ok(format!("{{\"row\":{}}}", json_u64_array(row))),
            Err(_) => self.not_found("row not found"),
        }
    }

    pub(super) fn tensor_opening(&self, tensor_id: &str, chunk_index: &str) -> RpcResponse {
        let Some(tensor) = self.lookup_tensor(tensor_id) else {
            return self.not_found("tensor not found");
        };
        let Ok(chunk_index) = chunk_index.parse::<u64>() else {
            return self.bad_request("invalid chunk index");
        };
        match tensor.opening(chunk_index, DEFAULT_CHUNK_SIZE) {
            Ok(opening) => self.ok(format!(
                "{{\"tensor_id\":\"{}\",\"chunk_index\":{},\"proof_len\":{}}}",
                hex(&opening.tensor_id),
                opening.chunk_index,
                opening.merkle_proof.siblings.len()
            )),
            Err(_) => self.not_found("opening not found"),
        }
    }

    pub(super) fn tensor_latest(&self) -> RpcResponse {
        let Some((tensor_id, tensor)) = self.tensors.iter().next_back() else {
            return self.not_found("tensor not found");
        };
        self.ok(format!(
            "{{\"tensor_id\":\"{}\",\"tensor_count\":{},\"root\":\"{}\"}}",
            hex(tensor_id),
            self.tensors.len(),
            hex(&tensor.commitment_root())
        ))
    }

    fn lookup_tensor(&self, tensor_id: &str) -> Option<&Tensor> {
        parse_hash(tensor_id)
            .ok()
            .and_then(|tensor_id| self.tensors.get(&tensor_id))
    }
}
