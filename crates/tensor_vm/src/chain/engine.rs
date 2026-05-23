use super::state::{BlockVote, ChainParams, ChainState, JobState, ReceiptState, TensorBlock};
use crate::error::Result;
use crate::types::{Address, Hash};
use crate::verify::ValidatorAttestation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockInvalidReason {
    ConflictingHeight,
    InvalidPayload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockAdmission {
    Applied {
        height: u64,
        hash: Hash,
    },
    Duplicate {
        height: u64,
        hash: Hash,
    },
    PendingParent {
        height: u64,
        parent_hash: Hash,
    },
    Invalid {
        height: u64,
        hash: Hash,
        reason: BlockInvalidReason,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChainCommand {
    RegisterMiner {
        address: Address,
        stake: u64,
    },
    RegisterValidator {
        address: Address,
        stake: u64,
    },
    SubmitJob(JobState),
    SubmitReceipt(ReceiptState),
    SubmitAttestation(ValidatorAttestation),
    SubmitBlock(TensorBlock),
    SubmitBlockVote(BlockVote),
    SettleEpoch {
        miner_reward_pool: u64,
        validator_reward_pool: u64,
    },
    ProduceBlock {
        proposer: Address,
        timestamp: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChainEvent {
    MinerRegistered(Address),
    ValidatorRegistered(Address),
    JobAccepted(Hash),
    ReceiptAccepted(Hash),
    AttestationAccepted {
        receipt_id: Hash,
        validator: Address,
    },
    BlockVoteAccepted {
        block_hash: Hash,
        validator: Address,
    },
    ReceiptSettled(Hash),
    RewardCredited {
        address: Address,
        amount: u64,
    },
    BlockProduced {
        height: u64,
        hash: Hash,
    },
    BlockAccepted {
        height: u64,
        hash: Hash,
    },
    BlockFinalized(Hash),
}

pub trait ChainEngine {
    fn apply_command(&mut self, command: ChainCommand) -> Result<Vec<ChainEvent>>;
    fn view(&self) -> &ChainState;
    fn params(&self) -> &ChainParams;
    fn blocks(&self) -> &[TensorBlock];
}
