use crate::chain::{Chain, ChainCommand, ChainEngine, TensorBlock};
use crate::types::Address;

pub(super) fn register_block_producer(chain: &mut Chain, producer: Address) {
    chain
        .apply_command(ChainCommand::RegisterMiner {
            address: producer,
            stake: chain.params().miner_min_stake,
        })
        .unwrap();
    chain
        .apply_command(ChainCommand::RegisterValidator {
            address: producer,
            stake: chain.params().validator_min_stake,
        })
        .unwrap();
}

pub(super) fn produce_block(chain: &mut Chain, proposer: Address, timestamp: u64) -> TensorBlock {
    let block_count = chain.blocks().len();
    chain
        .apply_command(ChainCommand::ProduceBlock {
            proposer,
            timestamp,
        })
        .unwrap();
    assert_eq!(chain.blocks().len(), block_count + 1);
    chain.blocks().last().unwrap().clone()
}
