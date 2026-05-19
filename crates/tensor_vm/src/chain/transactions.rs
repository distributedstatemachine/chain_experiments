use super::{LocalChain, Transaction, accounts};
use crate::error::{Result, TvmError};
use crate::types::Address;

pub fn apply(chain: &mut LocalChain, from: Option<Address>, tx: Transaction) -> Result<()> {
    match tx {
        Transaction::RegisterMiner(address) => {
            chain.register_miner(address, chain.params.miner_min_stake)
        }
        Transaction::RegisterValidator(address) => {
            chain.register_validator(address, chain.params.validator_min_stake)
        }
        Transaction::Transfer { to, amount } => {
            let from = from.ok_or(TvmError::InvalidReceipt("missing sender"))?;
            chain.transfer(from, to, amount)
        }
        Transaction::ClaimReward(address) => accounts::claim_reward(chain, address),
        Transaction::SubmitTensorOpReceipt(_)
        | Transaction::SubmitLinearTrainingStepReceipt(_)
        | Transaction::SubmitAttestation(_) => Ok(()),
    }
}
