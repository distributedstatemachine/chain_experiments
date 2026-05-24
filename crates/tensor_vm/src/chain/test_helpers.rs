use super::{Chain, ReceiptState, RewardState};
use crate::error::{Result, TvmError};
use crate::types::{Address, Hash};
use crate::verify::ValidatorAttestation;

impl Chain {
    pub(crate) fn set_position_for_testing(&mut self, height: u64, epoch: u64) {
        self.state.height = height;
        self.state.epoch = epoch;
    }

    pub(crate) fn mark_receipt_settled_for_testing(&mut self, receipt_id: Hash) {
        self.state.settled_receipts.insert(receipt_id);
    }

    pub(crate) fn mark_receipt_data_unavailable_for_testing(&mut self, receipt_id: Hash) {
        self.state.data_unavailable_receipts.insert(receipt_id);
    }

    pub(crate) fn set_miner_settled_tensor_work_for_testing(
        &mut self,
        miner: Address,
        settled_tensor_work: u64,
    ) -> Result<()> {
        self.state
            .miners
            .get_mut(&miner)
            .ok_or(TvmError::UnknownMiner)?
            .settled_tensor_work = settled_tensor_work;
        Ok(())
    }

    pub(crate) fn set_miner_tensor_work_for_testing(
        &mut self,
        miner: Address,
        settled_tensor_work: u64,
        pending_tensor_work: u64,
    ) -> Result<()> {
        let miner = self
            .state
            .miners
            .get_mut(&miner)
            .ok_or(TvmError::UnknownMiner)?;
        miner.settled_tensor_work = settled_tensor_work;
        miner.pending_tensor_work = pending_tensor_work;
        Ok(())
    }

    pub(crate) fn set_validator_stake_for_testing(
        &mut self,
        validator: Address,
        stake: u64,
    ) -> Result<()> {
        self.state
            .validators
            .get_mut(&validator)
            .ok_or(TvmError::UnknownValidator)?
            .stake = stake;
        Ok(())
    }

    pub(crate) fn insert_receipt_for_testing(&mut self, receipt: ReceiptState) {
        self.state.receipts.insert(receipt.receipt_id(), receipt);
    }

    pub(crate) fn insert_attestation_for_testing(&mut self, attestation: ValidatorAttestation) {
        self.state
            .attestations
            .entry(attestation.receipt_id)
            .or_default()
            .push(attestation);
    }

    pub(crate) fn set_model_optimizer_state_root_for_testing(
        &mut self,
        model_id: Hash,
        optimizer_state_root: Option<Hash>,
    ) -> Result<()> {
        self.state
            .model_states
            .get_mut(&model_id)
            .ok_or(TvmError::InvalidReceipt("unknown model"))?
            .optimizer_state_root = optimizer_state_root;
        Ok(())
    }

    pub(crate) fn remove_job_for_testing(&mut self, job_id: &Hash) {
        self.state.jobs.remove(job_id);
    }

    pub(crate) fn remove_receipt_for_testing(&mut self, receipt_id: &Hash) {
        self.state.receipts.remove(receipt_id);
    }

    pub(crate) fn remove_attestations_for_testing(&mut self, receipt_id: &Hash) {
        self.state.attestations.remove(receipt_id);
    }

    pub(crate) fn set_reward_treasury_for_testing(&mut self, treasury: u64) {
        self.state.rewards =
            RewardState::from_parts(self.state.rewards.balances().clone(), treasury);
    }
}
