use super::{Chain, ChainCommand, ChainEngine, ChainEvent, Transaction};
use crate::error::{Result, TvmError};
use crate::types::Address;

pub fn apply(chain: &mut Chain, from: Option<Address>, tx: Transaction) -> Result<Vec<ChainEvent>> {
    match tx {
        Transaction::RegisterMiner(address) => {
            let stake = chain.params.miner_min_stake;
            chain.apply_command(ChainCommand::RegisterMiner { address, stake })
        }
        Transaction::RegisterValidator(address) => {
            let stake = chain.params.validator_min_stake;
            chain.apply_command(ChainCommand::RegisterValidator { address, stake })
        }
        Transaction::Transfer { to, amount } => {
            let from = from.ok_or(TvmError::InvalidReceipt("missing sender"))?;
            chain.apply_command(ChainCommand::Transfer { from, to, amount })
        }
        Transaction::ClaimReward(address) => {
            chain.apply_command(ChainCommand::ClaimReward(address))
        }
        Transaction::SubmitTensorOpReceipt(_)
        | Transaction::SubmitLinearTrainingStepReceipt(_)
        | Transaction::SubmitAttestation(_) => Err(TvmError::InvalidReceipt(
            "reference submissions must enter the transaction pool",
        )),
    }
}
