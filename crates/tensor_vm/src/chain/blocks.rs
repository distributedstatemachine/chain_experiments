use super::roots::{attestation_root, job_root, receipt_root, reward_root};
use super::{LocalChain, TensorBlock};
use crate::types::{Address, hash_bytes, sign};

pub(super) fn produce(chain: &mut LocalChain, proposer: Address, timestamp: u64) -> TensorBlock {
    let parent_hash = chain
        .blocks
        .last()
        .map(TensorBlock::hash)
        .unwrap_or([0; 32]);
    let job_root = job_root(&chain.state.jobs);
    let receipt_root = receipt_root(&chain.state.receipts);
    let attestation_root = attestation_root(&chain.state.attestations);
    let state_root = chain.state_root();
    let reward_root = reward_root(&chain.state.rewards);
    let randomness = hash_bytes(
        b"tensor-vm-next-randomness-v1",
        &[
            &chain.state.finalized_randomness,
            &parent_hash,
            &chain.state.height.to_le_bytes(),
        ],
    );
    let mut block = TensorBlock {
        height: chain.state.height,
        parent_hash,
        epoch: chain.state.epoch,
        proposer,
        job_root,
        receipt_root,
        attestation_root,
        state_root,
        reward_root,
        randomness,
        timestamp,
        proposer_signature: [0; 32],
        validator_signature_aggregate: [0; 32],
    };
    let block_hash = block.hash();
    block.proposer_signature = sign(&proposer, &block_hash);
    block.validator_signature_aggregate =
        hash_bytes(b"tensor-vm-validator-aggregate-v1", &[&block_hash]);
    chain.blocks.push(block.clone());
    chain.state.height += 1;
    chain.state.epoch = chain.state.height / chain.params.epoch_length.max(1);
    chain.state.finalized_randomness = randomness;
    block
}

pub(super) fn produce_with_rewards(
    chain: &mut LocalChain,
    proposer: Address,
    timestamp: u64,
    fixed_block_reward: u64,
    fee_share: u64,
) -> TensorBlock {
    let proposer_reward = fixed_block_reward.saturating_add(fee_share);
    if proposer_reward > 0 {
        chain.state.rewards.credit(proposer, proposer_reward);
    }
    produce(chain, proposer, timestamp)
}
