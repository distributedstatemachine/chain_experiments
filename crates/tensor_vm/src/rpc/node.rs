use crate::chain::Chain;
use crate::error::Result;
use crate::faucet::Faucet;
use crate::localnet::{
    produce_synthetic_cpu_round_with_profile, produce_synthetic_cpu_round_with_tensors,
};
use crate::profile::ChainProfile;
use crate::tensor::Tensor;
use crate::txpool::TxPool;
use crate::types::Hash;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct RpcNode {
    pub chain: Chain,
    pub txpool: TxPool,
    pub faucet: Option<Faucet>,
    pub(super) tensors: BTreeMap<Hash, Tensor>,
}

impl RpcNode {
    pub fn new(chain: Chain) -> Self {
        Self {
            chain,
            txpool: TxPool::default(),
            faucet: None,
            tensors: BTreeMap::new(),
        }
    }

    pub fn with_faucet(chain: Chain, faucet: Faucet) -> Self {
        Self {
            chain,
            txpool: TxPool::default(),
            faucet: Some(faucet),
            tensors: BTreeMap::new(),
        }
    }

    pub fn insert_tensor(&mut self, tensor: Tensor) -> Hash {
        let id = tensor.tensor_id();
        self.tensors.insert(id, tensor);
        id
    }

    pub fn tensor_by_commitment_root(&self, commitment_root: &Hash) -> Option<&Tensor> {
        self.tensors
            .values()
            .find(|tensor| tensor.commitment_root() == *commitment_root)
    }

    pub fn contains_tensor_commitment_root(&self, commitment_root: &Hash) -> bool {
        self.tensor_by_commitment_root(commitment_root).is_some()
    }

    pub fn produce_synthetic_cpu_round(&mut self) -> Result<Option<u64>> {
        let Some(round) = produce_synthetic_cpu_round_with_tensors(&mut self.chain)? else {
            return Ok(None);
        };
        for tensor in round.tensors {
            self.insert_tensor(tensor);
        }
        Ok(Some(round.height))
    }

    pub fn produce_synthetic_cpu_round_with_profile(
        &mut self,
        profile: &ChainProfile,
    ) -> Result<Option<u64>> {
        let Some(round) = produce_synthetic_cpu_round_with_profile(&mut self.chain, profile)?
        else {
            return Ok(None);
        };
        for tensor in round.tensors {
            self.insert_tensor(tensor);
        }
        Ok(Some(round.height))
    }
}
