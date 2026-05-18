use crate::chain::LocalChain;
use crate::hash::hex;
use crate::types::Address;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerSummary {
    pub height: u64,
    pub epoch: u64,
    pub block_count: usize,
    pub miner_count: usize,
    pub validator_count: usize,
    pub receipt_count: usize,
    pub settled_receipt_count: usize,
}

impl ExplorerSummary {
    pub fn from_chain(chain: &LocalChain) -> Self {
        Self {
            height: chain.state.height,
            epoch: chain.state.epoch,
            block_count: chain.blocks.len(),
            miner_count: chain.state.miners.len(),
            validator_count: chain.state.validators.len(),
            receipt_count: chain.state.receipts.len(),
            settled_receipt_count: chain.state.settled_receipts.len(),
        }
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\"height\":{},\"epoch\":{},\"block_count\":{},\"miner_count\":{},\"validator_count\":{},\"receipt_count\":{},\"settled_receipt_count\":{}}}",
            self.height,
            self.epoch,
            self.block_count,
            self.miner_count,
            self.validator_count,
            self.receipt_count,
            self.settled_receipt_count
        )
    }
}

pub fn account_page(chain: &LocalChain, address: &Address) -> String {
    let miner = chain.state.miners.get(address);
    let validator = chain.state.validators.get(address);
    let balance = chain.state.rewards.balance(address);
    format!(
        "{{\"address\":\"{}\",\"is_miner\":{},\"is_validator\":{},\"balance\":{}}}",
        hex(address),
        miner.is_some(),
        validator.is_some(),
        balance
    )
}

pub fn latest_blocks(chain: &LocalChain, limit: usize) -> Vec<String> {
    chain
        .blocks
        .iter()
        .rev()
        .take(limit)
        .map(|block| {
            format!(
                "{{\"height\":{},\"epoch\":{},\"hash\":\"{}\"}}",
                block.height,
                block.epoch,
                hex(&block.hash())
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::LocalChain;
    use crate::types::{address, hash_bytes};

    #[test]
    fn explorer_summarizes_chain_and_accounts() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let miner = address(b"miner");
        let validator = address(b"validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();
        chain.produce_block(miner, 1);
        chain.state.rewards.credit(miner, 77);

        let summary = ExplorerSummary::from_chain(&chain);
        assert_eq!(summary.block_count, 1);
        assert!(summary.to_json().contains("\"miner_count\":1"));
        assert!(account_page(&chain, &miner).contains("\"balance\":77"));
        assert_eq!(latest_blocks(&chain, 10).len(), 1);
    }
}
