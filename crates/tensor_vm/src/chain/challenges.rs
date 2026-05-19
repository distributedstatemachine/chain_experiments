use super::LocalChain;
use crate::challenge::ChallengeOutcome;
use crate::error::{Result, TvmError};

pub fn apply_outcome(chain: &mut LocalChain, outcome: ChallengeOutcome) -> Result<()> {
    match outcome {
        ChallengeOutcome::Rejected { .. } => Ok(()),
        ChallengeOutcome::ProvenInvalid {
            dishonest_party,
            slash_amount,
            ..
        } => {
            if let Some(miner) = chain.state.miners.get_mut(&dishonest_party) {
                miner.stake = miner.stake.saturating_sub(slash_amount);
                miner.reputation -= 10;
                chain.state.rewards.treasury =
                    chain.state.rewards.treasury.saturating_add(slash_amount);
                return Ok(());
            }
            if let Some(validator) = chain.state.validators.get_mut(&dishonest_party) {
                validator.stake = validator.stake.saturating_sub(slash_amount);
                validator.reputation -= 10;
                chain.state.rewards.treasury =
                    chain.state.rewards.treasury.saturating_add(slash_amount);
                return Ok(());
            }
            Err(TvmError::InvalidReceipt("unknown dishonest party"))
        }
    }
}
