use crate::challenge::ChallengeOutcome;
use crate::error::{Result, TvmError};
use crate::jobs::{
    LinearTrainingStepJob, LinearTrainingStepReceipt, MatmulJob, PrimitiveType, TensorOpReceipt,
};
use crate::types::{Address, Hash, Signature, hash_bytes, hash_to_u128, sign, verify_signature};
use crate::verify::{FreivaldsParams, ValidatorAttestation, VerificationResult};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainParams {
    pub block_time_seconds: u64,
    pub epoch_length: u64,
    pub receipt_submission_window: u64,
    pub verification_window: u64,
    pub reward_settlement_delay_epochs: u64,
    pub challenge_window_epochs: u64,
    pub replication_factor: usize,
    pub agreement_quorum: usize,
    pub finality_stake_numerator: u64,
    pub finality_stake_denominator: u64,
    pub miner_reward_bps: u64,
    pub validator_reward_bps: u64,
    pub proposer_reward_bps: u64,
    pub treasury_reward_bps: u64,
    pub miner_min_stake: u64,
    pub validator_min_stake: u64,
    pub freivalds: FreivaldsParams,
}

impl Default for ChainParams {
    fn default() -> Self {
        Self {
            block_time_seconds: 6,
            epoch_length: 100,
            receipt_submission_window: 20,
            verification_window: 40,
            reward_settlement_delay_epochs: 1,
            challenge_window_epochs: 1,
            replication_factor: 5,
            agreement_quorum: 3,
            finality_stake_numerator: 2,
            finality_stake_denominator: 3,
            miner_reward_bps: 7_000,
            validator_reward_bps: 2_000,
            proposer_reward_bps: 500,
            treasury_reward_bps: 500,
            miner_min_stake: 100,
            validator_min_stake: 10_000,
            freivalds: FreivaldsParams::default(),
        }
    }
}

impl ChainParams {
    pub fn tensor_retention_window_blocks(&self) -> u64 {
        self.reward_settlement_delay_epochs
            .saturating_add(self.challenge_window_epochs)
            .saturating_mul(self.epoch_length.max(1))
    }

    pub fn tensor_retention_deadline(&self, submitted_at_block: u64) -> u64 {
        submitted_at_block.saturating_add(self.tensor_retention_window_blocks())
    }

