use super::{Chain, ChainCommand, ChainEngine, Transaction};
use crate::error::{Result, TvmError};
use crate::types::Address;

pub fn apply(chain: &mut Chain, from: Option<Address>, tx: Transaction) -> Result<()> {
    match tx {
        Transaction::RegisterMiner(address) => {
            let stake = chain.params.miner_min_stake;
            chain
                .apply_command(ChainCommand::RegisterMiner { address, stake })
                .map(|_| ())
        }
        Transaction::RegisterValidator(address) => {
            let stake = chain.params.validator_min_stake;
            chain
                .apply_command(ChainCommand::RegisterValidator { address, stake })
                .map(|_| ())
        }
        Transaction::Transfer { to, amount } => {
            let from = from.ok_or(TvmError::InvalidReceipt("missing sender"))?;
            chain
                .apply_command(ChainCommand::Transfer { from, to, amount })
                .map(|_| ())
        }
        Transaction::ClaimReward(address) => chain
            .apply_command(ChainCommand::ClaimReward(address))
            .map(|_| ()),
        Transaction::SubmitTensorOpReceipt(_)
        | Transaction::SubmitLinearTrainingStepReceipt(_)
        | Transaction::SubmitAttestation(_) => Err(TvmError::InvalidReceipt(
            "reference submissions must enter the transaction pool",
        )),
    }
}