    pub fn reward_allocation(&self, total_emission: u64) -> RewardAllocation {
        let miner_reward_pool = reward_share(total_emission, self.miner_reward_bps);
        let validator_reward_pool = reward_share(total_emission, self.validator_reward_bps);
        let proposer_reward = reward_share(total_emission, self.proposer_reward_bps);
        let explicit_treasury = reward_share(total_emission, self.treasury_reward_bps);
        let allocated = miner_reward_pool
            .saturating_add(validator_reward_pool)
            .saturating_add(proposer_reward)
            .saturating_add(explicit_treasury);
        RewardAllocation {
            miner_reward_pool,
            validator_reward_pool,
            proposer_reward,
            treasury_reward: explicit_treasury
                .saturating_add(total_emission.saturating_sub(allocated)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RewardAllocation {
    pub miner_reward_pool: u64,
    pub validator_reward_pool: u64,
    pub proposer_reward: u64,
    pub treasury_reward: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum HardwareClass {
    Cpu,
    ConsumerGpu,
    DatacenterGpu,
    Other,
}

impl HardwareClass {
    pub fn is_gpu(self) -> bool {
        matches!(self, Self::ConsumerGpu | Self::DatacenterGpu)
    }

    pub fn tag(self) -> u8 {
        match self {
            Self::Cpu => 1,
            Self::ConsumerGpu => 2,
            Self::DatacenterGpu => 3,
            Self::Other => 4,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinerState {
    pub address: Address,
    pub operator_id: Hash,
    pub stake: u64,
    pub reputation: i64,
    pub settled_tensor_work: u64,
    pub pending_tensor_work: u64,
    pub hardware_class: HardwareClass,
    pub gpu_utilization_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorState {
    pub address: Address,
    pub stake: u64,
    pub reputation: i64,
    pub valid_attestations: u64,
    pub missed_assignments: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct AccountState {
    pub address: Address,
    pub balance: u64,
    pub nonce: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct RewardState {
    pub balances: BTreeMap<Address, u64>,
    pub treasury: u64,
}

impl RewardState {
    pub fn credit(&mut self, address: Address, amount: u64) {
        *self.balances.entry(address).or_default() += amount;
    }

    pub fn balance(&self, address: &Address) -> u64 {
        self.balances.get(address).copied().unwrap_or(0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JobState {
    TensorOp(MatmulJob),
    LinearTrainingStep(LinearTrainingStepJob),
}

impl JobState {
    pub fn job_id(&self) -> Hash {
        match self {
            Self::TensorOp(job) => job.job_id,
            Self::LinearTrainingStep(job) => job.job_id,
        }
    }

    pub fn deadline_block(&self) -> u64 {
        match self {
            Self::TensorOp(job) => job.deadline_block,
            Self::LinearTrainingStep(job) => job.deadline_block,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReceiptState {
    TensorOp(TensorOpReceipt),
    LinearTrainingStep(LinearTrainingStepReceipt),
}

impl ReceiptState {
    pub fn receipt_id(&self) -> Hash {
        match self {
            Self::TensorOp(receipt) => receipt.receipt_id,
            Self::LinearTrainingStep(receipt) => receipt.receipt_id,
        }
    }

    pub fn job_id(&self) -> Hash {
        match self {
            Self::TensorOp(receipt) => receipt.job_id,
            Self::LinearTrainingStep(receipt) => receipt.job_id,
        }
    }

    pub fn miner(&self) -> Address {
        match self {
            Self::TensorOp(receipt) => receipt.miner,
            Self::LinearTrainingStep(receipt) => receipt.miner,
        }
    }

    pub fn primitive_type(&self) -> PrimitiveType {
        match self {
            Self::TensorOp(_) => PrimitiveType::TensorOp,
            Self::LinearTrainingStep(_) => PrimitiveType::LinearTrainingStep,
        }
    }

    pub fn tensor_work_units(&self) -> u64 {
        match self {
            Self::TensorOp(receipt) => receipt.tensor_work_units,
            Self::LinearTrainingStep(receipt) => receipt.tensor_work_units,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelState {
    pub model_id: Hash,
    pub architecture_hash: Hash,
    pub weight_root: Hash,
    pub optimizer_state_root: Option<Hash>,
    pub step: u64,
    pub config_hash: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Transaction {
    RegisterMiner(Address),
    RegisterValidator(Address),
    SubmitTensorOpReceipt(Hash),
    SubmitLinearTrainingStepReceipt(Hash),
    SubmitAttestation(Hash),
    Transfer { to: Address, amount: u64 },
    ClaimReward(Address),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TensorBlock {
    pub height: u64,
    pub parent_hash: Hash,
    pub epoch: u64,
    pub proposer: Address,
    pub job_root: Hash,
    pub receipt_root: Hash,
    pub attestation_root: Hash,
    pub state_root: Hash,
    pub reward_root: Hash,
    pub randomness: Hash,
    pub timestamp: u64,
    pub proposer_signature: Signature,
    pub validator_signature_aggregate: Signature,
}

impl TensorBlock {
    pub fn hash(&self) -> Hash {
        hash_bytes(
            b"tensor-vm-block-v1",
            &[
                &self.height.to_le_bytes(),
                &self.parent_hash,
                &self.epoch.to_le_bytes(),
                &self.proposer,
                &self.job_root,
                &self.receipt_root,
                &self.attestation_root,
                &self.state_root,
                &self.reward_root,
                &self.randomness,
                &self.timestamp.to_le_bytes(),
            ],
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockVote {
    pub validator: Address,
    pub block_hash: Hash,
    pub block_height: u64,
    pub stake: u64,
    pub signature: Signature,
}

impl BlockVote {
    pub fn new(validator: Address, stake: u64, block: &TensorBlock) -> Self {
        let block_hash = block.hash();
        let message = Self::message_hash(&block_hash, block.height, stake);
        Self {
            validator,
            block_hash,
            block_height: block.height,
            stake,
            signature: sign(&validator, &message),
        }
    }

    pub fn verify_signature(&self) -> bool {
        verify_signature(
            &self.validator,
            &Self::message_hash(&self.block_hash, self.block_height, self.stake),
            &self.signature,
        )
    }

    fn message_hash(block_hash: &Hash, block_height: u64, stake: u64) -> Hash {
        hash_bytes(
            b"tensor-vm-block-vote-v1",
            &[
                block_hash,
                &block_height.to_le_bytes(),
                &stake.to_le_bytes(),
            ],
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainState {
    pub height: u64,
    pub epoch: u64,
    pub finalized_randomness: Hash,
    pub accounts: BTreeMap<Address, AccountState>,
    pub miners: BTreeMap<Address, MinerState>,
    pub validators: BTreeMap<Address, ValidatorState>,
    pub jobs: BTreeMap<Hash, JobState>,
    pub receipts: BTreeMap<Hash, ReceiptState>,
    pub attestations: BTreeMap<Hash, Vec<ValidatorAttestation>>,
    pub block_votes: BTreeMap<Hash, Vec<BlockVote>>,
    pub finalized_blocks: BTreeSet<Hash>,
    pub data_unavailable_receipts: BTreeSet<Hash>,
    pub settled_receipts: BTreeSet<Hash>,
    pub model_states: BTreeMap<Hash, ModelState>,
    pub rewards: RewardState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalChain {
    pub params: ChainParams,
    pub state: ChainState,
    pub blocks: Vec<TensorBlock>,
}

impl LocalChain {
    pub fn new(finalized_randomness: Hash) -> Self {
        Self::with_params(ChainParams::default(), finalized_randomness)
    }

    pub fn with_params(params: ChainParams, finalized_randomness: Hash) -> Self {
        Self {
            params,
            state: ChainState {
                height: 0,
                epoch: 0,
                finalized_randomness,
                accounts: BTreeMap::new(),
                miners: BTreeMap::new(),
                validators: BTreeMap::new(),
                jobs: BTreeMap::new(),
                receipts: BTreeMap::new(),
                attestations: BTreeMap::new(),
                block_votes: BTreeMap::new(),
                finalized_blocks: BTreeSet::new(),
                data_unavailable_receipts: BTreeSet::new(),
                settled_receipts: BTreeSet::new(),
                model_states: BTreeMap::new(),
                rewards: RewardState::default(),
            },
            blocks: Vec::new(),
        }
    }

    pub fn register_miner(&mut self, address: Address, stake: u64) -> Result<()> {
        self.register_miner_with_profile_and_operator(
            address,
            stake,
            address,
            HardwareClass::Cpu,
            0,
        )
    }

    pub fn register_miner_with_operator(
        &mut self,
        address: Address,
        stake: u64,
        operator_id: Hash,
    ) -> Result<()> {
        self.register_miner_with_profile_and_operator(
            address,
            stake,
            operator_id,
            HardwareClass::Cpu,
            0,
        )
    }

    pub fn register_miner_with_profile(
        &mut self,
        address: Address,
        stake: u64,
        hardware_class: HardwareClass,
        gpu_utilization_bps: u64,
    ) -> Result<()> {
        self.register_miner_with_profile_and_operator(
            address,
            stake,
            address,
            hardware_class,
            gpu_utilization_bps,
        )
    }

    pub fn register_miner_with_profile_and_operator(
        &mut self,
        address: Address,
        stake: u64,
        operator_id: Hash,
        hardware_class: HardwareClass,
        gpu_utilization_bps: u64,
    ) -> Result<()> {
        if stake < self.params.miner_min_stake {
            return Err(TvmError::InsufficientStake);
        }
        if gpu_utilization_bps > 10_000 {
            return Err(TvmError::InvalidReceipt("gpu utilization exceeds 100%"));
        }
        if !hardware_class.is_gpu() && gpu_utilization_bps != 0 {
            return Err(TvmError::InvalidReceipt(
                "non-gpu miner cannot report gpu utilization",
            ));
        }
        if self.state.miners.contains_key(&address) {
            return Err(TvmError::InvalidReceipt("miner already registered"));
        }
        self.ensure_account(address);
        self.state.miners.insert(
            address,
            MinerState {
                address,
                operator_id,
                stake,
                reputation: 0,
                settled_tensor_work: 0,
                pending_tensor_work: 0,
                hardware_class,
                gpu_utilization_bps,
            },
        );
        Ok(())
    }

    pub fn register_validator(&mut self, address: Address, stake: u64) -> Result<()> {
        if stake < self.params.validator_min_stake {
            return Err(TvmError::InsufficientStake);
        }
        if self.state.validators.contains_key(&address) {
            return Err(TvmError::InvalidReceipt("validator already registered"));
        }
        self.ensure_account(address);
        self.state.validators.insert(
            address,
            ValidatorState {
                address,
                stake,
                reputation: 0,
                valid_attestations: 0,
                missed_assignments: 0,
            },
        );
        Ok(())
    }

    pub fn credit_account(&mut self, address: Address, amount: u64) {
        let account = self.ensure_account(address);
        account.balance = account.balance.saturating_add(amount);
    }

    pub fn transfer(&mut self, from: Address, to: Address, amount: u64) -> Result<()> {
        let from_account = self.ensure_account(from);
        if from_account.balance < amount {
            return Err(TvmError::InvalidReceipt("insufficient account balance"));
        }
        from_account.balance -= amount;
        from_account.nonce += 1;
        let to_account = self.ensure_account(to);
        to_account.balance = to_account.balance.saturating_add(amount);
        Ok(())
    }

    pub fn submit_job(&mut self, job: JobState) {
        self.state.jobs.insert(job.job_id(), job);
    }

    pub fn job(&self, job_id: &Hash) -> Option<&JobState> {
        self.state.jobs.get(job_id)
    }

    pub fn submit_tensor_op_receipt(&mut self, receipt: TensorOpReceipt) -> Result<()> {
        if !self.state.miners.contains_key(&receipt.miner) {
            return Err(TvmError::UnknownMiner);
        }
        if !self.state.jobs.contains_key(&receipt.job_id) {
            return Err(TvmError::InvalidReceipt("unknown job"));
        }
        if self.state.receipts.contains_key(&receipt.receipt_id) {
            return Err(TvmError::InvalidReceipt("duplicate receipt"));
        }
        self.state
            .receipts
            .insert(receipt.receipt_id, ReceiptState::TensorOp(receipt));
        Ok(())
    }

    pub fn submit_linear_receipt(&mut self, receipt: LinearTrainingStepReceipt) -> Result<()> {
        if !self.state.miners.contains_key(&receipt.miner) {
            return Err(TvmError::UnknownMiner);
        }
        if !self.state.jobs.contains_key(&receipt.job_id) {
            return Err(TvmError::InvalidReceipt("unknown job"));
        }
        if self.state.receipts.contains_key(&receipt.receipt_id) {
            return Err(TvmError::InvalidReceipt("duplicate receipt"));
        }
        self.state.receipts.insert(
            receipt.receipt_id,
            ReceiptState::LinearTrainingStep(receipt),
        );
        Ok(())
    }

    pub fn apply_transaction(&mut self, from: Option<Address>, tx: Transaction) -> Result<()> {
        match tx {
            Transaction::RegisterMiner(address) => {
                self.register_miner(address, self.params.miner_min_stake)
            }
            Transaction::RegisterValidator(address) => {
                self.register_validator(address, self.params.validator_min_stake)
            }
            Transaction::Transfer { to, amount } => {
                let from = from.ok_or(TvmError::InvalidReceipt("missing sender"))?;
                self.transfer(from, to, amount)
            }
            Transaction::ClaimReward(address) => {
                let reward = self.state.rewards.balance(&address);
                if reward == 0 {
                    return Err(TvmError::InvalidReceipt("no reward to claim"));
                }
                self.credit_account(address, reward);
                self.state.rewards.balances.insert(address, 0);
                Ok(())
            }
            Transaction::SubmitTensorOpReceipt(_)
            | Transaction::SubmitLinearTrainingStepReceipt(_)
            | Transaction::SubmitAttestation(_) => Ok(()),
        }
    }

    pub fn submit_attestation(&mut self, attestation: ValidatorAttestation) -> Result<()> {
        let validator_stake = self
            .state
            .validators
            .get(&attestation.validator)
            .ok_or(TvmError::UnknownValidator)?
            .stake;
        if attestation.stake != validator_stake {
            return Err(TvmError::InvalidReceipt("attestation stake mismatch"));
        }
        if !attestation.verify_signature() {
            return Err(TvmError::InvalidReceipt("bad attestation signature"));
        }
        let (receipt_job_id, receipt_primitive_type, receipt_miner) = {
            let receipt = self
                .state
                .receipts
                .get(&attestation.receipt_id)
                .ok_or(TvmError::UnknownReceipt)?;
            (receipt.job_id(), receipt.primitive_type(), receipt.miner())
        };
        if attestation.job_id != receipt_job_id
            || attestation.primitive_type != receipt_primitive_type
        {
            if let Some(validator) = self.state.validators.get_mut(&attestation.validator) {
                validator.reputation -= 1;
            }
            return Err(TvmError::InvalidReceipt("attestation receipt mismatch"));
        }
        if self
            .state
            .attestations
            .get(&attestation.receipt_id)
            .is_some_and(|items| {
                items
                    .iter()
                    .any(|existing| existing.validator == attestation.validator)
            })
        {
            return Err(TvmError::InvalidReceipt("duplicate validator attestation"));
        }
        if attestation.result == VerificationResult::Valid
            && let Some(validator) = self.state.validators.get_mut(&attestation.validator)
        {
            validator.valid_attestations += 1;
        }
        if (attestation.result == VerificationResult::Unavailable
            || !attestation.data_availability_passed)
            && self
                .state
                .data_unavailable_receipts
                .insert(attestation.receipt_id)
            && let Some(miner) = self.state.miners.get_mut(&receipt_miner)
        {
            miner.reputation -= 1;
        }
        self.state
            .attestations
            .entry(attestation.receipt_id)
            .or_default()
            .push(attestation);
        Ok(())
    }

    pub fn has_attestation_quorum(&self, receipt_id: &Hash) -> bool {
        let attestations = match self.state.attestations.get(receipt_id) {
            Some(attestations) => attestations,
            None => return false,
        };
        let receipt = match self.state.receipts.get(receipt_id) {
            Some(receipt) => receipt,
            None => return false,
        };
        let mut valid_count = 0_usize;
        let mut valid_stake = 0_u64;
        let mut seen_validators = BTreeSet::new();
        let assigned_stake: u64 = self
            .state
            .validators
            .values()
            .map(|validator| validator.stake)
            .sum();
        for attestation in attestations {
            if !seen_validators.insert(attestation.validator) {
                continue;
            }
            if attestation.result == VerificationResult::Valid
                && attestation.data_availability_passed
                && attestation.verify_signature()
                && attestation.job_id == receipt.job_id()
                && attestation.primitive_type == receipt.primitive_type()
            {
                valid_count += 1;
                valid_stake = valid_stake.saturating_add(attestation.stake);
            }
        }
        let stake_num = self.params.freivalds.minimum_stake_numerator;
        let stake_den = self.params.freivalds.minimum_stake_denominator.max(1);
        valid_count >= self.params.freivalds.minimum_validators
            && valid_stake.saturating_mul(stake_den) >= assigned_stake.saturating_mul(stake_num)
    }

    pub fn redundant_agreement_count(&self, receipt_id: &Hash) -> usize {
        let Some(receipt) = self.state.receipts.get(receipt_id) else {
            return 0;
        };
        let mut agreeing_miners = BTreeSet::new();
        for (other_id, other) in &self.state.receipts {
            if self.has_attestation_quorum(other_id) && receipts_agree(receipt, other) {
                agreeing_miners.insert(other.miner());
            }
        }
        agreeing_miners.len()
    }

    pub fn has_redundant_agreement(&self, receipt_id: &Hash) -> bool {
        if !self.state.receipts.contains_key(receipt_id) {
            return false;
        }
        if self.params.agreement_quorum <= 1 {
            return true;
        }
        self.redundant_agreement_count(receipt_id) >= self.params.agreement_quorum
    }

    pub fn submit_block_vote(&mut self, vote: BlockVote) -> Result<()> {
        let validator = self
            .state
            .validators
            .get(&vote.validator)
            .ok_or(TvmError::UnknownValidator)?;
        if validator.stake != vote.stake {
            return Err(TvmError::InvalidReceipt("block vote stake mismatch"));
        }
        if !vote.verify_signature() {
            return Err(TvmError::InvalidReceipt("bad block vote signature"));
        }
        if !self
            .blocks
            .iter()
            .any(|block| block.height == vote.block_height && block.hash() == vote.block_hash)
        {
            return Err(TvmError::InvalidReceipt("unknown block"));
        }
        if self
            .state
            .block_votes
            .get(&vote.block_hash)
            .is_some_and(|votes| {
                votes
                    .iter()
                    .any(|existing| existing.validator == vote.validator)
            })
        {
            return Err(TvmError::InvalidReceipt("duplicate block vote"));
        }

        let block_hash = vote.block_hash;
        self.state
            .block_votes
            .entry(block_hash)
            .or_default()
            .push(vote);
        if self.has_block_finality(&block_hash) {
            self.state.finalized_blocks.insert(block_hash);
        }
        Ok(())
    }

    pub fn has_block_finality(&self, block_hash: &Hash) -> bool {
        let total_stake: u64 = self
            .state
            .validators
            .values()
            .map(|validator| validator.stake)
            .sum();
        if total_stake == 0 {
            return false;
        }
        let mut seen_validators = BTreeSet::new();
        let mut signed_stake = 0_u64;
        for vote in self.state.block_votes.get(block_hash).into_iter().flatten() {
            let Some(validator) = self.state.validators.get(&vote.validator) else {
                continue;
            };
            if validator.stake != vote.stake {
                continue;
            }
            if !seen_validators.insert(vote.validator) {
                continue;
            }
            if vote.verify_signature() {
                signed_stake = signed_stake.saturating_add(vote.stake);
            }
        }
        let numerator = self.params.finality_stake_numerator;
        let denominator = self.params.finality_stake_denominator.max(1);
        signed_stake.saturating_mul(denominator) >= total_stake.saturating_mul(numerator)
    }

    pub fn is_block_finalized(&self, block_hash: &Hash) -> bool {
        self.state.finalized_blocks.contains(block_hash)
    }

    pub fn register_model(
        &mut self,
        model_id: Hash,
        architecture_hash: Hash,
        weight_root: Hash,
        config_hash: Hash,
    ) {
        self.state.model_states.insert(
            model_id,
            ModelState {
                model_id,
                architecture_hash,
                weight_root,
                optimizer_state_root: None,
                step: 0,
                config_hash,
            },
        );
    }

    pub fn apply_model_transition(
        &mut self,
        model_id: &Hash,
        step: u64,
        weight_root_before: &Hash,
        weight_root_after: Hash,
    ) -> Result<()> {
        let model = self
            .state
            .model_states
            .get_mut(model_id)
            .ok_or(TvmError::InvalidReceipt("unknown model"))?;
        if model.step != step {
            return Err(TvmError::InvalidReceipt("model step mismatch"));
        }
        if &model.weight_root != weight_root_before {
            return Err(TvmError::InvalidReceipt("model weight root mismatch"));
        }
        model.weight_root = weight_root_after;
        model.step += 1;
        Ok(())
    }

    pub fn apply_challenge_outcome(&mut self, outcome: ChallengeOutcome) -> Result<()> {
        match outcome {
            ChallengeOutcome::Rejected { .. } => Ok(()),
            ChallengeOutcome::ProvenInvalid {
                dishonest_party,
                slash_amount,
                ..
            } => {
                if let Some(miner) = self.state.miners.get_mut(&dishonest_party) {
                    miner.stake = miner.stake.saturating_sub(slash_amount);
                    miner.reputation -= 10;
                    self.state.rewards.treasury =
                        self.state.rewards.treasury.saturating_add(slash_amount);
                    return Ok(());
                }
                if let Some(validator) = self.state.validators.get_mut(&dishonest_party) {
                    validator.stake = validator.stake.saturating_sub(slash_amount);
                    validator.reputation -= 10;
                    self.state.rewards.treasury =
                        self.state.rewards.treasury.saturating_add(slash_amount);
                    return Ok(());
                }
                Err(TvmError::InvalidReceipt("unknown dishonest party"))
            }
        }
    }

    pub fn validation_seed(&self, receipt_id: &Hash) -> Hash {
        hash_bytes(
            b"tensor-vm-validation-seed-v1",
            &[&self.state.finalized_randomness, receipt_id],
        )
    }

    pub fn settle_epoch(&mut self, miner_reward_pool: u64, validator_reward_pool: u64) {
        let mut newly_settled = Vec::new();
        for (receipt_id, receipt) in &self.state.receipts {
            if self.state.settled_receipts.contains(receipt_id) {
                continue;
            }
            if self.has_attestation_quorum(receipt_id) {
                if !self.has_redundant_agreement(receipt_id) {
                    continue;
                }
                if let ReceiptState::LinearTrainingStep(receipt) = receipt
                    && self.has_conflicting_linear_receipt(*receipt_id, receipt)
                {
                    continue;
                }
                newly_settled.push((*receipt_id, receipt.clone()));
            }
        }

        let total_work: u64 = newly_settled
            .iter()
            .map(|(_, receipt)| receipt.tensor_work_units())
            .sum();
        let newly_settled_ids: BTreeSet<Hash> = newly_settled
            .iter()
            .map(|(receipt_id, _)| *receipt_id)
            .collect();
        for (receipt_id, receipt) in newly_settled {
            self.state.settled_receipts.insert(receipt_id);
            if let Some(miner) = self.state.miners.get_mut(&receipt.miner()) {
                miner.pending_tensor_work = miner
                    .pending_tensor_work
                    .saturating_add(receipt.tensor_work_units());
                miner.settled_tensor_work = miner
                    .settled_tensor_work
                    .saturating_add(receipt.tensor_work_units());
                if total_work > 0 {
                    let reward =
                        miner_reward_pool.saturating_mul(receipt.tensor_work_units()) / total_work;
                    self.state.rewards.credit(miner.address, reward);
                }
            }
        }

        let valid_attestations: Vec<_> = self
            .state
            .attestations
            .iter()
            .filter(|(receipt_id, _)| newly_settled_ids.contains(*receipt_id))
            .flat_map(|(_, items)| items.iter())
            .filter(|att| att.result == VerificationResult::Valid && att.data_availability_passed)
            .cloned()
            .collect();
        let total_valid = valid_attestations.len() as u64;
        if total_valid > 0 {
            for attestation in valid_attestations {
                self.state
                    .rewards
                    .credit(attestation.validator, validator_reward_pool / total_valid);
            }
        }
    }

    pub fn settle_epoch_rewards(&mut self, allocation: RewardAllocation, proposer: Address) {
        self.settle_epoch(
            allocation.miner_reward_pool,
            allocation.validator_reward_pool,
        );
        if allocation.proposer_reward > 0 {
            self.state
                .rewards
                .credit(proposer, allocation.proposer_reward);
        }
        self.state.rewards.treasury = self
            .state
            .rewards
            .treasury
            .saturating_add(allocation.treasury_reward);
    }

    pub fn proposer_for_next_epoch(&self, beacon: &Hash) -> Option<Address> {
        let total_work: u64 = self
            .state
            .miners
            .values()
            .map(|miner| miner.settled_tensor_work)
            .sum();
        if total_work == 0 {
            return self.fallback_proposer(beacon);
        }
        let mut draw = (hash_to_u128(beacon) % total_work as u128) as u64;
        let mut selected = None;
        for miner in self.state.miners.values() {
            if miner.settled_tensor_work == 0 {
                continue;
            }
            selected = Some(miner.address);
            if draw < miner.settled_tensor_work {
                break;
            }
            draw -= miner.settled_tensor_work;
        }
        selected
    }

    pub fn produce_block(&mut self, proposer: Address, timestamp: u64) -> TensorBlock {
        let parent_hash = self.blocks.last().map(TensorBlock::hash).unwrap_or([0; 32]);
        let job_root = job_root(&self.state.jobs);
        let receipt_root = receipt_root(&self.state.receipts);
        let attestation_root = attestation_root(&self.state.attestations);
        let state_root = self.state_root();
        let reward_root = reward_root(&self.state.rewards);
        let randomness = hash_bytes(
            b"tensor-vm-next-randomness-v1",
            &[
                &self.state.finalized_randomness,
                &parent_hash,
                &self.state.height.to_le_bytes(),
            ],
        );
        let mut block = TensorBlock {
            height: self.state.height,
            parent_hash,
            epoch: self.state.epoch,
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
        self.blocks.push(block.clone());
        self.state.height += 1;
        self.state.epoch = self.state.height / self.params.epoch_length.max(1);
        self.state.finalized_randomness = randomness;
        block
    }

    pub fn produce_block_with_rewards(
        &mut self,
        proposer: Address,
        timestamp: u64,
        fixed_block_reward: u64,
        fee_share: u64,
    ) -> TensorBlock {
        let proposer_reward = fixed_block_reward.saturating_add(fee_share);
        if proposer_reward > 0 {
            self.state.rewards.credit(proposer, proposer_reward);
        }
        self.produce_block(proposer, timestamp)
    }

    pub fn state_root(&self) -> Hash {
        let mut parts = Vec::new();
        parts.extend_from_slice(&self.state.height.to_le_bytes());
        parts.extend_from_slice(&self.state.epoch.to_le_bytes());
        parts.extend_from_slice(&self.state.finalized_randomness);
        parts.extend_from_slice(&account_root(&self.state.accounts));
        parts.extend_from_slice(&miner_root(&self.state.miners));
        parts.extend_from_slice(&validator_root(&self.state.validators));
        parts.extend_from_slice(&job_root(&self.state.jobs));
        parts.extend_from_slice(&receipt_root(&self.state.receipts));
        parts.extend_from_slice(&attestation_root(&self.state.attestations));
        parts.extend_from_slice(&block_finality_root(
            &self.state.block_votes,
            &self.state.finalized_blocks,
        ));
        parts.extend_from_slice(&hash_set_root(
            b"tensor-vm-data-unavailable-root-v1",
            &self.state.data_unavailable_receipts,
        ));
        parts.extend_from_slice(&settled_receipt_root(&self.state.settled_receipts));
        parts.extend_from_slice(&model_state_root(&self.state.model_states));
        parts.extend_from_slice(&reward_root(&self.state.rewards));
        hash_bytes(b"tensor-vm-state-root-v1", &[&parts])
    }

    fn ensure_account(&mut self, address: Address) -> &mut AccountState {
        self.state.accounts.entry(address).or_insert(AccountState {
            address,
            balance: 0,
            nonce: 0,
        })
    }

    fn fallback_proposer(&self, beacon: &Hash) -> Option<Address> {
        if self.state.validators.is_empty() {
            return self.state.miners.keys().next().copied();
        }
        let total_stake: u64 = self
            .state
            .validators
            .values()
            .map(|validator| validator.stake)
            .sum();
        let mut draw = if total_stake == 0 {
            0
        } else {
            (hash_to_u128(beacon) % total_stake as u128) as u64
        };
        for validator in self.state.validators.values() {
            if draw < validator.stake {
                return Some(validator.address);
            }
            draw -= validator.stake;
        }
        self.state.validators.keys().next().copied()
    }

    fn has_conflicting_linear_receipt(
        &self,
        receipt_id: Hash,
        receipt: &LinearTrainingStepReceipt,
    ) -> bool {
        self.state
            .receipts
            .iter()
            .any(|(other_id, other)| match other {
                ReceiptState::LinearTrainingStep(other) => {
                    *other_id != receipt_id
                        && other.model_id == receipt.model_id
                        && other.step == receipt.step
                        && other.weight_root_before == receipt.weight_root_before
                        && other.weight_root_after != receipt.weight_root_after
                        && self.has_attestation_quorum(other_id)
                }
                ReceiptState::TensorOp(_) => false,
            })
    }
}

fn reward_root(rewards: &RewardState) -> Hash {
    let mut encoded = Vec::new();
    for (address, balance) in &rewards.balances {
        encoded.extend_from_slice(address);
        encoded.extend_from_slice(&balance.to_le_bytes());
    }
    encoded.extend_from_slice(&rewards.treasury.to_le_bytes());
    hash_bytes(b"tensor-vm-reward-root-v1", &[&encoded])
}

fn reward_share(total_emission: u64, basis_points: u64) -> u64 {
    total_emission.saturating_mul(basis_points) / 10_000
}

fn receipts_agree(left: &ReceiptState, right: &ReceiptState) -> bool {
    match (left, right) {
        (ReceiptState::TensorOp(left), ReceiptState::TensorOp(right)) => {
            left.job_id == right.job_id
                && left.program_hash == right.program_hash
                && left.input_roots == right.input_roots
                && left.output_roots == right.output_roots
                && left.trace_root == right.trace_root
        }
        (ReceiptState::LinearTrainingStep(left), ReceiptState::LinearTrainingStep(right)) => {
            left.job_id == right.job_id
                && left.model_id == right.model_id
                && left.step == right.step
                && left.weight_root_before == right.weight_root_before
                && left.batch_root == right.batch_root
                && left.y_root == right.y_root
                && left.loss_commitment == right.loss_commitment
                && left.grad_w_root == right.grad_w_root
                && left.weight_root_after == right.weight_root_after
                && left.trace_root == right.trace_root
        }
        _ => false,
    }
}

fn block_finality_root(votes: &BTreeMap<Hash, Vec<BlockVote>>, finalized: &BTreeSet<Hash>) -> Hash {
    let mut encoded = Vec::new();
    for (block_hash, votes) in votes {
        encoded.extend_from_slice(block_hash);
        encoded.extend_from_slice(&(votes.len() as u64).to_le_bytes());
        for vote in votes {
            encoded.extend_from_slice(&vote.validator);
            encoded.extend_from_slice(&vote.block_hash);
            encoded.extend_from_slice(&vote.block_height.to_le_bytes());
            encoded.extend_from_slice(&vote.stake.to_le_bytes());
            encoded.extend_from_slice(&vote.signature);
        }
    }
    encoded.extend_from_slice(&(finalized.len() as u64).to_le_bytes());
    for block_hash in finalized {
        encoded.extend_from_slice(block_hash);
    }
    hash_bytes(b"tensor-vm-block-finality-root-v1", &[&encoded])
}

fn account_root(accounts: &BTreeMap<Address, AccountState>) -> Hash {
    let mut encoded = Vec::new();
    for (address, account) in accounts {
        encoded.extend_from_slice(address);
        encoded.extend_from_slice(&account.balance.to_le_bytes());
        encoded.extend_from_slice(&account.nonce.to_le_bytes());
    }
    hash_bytes(b"tensor-vm-account-root-v1", &[&encoded])
}

fn miner_root(miners: &BTreeMap<Address, MinerState>) -> Hash {
    let mut encoded = Vec::new();
    for (address, miner) in miners {
        encoded.extend_from_slice(address);
        encoded.extend_from_slice(&miner.address);
        encoded.extend_from_slice(&miner.operator_id);
        encoded.extend_from_slice(&miner.stake.to_le_bytes());
        encoded.extend_from_slice(&miner.reputation.to_le_bytes());
        encoded.extend_from_slice(&miner.settled_tensor_work.to_le_bytes());
        encoded.extend_from_slice(&miner.pending_tensor_work.to_le_bytes());
        encoded.push(miner.hardware_class.tag());
        encoded.extend_from_slice(&miner.gpu_utilization_bps.to_le_bytes());
    }
    hash_bytes(b"tensor-vm-miner-root-v1", &[&encoded])
}

fn validator_root(validators: &BTreeMap<Address, ValidatorState>) -> Hash {
    let mut encoded = Vec::new();
    for (address, validator) in validators {
        encoded.extend_from_slice(address);
        encoded.extend_from_slice(&validator.address);
        encoded.extend_from_slice(&validator.stake.to_le_bytes());
        encoded.extend_from_slice(&validator.reputation.to_le_bytes());
        encoded.extend_from_slice(&validator.valid_attestations.to_le_bytes());
        encoded.extend_from_slice(&validator.missed_assignments.to_le_bytes());
    }
    hash_bytes(b"tensor-vm-validator-root-v1", &[&encoded])
}

fn job_root(jobs: &BTreeMap<Hash, JobState>) -> Hash {
    let mut encoded = Vec::new();
    for (job_id, job) in jobs {
        encoded.extend_from_slice(job_id);
        match job {
            JobState::TensorOp(job) => {
                encoded.push(1);
                encoded.extend_from_slice(&job.job_id);
                encoded.extend_from_slice(&job.epoch.to_le_bytes());
                encode_usize(&mut encoded, job.m);
                encode_usize(&mut encoded, job.k);
                encode_usize(&mut encoded, job.n);
                encoded.push(dtype_code(job.dtype));
                encoded.extend_from_slice(&job.modulus.unwrap_or_default().to_le_bytes());
                encoded.extend_from_slice(&job.seed_a);
                encoded.extend_from_slice(&job.seed_b);
                encoded.extend_from_slice(&job.deadline_block.to_le_bytes());
                encoded.extend_from_slice(&job.reward_weight.to_le_bytes());
            }
            JobState::LinearTrainingStep(job) => {
                encoded.push(2);
                encoded.extend_from_slice(&job.job_id);
                encoded.extend_from_slice(&job.model_id);
                encoded.extend_from_slice(&job.step.to_le_bytes());
                encoded.extend_from_slice(&job.batch_seed);
                encoded.extend_from_slice(&job.weight_root_before);
                encode_usizes(&mut encoded, &job.input_shape);
                encode_usizes(&mut encoded, &job.weight_shape);
                encode_usizes(&mut encoded, &job.target_shape);
                encoded.extend_from_slice(&job.lr.to_le_bytes());
                encoded.push(dtype_code(job.dtype));
                encoded.extend_from_slice(&job.deadline_block.to_le_bytes());
                encoded.extend_from_slice(&job.reward_weight.to_le_bytes());
            }
        }
    }
    hash_bytes(b"tensor-vm-job-root-v1", &[&encoded])
}

fn receipt_root(receipts: &BTreeMap<Hash, ReceiptState>) -> Hash {
    let mut encoded = Vec::new();
    for (receipt_id, receipt) in receipts {
        encoded.extend_from_slice(receipt_id);
        match receipt {
            ReceiptState::TensorOp(receipt) => {
                encoded.push(1);
                encoded.extend_from_slice(&receipt.receipt_id);
                encoded.extend_from_slice(&receipt.job_id);
                encoded.extend_from_slice(&receipt.miner);
                encoded.extend_from_slice(&receipt.program_hash);
                encode_hashes(&mut encoded, &receipt.input_roots);
                encode_hashes(&mut encoded, &receipt.output_roots);
                encoded.extend_from_slice(&receipt.trace_root);
                encoded.extend_from_slice(&receipt.tensor_work_units.to_le_bytes());
                encoded.extend_from_slice(&receipt.execution_time_ms.to_le_bytes());
                encoded.extend_from_slice(&receipt.submitted_at_block.to_le_bytes());
                encoded.extend_from_slice(&receipt.signature);
            }
            ReceiptState::LinearTrainingStep(receipt) => {
                encoded.push(2);
                encoded.extend_from_slice(&receipt.receipt_id);
                encoded.extend_from_slice(&receipt.job_id);
                encoded.extend_from_slice(&receipt.miner);
                encoded.extend_from_slice(&receipt.model_id);
                encoded.extend_from_slice(&receipt.step.to_le_bytes());
                encoded.extend_from_slice(&receipt.weight_root_before);
                encoded.extend_from_slice(&receipt.batch_root);
                encoded.extend_from_slice(&receipt.y_root);
                encoded.extend_from_slice(&receipt.loss_commitment);
                encoded.extend_from_slice(&receipt.grad_w_root);
                encoded.extend_from_slice(&receipt.weight_root_after);
                encoded.extend_from_slice(&receipt.trace_root);
                encoded.extend_from_slice(&receipt.tensor_work_units.to_le_bytes());
                encoded.extend_from_slice(&receipt.execution_time_ms.to_le_bytes());
                encoded.extend_from_slice(&receipt.submitted_at_block.to_le_bytes());
                encoded.extend_from_slice(&receipt.signature);
            }
        }
    }
    hash_bytes(b"tensor-vm-receipt-root-v1", &[&encoded])
}

fn attestation_root(attestations: &BTreeMap<Hash, Vec<ValidatorAttestation>>) -> Hash {
    let mut encoded = Vec::new();
    for (receipt_id, attestations) in attestations {
        encoded.extend_from_slice(receipt_id);
        encoded.extend_from_slice(&(attestations.len() as u64).to_le_bytes());
        for attestation in attestations {
            encoded.extend_from_slice(&attestation.validator);
            encoded.extend_from_slice(&attestation.receipt_id);
            encoded.extend_from_slice(&attestation.job_id);
            encoded.push(primitive_type_code(attestation.primitive_type));
            encoded.push(verification_result_code(attestation.result));
            encoded.push(attestation.data_availability_passed as u8);
            encoded.extend_from_slice(&attestation.checks_root);
            encoded.extend_from_slice(&attestation.stake.to_le_bytes());
            encoded.extend_from_slice(&attestation.signature);
        }
    }
    hash_bytes(b"tensor-vm-attestation-root-v1", &[&encoded])
}

fn settled_receipt_root(receipts: &BTreeSet<Hash>) -> Hash {
    hash_set_root(b"tensor-vm-settled-receipt-root-v1", receipts)
}

fn hash_set_root(domain: &[u8], items: &BTreeSet<Hash>) -> Hash {
    let mut encoded = Vec::new();
    for item in items {
        encoded.extend_from_slice(item);
    }
    hash_bytes(domain, &[&encoded])
}

fn model_state_root(models: &BTreeMap<Hash, ModelState>) -> Hash {
    let mut encoded = Vec::new();
    for (model_id, model) in models {
        encoded.extend_from_slice(model_id);
        encoded.extend_from_slice(&model.model_id);
        encoded.extend_from_slice(&model.architecture_hash);
        encoded.extend_from_slice(&model.weight_root);
        match model.optimizer_state_root {
            Some(root) => {
                encoded.push(1);
                encoded.extend_from_slice(&root);
            }
            None => encoded.push(0),
        }
        encoded.extend_from_slice(&model.step.to_le_bytes());
        encoded.extend_from_slice(&model.config_hash);
    }
    hash_bytes(b"tensor-vm-model-state-root-v1", &[&encoded])
}

fn encode_hashes(out: &mut Vec<u8>, hashes: &[Hash]) {
    out.extend_from_slice(&(hashes.len() as u64).to_le_bytes());
    for hash in hashes {
        out.extend_from_slice(hash);
    }
}

fn encode_usizes(out: &mut Vec<u8>, values: &[usize]) {
    out.extend_from_slice(&(values.len() as u64).to_le_bytes());
    for value in values {
        encode_usize(out, *value);
    }
}

fn encode_usize(out: &mut Vec<u8>, value: usize) {
    out.extend_from_slice(&(value as u64).to_le_bytes());
}

fn dtype_code(dtype: crate::tensor::DType) -> u8 {
    dtype.tag()
}

fn primitive_type_code(primitive_type: PrimitiveType) -> u8 {
    match primitive_type {
        PrimitiveType::TensorOp => 1,
        PrimitiveType::LinearTrainingStep => 2,
    }
}

fn verification_result_code(result: VerificationResult) -> u8 {
    match result {
        VerificationResult::Valid => 1,
        VerificationResult::Invalid => 2,
        VerificationResult::Unavailable => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::{
        LinearTrainingStepJob, LinearTrainingStepReceipt, LinearTrainingStepSpec, MatmulJob,
        TensorOpReceipt,
    };
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes};
    use crate::verify::{
        AttestationStatement, FreivaldsParams, ValidatorAttestation, VerificationResult,
        verify_tensor_op,
    };

    #[test]
    fn chain_settles_valid_tensorwork_and_rewards_participants() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let params = ChainParams {
            agreement_quorum: 1,
            ..ChainParams::default()
        };
        let mut chain = LocalChain::with_params(params, beacon);
        let miner = address(b"miner");
        chain.register_miner(miner, 100).unwrap();
        let validators: Vec<_> = (0..5)
            .map(|i| address(format!("validator-{i}").as_bytes()))
            .collect();
        for validator in &validators {
            chain.register_validator(*validator, 10_000).unwrap();
        }

        let job = MatmulJob::synthetic(0, 0, 8, 8, 8, &beacon, 10);
        let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        let report = verify_tensor_op(
            &job,
            &receipt,
            &a,
            &b,
            &c,
            &hash_bytes(b"test", &[b"validation"]),
            &chain.params.freivalds,
        )
        .unwrap();
        chain.submit_job(JobState::TensorOp(job.clone()));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
        for validator in &validators {
            chain
                .submit_attestation(ValidatorAttestation::new(
                    *validator,
                    10_000,
                    AttestationStatement {
                        receipt_id: receipt.receipt_id,
                        job_id: receipt.job_id,
                        primitive_type: PrimitiveType::TensorOp,
                        result: report.result,
                        checks_root: report.checks_root,
                        data_availability_passed: report.data_availability_passed,
                    },
                ))
                .unwrap();
        }

        assert!(chain.has_attestation_quorum(&receipt.receipt_id));
        chain.settle_epoch(1_000, 500);
        assert_eq!(
            chain.state.miners.get(&miner).unwrap().settled_tensor_work,
            receipt.tensor_work_units
        );
        assert_eq!(chain.state.rewards.balance(&miner), 1_000);
        let validator_reward = chain.state.rewards.balance(&validators[0]);
        assert!(validator_reward > 0);
        chain.settle_epoch(1_000, 500);
        assert_eq!(chain.state.rewards.balance(&miner), 1_000);
        assert_eq!(
            chain.state.rewards.balance(&validators[0]),
            validator_reward
        );
    }

    #[test]
    fn chain_tracks_accounts_jobs_and_transfers() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let alice = address(b"alice");
        let bob = address(b"bob");
        chain.credit_account(alice, 1_000);
        chain.transfer(alice, bob, 250).unwrap();
        assert_eq!(chain.state.accounts.get(&alice).unwrap().balance, 750);
        assert_eq!(chain.state.accounts.get(&alice).unwrap().nonce, 1);
        assert_eq!(chain.state.accounts.get(&bob).unwrap().balance, 250);

        let job = MatmulJob::synthetic(0, 3, 2, 2, 2, &beacon, 10);
        chain.submit_job(JobState::TensorOp(job.clone()));
        assert_eq!(chain.job(&job.job_id).unwrap().deadline_block(), 10);
    }

    #[test]
    fn chain_params_define_tensor_retention_deadline() {
        let params = ChainParams {
            epoch_length: 50,
            reward_settlement_delay_epochs: 2,
            challenge_window_epochs: 3,
            ..ChainParams::default()
        };
        assert_eq!(params.tensor_retention_window_blocks(), 250);
        assert_eq!(params.tensor_retention_deadline(10), 260);
    }

    #[test]
    fn reward_allocation_matches_mvp_split_and_credits_proposer_and_treasury() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let proposer = address(b"reward-proposer");
        chain
            .register_miner(proposer, chain.params.miner_min_stake)
            .unwrap();

        let allocation = chain.params.reward_allocation(10_000);
        assert_eq!(
            allocation,
            RewardAllocation {
                miner_reward_pool: 7_000,
                validator_reward_pool: 2_000,
                proposer_reward: 500,
                treasury_reward: 500,
            }
        );

        let block = chain.produce_block_with_rewards(proposer, 1_000, 400, 100);
        assert_eq!(chain.state.rewards.balance(&proposer), 500);
        assert_eq!(block.reward_root, reward_root(&chain.state.rewards));

        chain.settle_epoch_rewards(allocation, proposer);
        assert_eq!(chain.state.rewards.balance(&proposer), 1_000);
        assert_eq!(chain.state.rewards.treasury, 500);
    }

    #[test]
    fn validation_seed_is_bound_to_finalized_randomness_and_receipt() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let chain = LocalChain::new(beacon);
        let receipt_a = hash_bytes(b"test", &[b"receipt-a"]);
        let receipt_b = hash_bytes(b"test", &[b"receipt-b"]);
        assert_ne!(
            chain.validation_seed(&receipt_a),
            chain.validation_seed(&receipt_b)
        );

        let other_chain = LocalChain::new(hash_bytes(b"test", &[b"other-beacon"]));
        assert_ne!(
            chain.validation_seed(&receipt_a),
            other_chain.validation_seed(&receipt_a)
        );
    }

    #[test]
    fn chain_applies_register_transfer_and_claim_reward_transactions() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let miner = address(b"miner-tx");
        let validator = address(b"validator-tx");
        let receiver = address(b"receiver");
        chain
            .apply_transaction(None, Transaction::RegisterMiner(miner))
            .unwrap();
        chain
            .apply_transaction(None, Transaction::RegisterValidator(validator))
            .unwrap();
        assert!(chain.state.miners.contains_key(&miner));
        assert!(chain.state.validators.contains_key(&validator));

        chain.credit_account(miner, 500);
        chain
            .apply_transaction(
                Some(miner),
                Transaction::Transfer {
                    to: receiver,
                    amount: 125,
                },
            )
            .unwrap();
        assert_eq!(chain.state.accounts.get(&receiver).unwrap().balance, 125);

        chain.state.rewards.credit(miner, 42);
        chain
            .apply_transaction(None, Transaction::ClaimReward(miner))
            .unwrap();
        assert_eq!(chain.state.rewards.balance(&miner), 0);
        assert_eq!(chain.state.accounts.get(&miner).unwrap().balance, 417);
        assert_eq!(
            chain.apply_transaction(
                None,
                Transaction::SubmitTensorOpReceipt(hash_bytes(
                    b"test",
                    &[b"queued-tensor-receipt"]
                ))
            ),
            Ok(())
        );
        assert_eq!(
            chain.apply_transaction(
                None,
                Transaction::SubmitLinearTrainingStepReceipt(hash_bytes(
                    b"test",
                    &[b"queued-linear-receipt"]
                ))
            ),
            Ok(())
        );
        assert_eq!(
            chain.apply_transaction(
                None,
                Transaction::SubmitAttestation(hash_bytes(b"test", &[b"queued-attestation"]))
            ),
            Ok(())
        );
    }

    #[test]
    fn miner_root_commits_to_operator_identity() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let miner = address(b"operator-root-miner");
        chain
            .register_miner_with_operator(
                miner,
                chain.params.miner_min_stake,
                address(b"operator-root-a"),
            )
            .unwrap();

        let original_root = miner_root(&chain.state.miners);
        let mut changed_miners = chain.state.miners.clone();
        changed_miners.get_mut(&miner).unwrap().operator_id = address(b"operator-root-b");

        assert_ne!(original_root, miner_root(&changed_miners));
    }

    #[test]
    fn chain_rejects_boundary_registration_receipt_vote_and_challenge_errors() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let miner = address(b"boundary-miner");
        let validator = address(b"boundary-validator");
        let receiver = address(b"boundary-receiver");

        assert_eq!(chain.proposer_for_next_epoch(&beacon), None);
        assert_eq!(
            chain.register_miner(miner, chain.params.miner_min_stake - 1),
            Err(TvmError::InsufficientStake)
        );
        assert_eq!(
            chain.register_miner_with_profile(
                miner,
                chain.params.miner_min_stake,
                HardwareClass::ConsumerGpu,
                10_001,
            ),
            Err(TvmError::InvalidReceipt("gpu utilization exceeds 100%"))
        );
        assert_eq!(
            chain.register_miner_with_profile(
                miner,
                chain.params.miner_min_stake,
                HardwareClass::Other,
                1,
            ),
            Err(TvmError::InvalidReceipt(
                "non-gpu miner cannot report gpu utilization"
            ))
        );
        chain
            .register_miner_with_profile(
                miner,
                chain.params.miner_min_stake,
                HardwareClass::DatacenterGpu,
                9_000,
            )
            .unwrap();
        let registered_miner = chain.state.miners.get(&miner).unwrap();
        assert_eq!(registered_miner.operator_id, miner);
        assert_eq!(
            registered_miner.hardware_class,
            HardwareClass::DatacenterGpu
        );
        let explicit_operator = address(b"boundary-operator");
        let explicit_miner = address(b"boundary-explicit-miner");
        chain
            .register_miner_with_operator(
                explicit_miner,
                chain.params.miner_min_stake,
                explicit_operator,
            )
            .unwrap();
        assert_eq!(
            chain.state.miners.get(&explicit_miner).unwrap().operator_id,
            explicit_operator
        );
        assert_ne!(miner_root(&chain.state.miners), [0; 32]);
        assert_eq!(
            [HardwareClass::Cpu.tag(), HardwareClass::ConsumerGpu.tag()],
            [1, 2]
        );
        assert_eq!(HardwareClass::Other.tag(), 4);
        assert!(HardwareClass::DatacenterGpu.is_gpu());

        assert_eq!(
            chain.register_validator(validator, chain.params.validator_min_stake - 1),
            Err(TvmError::InsufficientStake)
        );
        chain
            .register_validator(validator, chain.params.validator_min_stake)
            .unwrap();

        assert_eq!(
            chain.transfer(miner, receiver, 1),
            Err(TvmError::InvalidReceipt("insufficient account balance"))
        );
        assert_eq!(
            chain.apply_transaction(
                None,
                Transaction::Transfer {
                    to: receiver,
                    amount: 1,
                },
            ),
            Err(TvmError::InvalidReceipt("missing sender"))
        );
        assert_eq!(
            chain.apply_transaction(None, Transaction::ClaimReward(miner)),
            Err(TvmError::InvalidReceipt("no reward to claim"))
        );

        let job = MatmulJob::synthetic(0, 77, 2, 2, 2, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        let mut unknown_miner_receipt = receipt.clone();
        unknown_miner_receipt.miner = address(b"missing-miner");
        assert_eq!(
            chain.submit_tensor_op_receipt(unknown_miner_receipt),
            Err(TvmError::UnknownMiner)
        );
        assert_eq!(
            chain.submit_tensor_op_receipt(receipt.clone()),
            Err(TvmError::InvalidReceipt("unknown job"))
        );

        let weights = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let linear_job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"boundary-model"]),
            step: 0,
            batch_seed: hash_bytes(b"test", &[b"boundary-batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![2, 2],
            weight_shape: vec![2, 2],
            target_shape: vec![2, 2],
            lr: 1,
            deadline_block: 10,
        });
        let (linear_receipt, _output) =
            LinearTrainingStepReceipt::from_job(&linear_job, miner, &weights, 1, 5).unwrap();
        assert_eq!(
            chain.submit_linear_receipt(linear_receipt.clone()),
            Err(TvmError::InvalidReceipt("unknown job"))
        );
        let mut unknown_linear_miner = linear_receipt.clone();
        unknown_linear_miner.miner = address(b"missing-linear-miner");
        assert_eq!(
            chain.submit_linear_receipt(unknown_linear_miner),
            Err(TvmError::UnknownMiner)
        );
        chain.submit_job(JobState::LinearTrainingStep(linear_job.clone()));
        assert_eq!(chain.job(&linear_job.job_id).unwrap().deadline_block(), 10);
        chain.submit_linear_receipt(linear_receipt.clone()).unwrap();
        assert!(!receipts_agree(
            &ReceiptState::TensorOp(receipt.clone()),
            &ReceiptState::LinearTrainingStep(linear_receipt.clone())
        ));
        assert_eq!(
            chain
                .state
                .receipts
                .get(&linear_receipt.receipt_id)
                .unwrap()
                .receipt_id(),
            linear_receipt.receipt_id
        );
        assert_eq!(
            chain.submit_linear_receipt(linear_receipt.clone()),
            Err(TvmError::InvalidReceipt("duplicate receipt"))
        );

        chain.submit_job(JobState::TensorOp(job.clone()));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
        let statement = AttestationStatement {
            receipt_id: receipt.receipt_id,
            job_id: receipt.job_id,
            primitive_type: PrimitiveType::TensorOp,
            result: VerificationResult::Valid,
            checks_root: hash_bytes(b"test", &[b"checks"]),
            data_availability_passed: true,
        };
        assert_eq!(
            chain.submit_attestation(ValidatorAttestation::new(
                address(b"unknown-validator"),
                chain.params.validator_min_stake,
                statement.clone(),
            )),
            Err(TvmError::UnknownValidator)
        );
        let mut bad_signature =
            ValidatorAttestation::new(validator, chain.params.validator_min_stake, statement);
        bad_signature.signature = [9; 32];
        assert_eq!(
            chain.submit_attestation(bad_signature),
            Err(TvmError::InvalidReceipt("bad attestation signature"))
        );
        assert_eq!(
            chain.submit_attestation(ValidatorAttestation::new(
                validator,
                chain.params.validator_min_stake,
                AttestationStatement {
                    receipt_id: hash_bytes(b"test", &[b"unknown-receipt"]),
                    job_id: receipt.job_id,
                    primitive_type: PrimitiveType::TensorOp,
                    result: VerificationResult::Valid,
                    checks_root: hash_bytes(b"test", &[b"checks"]),
                    data_availability_passed: true,
                },
            )),
            Err(TvmError::UnknownReceipt)
        );

        let block = chain.produce_block(miner, 1_000);
        assert_eq!(
            chain.submit_block_vote(BlockVote::new(
                address(b"unknown-vote-validator"),
                1,
                &block
            )),
            Err(TvmError::UnknownValidator)
        );
        let mut bad_vote = BlockVote::new(validator, chain.params.validator_min_stake, &block);
        bad_vote.signature = [7; 32];
        assert_eq!(
            chain.submit_block_vote(bad_vote),
            Err(TvmError::InvalidReceipt("bad block vote signature"))
        );
        let mut orphan = block.clone();
        orphan.height = 999;
        assert_eq!(
            chain.submit_block_vote(BlockVote::new(
                validator,
                chain.params.validator_min_stake,
                &orphan,
            )),
            Err(TvmError::InvalidReceipt("unknown block"))
        );

        let model = hash_bytes(b"test", &[b"missing-model"]);
        assert_eq!(
            chain.apply_model_transition(&model, 0, &weights.commitment_root(), [1; 32]),
            Err(TvmError::InvalidReceipt("unknown model"))
        );
        assert_eq!(
            chain.apply_challenge_outcome(ChallengeOutcome::Rejected {
                reason: "honest".to_owned(),
            }),
            Ok(())
        );
        assert_eq!(
            chain.apply_challenge_outcome(ChallengeOutcome::ProvenInvalid {
                dishonest_party: address(b"unknown-dishonest-party"),
                slash_amount: 1,
                reason: "invalid".to_owned(),
            }),
            Err(TvmError::InvalidReceipt("unknown dishonest party"))
        );
        assert_eq!(
            chain.apply_challenge_outcome(ChallengeOutcome::ProvenInvalid {
                dishonest_party: validator,
                slash_amount: 100,
                reason: "bad attestation".to_owned(),
            }),
            Ok(())
        );
        assert_eq!(
            chain.state.validators.get(&validator).unwrap().stake,
            chain.params.validator_min_stake - 100
        );
    }

    #[test]
    fn invalid_attestations_do_not_create_quorum() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let miner = address(b"miner");
        chain.register_miner(miner, 100).unwrap();
        let validator = address(b"validator");
        chain.register_validator(validator, 10_000).unwrap();
        let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        chain.submit_job(JobState::TensorOp(job.clone()));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
        chain
            .submit_attestation(ValidatorAttestation::new(
                validator,
                10_000,
                AttestationStatement {
                    receipt_id: receipt.receipt_id,
                    job_id: receipt.job_id,
                    primitive_type: PrimitiveType::TensorOp,
                    result: VerificationResult::Invalid,
                    checks_root: hash_bytes(b"test", &[b"checks"]),
                    data_availability_passed: true,
                },
            ))
            .unwrap();
        assert!(!chain.has_attestation_quorum(&receipt.receipt_id));
        assert_ne!(attestation_root(&chain.state.attestations), [0; 32]);
        chain.settle_epoch(1_000, 500);
        assert_eq!(chain.state.rewards.balance(&miner), 0);
    }

    #[test]
    fn quorum_and_agreement_helpers_reject_unknown_receipts() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let validator = address(b"orphan-validator");
        chain.register_validator(validator, 10_000).unwrap();
        let receipt_id = hash_bytes(b"test", &[b"orphan-receipt"]);
        chain.state.attestations.insert(
            receipt_id,
            vec![ValidatorAttestation::new(
                validator,
                10_000,
                AttestationStatement {
                    receipt_id,
                    job_id: hash_bytes(b"test", &[b"orphan-job"]),
                    primitive_type: PrimitiveType::TensorOp,
                    result: VerificationResult::Valid,
                    checks_root: hash_bytes(b"test", &[b"orphan-checks"]),
                    data_availability_passed: true,
                },
            )],
        );

        assert!(!chain.has_attestation_quorum(&receipt_id));
        assert_eq!(chain.redundant_agreement_count(&receipt_id), 0);
        assert!(!chain.has_redundant_agreement(&receipt_id));
    }

    #[test]
    fn unavailable_data_attestation_penalizes_receipt_miner_once() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let miner = address(b"unavailable-miner");
        chain.register_miner(miner, 100).unwrap();
        let validators: Vec<_> = (0..2)
            .map(|i| address(format!("unavailable-validator-{i}").as_bytes()))
            .collect();
        for validator in &validators {
            chain.register_validator(*validator, 10_000).unwrap();
        }
        let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        chain.submit_job(JobState::TensorOp(job));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();

        for validator in &validators {
            chain
                .submit_attestation(ValidatorAttestation::new(
                    *validator,
                    10_000,
                    AttestationStatement {
                        receipt_id: receipt.receipt_id,
                        job_id: receipt.job_id,
                        primitive_type: PrimitiveType::TensorOp,
                        result: VerificationResult::Unavailable,
                        checks_root: hash_bytes(b"test", &[b"unavailable"]),
                        data_availability_passed: false,
                    },
                ))
                .unwrap();
        }

        assert_eq!(
            chain.state.miners.get(&miner).unwrap().reputation,
            -1,
            "availability penalty is per receipt, not per validator"
        );
        assert!(
            chain
                .state
                .data_unavailable_receipts
                .contains(&receipt.receipt_id)
        );
        assert_ne!(attestation_root(&chain.state.attestations), [0; 32]);
        assert!(!chain.has_attestation_quorum(&receipt.receipt_id));
        chain.settle_epoch(1_000, 500);
        assert_eq!(chain.state.rewards.balance(&miner), 0);
    }

    #[test]
    fn mismatched_attestation_metadata_penalizes_validator_and_is_rejected() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let miner = address(b"mismatch-miner");
        let validator = address(b"mismatch-validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();
        let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        chain.submit_job(JobState::TensorOp(job));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();

        let bad_attestation = ValidatorAttestation::new(
            validator,
            10_000,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: hash_bytes(b"test", &[b"wrong-job"]),
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"checks"]),
                data_availability_passed: true,
            },
        );

        assert_eq!(
            chain.submit_attestation(bad_attestation),
            Err(TvmError::InvalidReceipt("attestation receipt mismatch"))
        );
        assert_eq!(
            chain.state.validators.get(&validator).unwrap().reputation,
            -1
        );
        assert!(!chain.state.attestations.contains_key(&receipt.receipt_id));
    }

    #[test]
    fn redundant_agreement_quorum_is_required_before_settlement() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let params = ChainParams {
            agreement_quorum: 3,
            freivalds: FreivaldsParams {
                minimum_validators: 1,
                validators_per_job: 1,
                minimum_stake_numerator: 1,
                minimum_stake_denominator: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = LocalChain::with_params(params, beacon);
        let miners: Vec<_> = (0..3)
            .map(|i| address(format!("agreement-miner-{i}").as_bytes()))
            .collect();
        for miner in &miners {
            chain.register_miner(*miner, 100).unwrap();
        }
        let validator = address(b"agreement-validator");
        chain.register_validator(validator, 10_000).unwrap();

        let job = MatmulJob::synthetic(0, 9, 4, 4, 4, &beacon, 10);
        chain.submit_job(JobState::TensorOp(job.clone()));
        let receipts: Vec<_> = miners
            .iter()
            .map(|miner| TensorOpReceipt::from_job(&job, *miner, 1, 5).unwrap().0)
            .collect();
        for receipt in receipts.iter().take(2) {
            chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
            chain
                .submit_attestation(ValidatorAttestation::new(
                    validator,
                    10_000,
                    AttestationStatement {
                        receipt_id: receipt.receipt_id,
                        job_id: receipt.job_id,
                        primitive_type: PrimitiveType::TensorOp,
                        result: VerificationResult::Valid,
                        checks_root: hash_bytes(b"test", &[&receipt.receipt_id]),
                        data_availability_passed: true,
                    },
                ))
                .unwrap();
        }

        assert_eq!(chain.redundant_agreement_count(&receipts[0].receipt_id), 2);
        assert!(!chain.has_redundant_agreement(&receipts[0].receipt_id));
        chain.settle_epoch(1_000, 500);
        assert!(chain.state.settled_receipts.is_empty());

        let receipt = &receipts[2];
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
        chain
            .submit_attestation(ValidatorAttestation::new(
                validator,
                10_000,
                AttestationStatement {
                    receipt_id: receipt.receipt_id,
                    job_id: receipt.job_id,
                    primitive_type: PrimitiveType::TensorOp,
                    result: VerificationResult::Valid,
                    checks_root: hash_bytes(b"test", &[&receipt.receipt_id]),
                    data_availability_passed: true,
                },
            ))
            .unwrap();

        assert_eq!(chain.redundant_agreement_count(&receipts[0].receipt_id), 3);
        assert!(chain.has_redundant_agreement(&receipts[0].receipt_id));
        chain.settle_epoch(1_000, 500);
        assert_eq!(chain.state.settled_receipts.len(), 3);
    }

    #[test]
    fn duplicate_receipts_and_validator_attestations_are_rejected() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let miner = address(b"miner");
        let validator = address(b"validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();

        assert_eq!(
            chain.register_miner(miner, 100),
            Err(TvmError::InvalidReceipt("miner already registered"))
        );
        assert_eq!(
            chain.register_validator(validator, 10_000),
            Err(TvmError::InvalidReceipt("validator already registered"))
        );

        let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
        let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        let report = verify_tensor_op(
            &job,
            &receipt,
            &a,
            &b,
            &c,
            &hash_bytes(b"test", &[b"validation"]),
            &chain.params.freivalds,
        )
        .unwrap();
        chain.submit_job(JobState::TensorOp(job));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
        assert_eq!(
            chain.submit_tensor_op_receipt(receipt.clone()),
            Err(TvmError::InvalidReceipt("duplicate receipt"))
        );

        let attestation = ValidatorAttestation::new(
            validator,
            10_000,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: report.result,
                checks_root: report.checks_root,
                data_availability_passed: report.data_availability_passed,
            },
        );
        chain.submit_attestation(attestation.clone()).unwrap();
        assert_eq!(
            chain.submit_attestation(attestation),
            Err(TvmError::InvalidReceipt("duplicate validator attestation"))
        );
        assert_eq!(
            chain
                .state
                .attestations
                .get(&receipt.receipt_id)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn forged_attestation_stake_is_rejected() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let miner = address(b"miner");
        let validator = address(b"validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();
        let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        chain.submit_job(JobState::TensorOp(job.clone()));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();

        let result = chain.submit_attestation(ValidatorAttestation::new(
            validator,
            1_000_000,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"checks"]),
                data_availability_passed: true,
            },
        ));

        assert!(matches!(
            result,
            Err(TvmError::InvalidReceipt("attestation stake mismatch"))
        ));
    }

    #[test]
    fn proposer_selection_uses_fallback_until_work_settles() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let validator = address(b"validator");
        chain.register_validator(validator, 10_000).unwrap();
        assert_eq!(chain.proposer_for_next_epoch(&beacon), Some(validator));
    }

    #[test]
    fn fallback_proposer_handles_zero_stake_validator_records() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let validator = address(b"zero-stake-validator");
        chain.register_validator(validator, 10_000).unwrap();
        chain.state.validators.get_mut(&validator).unwrap().stake = 0;

        assert_eq!(chain.proposer_for_next_epoch(&beacon), Some(validator));
    }

    #[test]
    fn proposer_selection_ignores_pending_tensorwork() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let settled = address(b"settled-miner");
        let pending = address(b"pending-miner");
        chain.register_miner(settled, 100).unwrap();
        chain.register_miner(pending, 100).unwrap();
        chain
            .state
            .miners
            .get_mut(&settled)
            .unwrap()
            .settled_tensor_work = 1;
        chain
            .state
            .miners
            .get_mut(&pending)
            .unwrap()
            .pending_tensor_work = 1_000_000;

        assert_eq!(chain.proposer_for_next_epoch(&beacon), Some(settled));
    }

    #[test]
    fn blocks_advance_height_and_commit_state() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let proposer = address(b"proposer");
        chain.register_miner(proposer, 100).unwrap();
        let block = chain.produce_block(proposer, 1_000);
        assert_eq!(block.height, 0);
        assert_eq!(chain.state.height, 1);
        assert_eq!(chain.blocks.len(), 1);
    }

    #[test]
    fn block_finality_requires_two_thirds_validator_stake() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let proposer = address(b"proposer");
        chain.register_miner(proposer, 100).unwrap();
        let validators: Vec<_> = (0..3)
            .map(|i| address(format!("finality-validator-{i}").as_bytes()))
            .collect();
        for validator in &validators {
            chain.register_validator(*validator, 10_000).unwrap();
        }
        let block = chain.produce_block(proposer, 1_000);
        let block_hash = block.hash();

        assert!(!chain.has_block_finality(&block_hash));
        chain
            .submit_block_vote(BlockVote::new(validators[0], 10_000, &block))
            .unwrap();
        assert!(!chain.has_block_finality(&block_hash));
        chain
            .submit_block_vote(BlockVote::new(validators[1], 10_000, &block))
            .unwrap();

        assert!(chain.has_block_finality(&block_hash));
        assert!(chain.is_block_finalized(&block_hash));
        assert_eq!(
            chain.submit_block_vote(BlockVote::new(validators[1], 10_000, &block)),
            Err(TvmError::InvalidReceipt("duplicate block vote"))
        );
        assert_eq!(
            chain.submit_block_vote(BlockVote::new(validators[2], 1, &block)),
            Err(TvmError::InvalidReceipt("block vote stake mismatch"))
        );
    }

    #[test]
    fn block_finality_ignores_invalid_direct_vote_records() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        assert!(!LocalChain::new(beacon).has_block_finality(&hash_bytes(b"test", &[b"no-stake"])));

        let mut chain = LocalChain::new(beacon);
        let proposer = address(b"finality-proposer");
        chain.register_miner(proposer, 100).unwrap();
        let validators: Vec<_> = (0..3)
            .map(|i| address(format!("invalid-finality-validator-{i}").as_bytes()))
            .collect();
        for validator in &validators {
            chain.register_validator(*validator, 10_000).unwrap();
        }
        let block = chain.produce_block(proposer, 1_000);
        let block_hash = block.hash();

        let unknown = BlockVote::new(address(b"unknown-direct-validator"), 10_000, &block);
        let wrong_stake = BlockVote::new(validators[0], 1, &block);
        let valid = BlockVote::new(validators[0], 10_000, &block);
        let duplicate = BlockVote::new(validators[0], 10_000, &block);
        let mut bad_signature = BlockVote::new(validators[1], 10_000, &block);
        bad_signature.signature = [9; 32];
        chain.state.block_votes.insert(
            block_hash,
            vec![unknown, wrong_stake, valid, duplicate, bad_signature],
        );

        assert!(!chain.has_block_finality(&block_hash));
        assert!(!chain.is_block_finalized(&block_hash));
    }

    #[test]
    fn block_roots_commit_to_jobs_receipts_attestations_and_state_values() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let miner = address(b"root-miner");
        let validator = address(b"root-validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();

        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        let report = verify_tensor_op(
            &job,
            &receipt,
            &a,
            &b,
            &c,
            &hash_bytes(b"test", &[b"validation"]),
            &chain.params.freivalds,
        )
        .unwrap();
        chain.submit_job(JobState::TensorOp(job.clone()));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
        chain
            .submit_attestation(ValidatorAttestation::new(
                validator,
                10_000,
                AttestationStatement {
                    receipt_id: receipt.receipt_id,
                    job_id: receipt.job_id,
                    primitive_type: PrimitiveType::TensorOp,
                    result: report.result,
                    checks_root: report.checks_root,
                    data_availability_passed: report.data_availability_passed,
                },
            ))
            .unwrap();

        let expected_job_root = job_root(&chain.state.jobs);
        let expected_receipt_root = receipt_root(&chain.state.receipts);
        let expected_attestation_root = attestation_root(&chain.state.attestations);
        let expected_state_root = chain.state_root();
        let block = chain.produce_block(miner, 1_000);
        assert_eq!(block.job_root, expected_job_root);
        assert_eq!(block.receipt_root, expected_receipt_root);
        assert_eq!(block.attestation_root, expected_attestation_root);
        assert_eq!(block.state_root, expected_state_root);

        let mut altered_miners = chain.state.miners.clone();
        altered_miners.get_mut(&miner).unwrap().stake += 1;
        assert_ne!(miner_root(&chain.state.miners), miner_root(&altered_miners));

        let mut altered_receipts = chain.state.receipts.clone();
        match altered_receipts.get_mut(&receipt.receipt_id).unwrap() {
            ReceiptState::TensorOp(receipt) => receipt.execution_time_ms += 1,
            ReceiptState::LinearTrainingStep(_) => unreachable!("test inserts tensor op receipt"),
        }
        assert_ne!(expected_receipt_root, receipt_root(&altered_receipts));
    }

    #[test]
    fn model_transition_enforces_single_sequential_weight_root() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let model_id = hash_bytes(b"test", &[b"model"]);
        let architecture = hash_bytes(b"test", &[b"architecture"]);
        let config = hash_bytes(b"test", &[b"config"]);
        let before = hash_bytes(b"test", &[b"weights-before"]);
        let after = hash_bytes(b"test", &[b"weights-after"]);
        let conflicting = hash_bytes(b"test", &[b"conflicting"]);

        chain.register_model(model_id, architecture, before, config);
        let before_optimizer_root = chain.state_root();
        chain
            .state
            .model_states
            .get_mut(&model_id)
            .unwrap()
            .optimizer_state_root = Some(hash_bytes(b"test", &[b"optimizer"]));
        assert_ne!(before_optimizer_root, chain.state_root());
        chain
            .apply_model_transition(&model_id, 0, &before, after)
            .unwrap();
        assert_eq!(chain.state.model_states.get(&model_id).unwrap().step, 1);
        assert_eq!(
            chain.apply_model_transition(&model_id, 0, &before, conflicting),
            Err(TvmError::InvalidReceipt("model step mismatch"))
        );
        assert_eq!(
            chain.apply_model_transition(&model_id, 1, &before, conflicting),
            Err(TvmError::InvalidReceipt("model weight root mismatch"))
        );
    }

    #[test]
    fn challenge_outcome_slashes_miner_and_credits_treasury() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        let miner = address(b"miner");
        chain.register_miner(miner, 100).unwrap();
        chain
            .apply_challenge_outcome(ChallengeOutcome::ProvenInvalid {
                dishonest_party: miner,
                slash_amount: 25,
                reason: "invalid receipt".to_owned(),
            })
            .unwrap();
        assert_eq!(chain.state.miners.get(&miner).unwrap().stake, 75);
        assert_eq!(chain.state.miners.get(&miner).unwrap().reputation, -10);
        assert_eq!(chain.state.rewards.treasury, 25);
    }

    #[test]
    fn conflicting_linear_training_roots_do_not_settle() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut params = ChainParams::default();
        params.freivalds.minimum_validators = 1;
        params.freivalds.minimum_stake_numerator = 1;
        params.freivalds.minimum_stake_denominator = 1;
        params.agreement_quorum = 1;
        let mut chain = LocalChain::with_params(params, beacon);
        let miner = address(b"miner");
        let validator = address(b"validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();

        let weights = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"model"]),
            step: 0,
            batch_seed: hash_bytes(b"test", &[b"batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![3, 2],
            weight_shape: vec![2, 2],
            target_shape: vec![3, 2],
            lr: 2,
            deadline_block: 20,
        });
        let (receipt, mut output) =
            LinearTrainingStepReceipt::from_job(&job, miner, &weights, 1, 5).unwrap();
        let tensor_job = MatmulJob::synthetic(0, 99, 2, 2, 2, &beacon, 20);
        let (tensor_receipt, _a, _b, _c) =
            TensorOpReceipt::from_job(&tensor_job, miner, 1, 5).unwrap();
        output
            .weight_after
            .set2(0, 0, output.weight_after.get2(0, 0).unwrap() + 1)
            .unwrap();
        let conflicting = LinearTrainingStepReceipt::from_output(&job, miner, &output, 1, 5);
        chain.submit_job(JobState::LinearTrainingStep(job));
        chain.submit_job(JobState::TensorOp(tensor_job));
        chain
            .submit_tensor_op_receipt(tensor_receipt.clone())
            .unwrap();
        chain.submit_linear_receipt(receipt.clone()).unwrap();
        assert!(!chain.has_conflicting_linear_receipt(receipt.receipt_id, &receipt));
        chain.submit_linear_receipt(conflicting.clone()).unwrap();

        for receipt in [&receipt, &conflicting] {
            chain
                .submit_attestation(ValidatorAttestation::new(
                    validator,
                    10_000,
                    AttestationStatement {
                        receipt_id: receipt.receipt_id,
                        job_id: receipt.job_id,
                        primitive_type: PrimitiveType::LinearTrainingStep,
                        result: VerificationResult::Valid,
                        checks_root: hash_bytes(b"test", &[&receipt.receipt_id]),
                        data_availability_passed: true,
                    },
                ))
                .unwrap();
        }

        chain.settle_epoch(1_000, 500);
        assert!(chain.state.settled_receipts.is_empty());
        assert_eq!(chain.state.rewards.balance(&miner), 0);
    }
}
