use crate::challenge::ChallengeOutcome;
use crate::error::Result;
#[cfg(test)]
use crate::error::TvmError;
#[cfg(test)]
use crate::jobs::PrimitiveType;
use crate::jobs::{LinearTrainingStepReceipt, TensorOpReceipt};
use crate::types::{Address, Hash};
use crate::verify::ValidatorAttestation;

mod accounts;
mod blocks;
mod challenges;
mod commands;
mod engine;
mod genesis;
mod models;
mod operators;
mod proposer;
mod receipts;
mod roots;
mod settlement;
mod state;
#[cfg(test)]
mod test_helpers;
mod transactions;
mod validation;

pub use engine::{BlockAdmission, BlockInvalidReason, ChainCommand, ChainEngine, ChainEvent};
#[cfg(test)]
use settlement::{has_conflicting_linear_receipt, receipts_agree};
pub use state::{
    AccountState, BlockVote, BlockspaceCaps, BlockspaceSelection, Chain, ChainParams, ChainState,
    HardwareClass, JobState, MinerState, ModelState, ReceiptState, RewardAllocation, RewardState,
    TensorBlock, Transaction, ValidatorState,
};
pub(crate) use state::{ChainParts, ChainStateParts};

impl Chain {
    pub fn new(finalized_randomness: Hash) -> Self {
        Self::with_params(ChainParams::default(), finalized_randomness)
    }

    pub fn with_params(params: ChainParams, finalized_randomness: Hash) -> Self {
        genesis::with_params(params, finalized_randomness)
    }

    pub fn params(&self) -> &ChainParams {
        &self.params
    }

    pub fn state(&self) -> &ChainState {
        &self.state
    }

    pub fn blocks(&self) -> &[TensorBlock] {
        &self.blocks
    }

    pub fn register_miner(&mut self, address: Address, stake: u64) -> Result<()> {
        operators::register_miner(self, address, stake)
    }

    pub fn register_miner_with_operator(
        &mut self,
        address: Address,
        stake: u64,
        operator_id: Hash,
    ) -> Result<()> {
        operators::register_miner_with_operator(self, address, stake, operator_id)
    }

    pub fn register_miner_with_profile(
        &mut self,
        address: Address,
        stake: u64,
        hardware_class: HardwareClass,
        gpu_utilization_bps: u64,
    ) -> Result<()> {
        operators::register_miner_with_profile(
            self,
            address,
            stake,
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
        operators::register_miner_with_profile_and_operator(
            self,
            address,
            stake,
            operator_id,
            hardware_class,
            gpu_utilization_bps,
        )
    }

    pub fn register_validator(&mut self, address: Address, stake: u64) -> Result<()> {
        operators::register_validator(self, address, stake)
    }

    pub fn credit_account(&mut self, address: Address, amount: u64) {
        accounts::credit(self, address, amount);
    }

    pub fn transfer(&mut self, from: Address, to: Address, amount: u64) -> Result<()> {
        accounts::transfer(self, from, to, amount)
    }

    pub fn submit_job(&mut self, job: JobState) {
        receipts::submit_job(self, job);
    }

    pub fn job(&self, job_id: &Hash) -> Option<&JobState> {
        receipts::job(self, job_id)
    }

    pub fn submit_tensor_op_receipt(&mut self, receipt: TensorOpReceipt) -> Result<()> {
        receipts::submit_tensor_op(self, receipt)
    }

    pub fn submit_linear_receipt(&mut self, receipt: LinearTrainingStepReceipt) -> Result<()> {
        receipts::submit_linear_training_step(self, receipt)
    }

    pub fn apply_transaction(
        &mut self,
        from: Option<Address>,
        tx: Transaction,
    ) -> Result<Vec<ChainEvent>> {
        transactions::apply(self, from, tx)
    }

    pub fn submit_attestation(&mut self, attestation: ValidatorAttestation) -> Result<()> {
        validation::submit_attestation(self, attestation)
    }

    pub fn has_attestation_quorum(&self, receipt_id: &Hash) -> bool {
        validation::has_attestation_quorum(self, receipt_id)
    }

    pub fn redundant_agreement_count(&self, receipt_id: &Hash) -> usize {
        settlement::redundant_agreement_count(self, receipt_id)
    }

    pub fn has_redundant_agreement(&self, receipt_id: &Hash) -> bool {
        settlement::has_redundant_agreement(self, receipt_id)
    }

    pub fn submit_block_vote(&mut self, vote: BlockVote) -> Result<()> {
        validation::submit_block_vote(self, vote)
    }

    pub fn has_block_finality(&self, block_hash: &Hash) -> bool {
        validation::has_block_finality(self, block_hash)
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
    ) -> Result<()> {
        models::register(self, model_id, architecture_hash, weight_root, config_hash)
    }

    pub fn apply_model_transition(
        &mut self,
        model_id: &Hash,
        step: u64,
        weight_root_before: &Hash,
        weight_root_after: Hash,
    ) -> Result<()> {
        models::apply_transition(self, model_id, step, weight_root_before, weight_root_after)
    }

    pub fn apply_challenge_outcome(&mut self, outcome: ChallengeOutcome) -> Result<()> {
        self.apply_command(ChainCommand::ApplyChallengeOutcome(outcome))
            .map(|_| ())
    }

    pub fn validation_seed(&self, receipt_id: &Hash) -> Hash {
        validation::seed(&self.state.finalized_randomness, receipt_id)
    }

    pub fn settle_epoch(&mut self, miner_reward_pool: u64, validator_reward_pool: u64) {
        settlement::settle_epoch(self, miner_reward_pool, validator_reward_pool);
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
        self.state
            .rewards
            .credit_treasury(allocation.treasury_reward);
    }

    pub fn proposer_for_next_epoch(&self, beacon: &Hash) -> Option<Address> {
        proposer::for_next_epoch(&self.state, beacon)
    }

    pub fn produce_block(&mut self, proposer: Address, timestamp: u64) -> Result<TensorBlock> {
        blocks::produce(self, proposer, timestamp)
    }

    pub fn produce_block_with_rewards(
        &mut self,
        proposer: Address,
        timestamp: u64,
        fixed_block_reward: u64,
        fee_share: u64,
    ) -> Result<TensorBlock> {
        blocks::produce_with_rewards(self, proposer, timestamp, fixed_block_reward, fee_share)
    }

    pub fn prepare_block_parent_state(&mut self) -> Result<()> {
        blocks::prepare_parent_state(self)
    }

    pub fn admit_block(&mut self, block: TensorBlock) -> Result<BlockAdmission> {
        blocks::admit(self, block)
    }

    pub fn blockspace_caps(&self) -> BlockspaceCaps {
        blocks::blockspace_caps()
    }

    pub fn canonical_blockspace(&self, parent_hash: &Hash, beacon: &Hash) -> BlockspaceSelection {
        blocks::canonical_blockspace(&self.state, parent_hash, beacon, self.blockspace_caps())
    }

    pub fn selected_receipts_for_block(&self, block: &TensorBlock) -> Vec<Hash> {
        blocks::selected_receipts(self, block)
    }

    pub fn validate_block(&self, block: &TensorBlock) -> Result<()> {
        blocks::validate(self, block, true)
    }

    pub fn state_root(&self) -> Hash {
        roots::state_root(&self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::roots::{
        attestation_root, block_checks_root, miner_root, receipt_root, reward_root,
        selected_receipt_root,
    };
    use super::*;
    use crate::jobs::{
        LinearTrainingStepJob, LinearTrainingStepReceipt, LinearTrainingStepSpec, MatmulJob,
        TensorOpReceipt,
    };
    use crate::scheduler::JobScheduler;
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes, sign};
    use crate::verify::{
        AttestationStatement, FreivaldsParams, ValidatorAttestation, VerificationResult,
        verify_tensor_op,
    };
    use std::collections::BTreeSet;

    fn resign_test_block(block: &mut TensorBlock) {
        let block_hash = block.hash();
        block.proposer_signature = sign(&block.proposer, &block_hash);
        block.validator_signature_aggregate =
            hash_bytes(b"tensor-vm-validator-aggregate", &[&block_hash]);
    }

    fn mine_test_block(block: &mut TensorBlock) {
        while !block.pow_valid() {
            block.nonce = block.nonce.saturating_add(1);
        }
        resign_test_block(block);
    }

    mod commands;
    mod settlement;

    #[test]
    fn chain_tracks_accounts_jobs_and_transfers() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
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
        let mut chain = Chain::new(beacon);
        let proposer = address(b"reward-proposer");
        chain
            .register_validator(proposer, chain.params.validator_min_stake)
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

        let block = chain
            .produce_block_with_rewards(proposer, 1_000, 400, 100)
            .unwrap();
        assert_eq!(chain.state.rewards.balance(&proposer), 500);
        assert_eq!(block.reward_root, reward_root(&chain.state.rewards));

        chain.settle_epoch_rewards(allocation, proposer);
        assert_eq!(chain.state.rewards.balance(&proposer), 1_000);
        assert_eq!(chain.state.rewards.treasury(), 500);
    }

    #[test]
    fn reward_block_production_failure_does_not_credit_proposer() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let proposer = address(b"unknown-reward-proposer");
        let rewards_before = chain.state.rewards.clone();

        assert_eq!(
            chain.produce_block_with_rewards(proposer, 1_000, 400, 100),
            Err(TvmError::UnknownValidator)
        );
        assert_eq!(chain.state.rewards, rewards_before);
        assert!(chain.blocks.is_empty());
    }

    #[test]
    fn validation_seed_is_bound_to_finalized_randomness_and_receipt() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let chain = Chain::new(beacon);
        let receipt_a = hash_bytes(b"test", &[b"receipt-a"]);
        let receipt_b = hash_bytes(b"test", &[b"receipt-b"]);
        assert_ne!(
            chain.validation_seed(&receipt_a),
            chain.validation_seed(&receipt_b)
        );

        let other_chain = Chain::new(hash_bytes(b"test", &[b"other-beacon"]));
        assert_ne!(
            chain.validation_seed(&receipt_a),
            other_chain.validation_seed(&receipt_a)
        );
    }

    #[test]
    fn chain_applies_register_transfer_and_claim_reward_transactions() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let miner = address(b"miner-tx");
        let validator = address(b"validator-tx");
        let receiver = address(b"receiver");
        assert_eq!(
            chain
                .apply_transaction(None, Transaction::RegisterMiner(miner))
                .unwrap(),
            vec![ChainEvent::MinerRegistered(miner)]
        );
        assert_eq!(
            chain
                .apply_transaction(None, Transaction::RegisterValidator(validator))
                .unwrap(),
            vec![ChainEvent::ValidatorRegistered(validator)]
        );
        assert!(chain.state.miners.contains_key(&miner));
        assert!(chain.state.validators.contains_key(&validator));

        chain.credit_account(miner, 500);
        assert_eq!(
            chain
                .apply_transaction(
                    Some(miner),
                    Transaction::Transfer {
                        to: receiver,
                        amount: 125,
                    },
                )
                .unwrap(),
            vec![ChainEvent::AccountTransferred {
                from: miner,
                to: receiver,
                amount: 125,
            }]
        );
        assert_eq!(chain.state.accounts.get(&receiver).unwrap().balance, 125);

        chain.state.rewards.credit(miner, 42);
        assert_eq!(
            chain
                .apply_transaction(None, Transaction::ClaimReward(miner))
                .unwrap(),
            vec![ChainEvent::RewardClaimed {
                address: miner,
                amount: 42,
            }]
        );
        assert_eq!(chain.state.rewards.balance(&miner), 0);
        assert_eq!(chain.state.accounts.get(&miner).unwrap().balance, 417);
    }
    #[test]
    fn reference_submission_transactions_are_txpool_only() {
        let beacon = hash_bytes(b"test", &[b"reference-submission-txpool-only"]);
        let mut chain = Chain::new(beacon);
        for tx in [
            Transaction::SubmitTensorOpReceipt(hash_bytes(b"test", &[b"queued-tensor-receipt"])),
            Transaction::SubmitLinearTrainingStepReceipt(hash_bytes(
                b"test",
                &[b"queued-linear-receipt"],
            )),
            Transaction::SubmitAttestation(hash_bytes(b"test", &[b"queued-attestation"])),
        ] {
            assert!(tx.is_reference_submission());
            assert_eq!(
                chain.apply_transaction(None, tx),
                Err(TvmError::InvalidReceipt(
                    "reference submissions must enter the transaction pool"
                ))
            );
        }
    }

    #[test]
    fn miner_root_commits_to_operator_identity() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
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
        let mut chain = Chain::new(beacon);
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

        let block = chain.produce_block(validator, 1_000).unwrap();
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
    fn proposer_selection_uses_validator_stake() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let validator = address(b"validator");
        chain.register_validator(validator, 10_000).unwrap();
        assert_eq!(chain.proposer_for_next_epoch(&beacon), Some(validator));
    }

    #[test]
    fn fallback_proposer_handles_zero_stake_validator_records() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let validator = address(b"zero-stake-validator");
        chain.register_validator(validator, 10_000).unwrap();
        chain.state.validators.get_mut(&validator).unwrap().stake = 0;

        assert_eq!(chain.proposer_for_next_epoch(&beacon), Some(validator));
    }

    #[test]
    fn proposer_selection_ignores_tensorwork() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let miner = address(b"settled-miner");
        let validator = address(b"validator-proposer");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();
        chain
            .state
            .miners
            .get_mut(&miner)
            .unwrap()
            .settled_tensor_work = 1_000_000;
        chain
            .state
            .miners
            .get_mut(&miner)
            .unwrap()
            .pending_tensor_work = 1_000_000;

        assert_eq!(chain.proposer_for_next_epoch(&beacon), Some(validator));
        assert_eq!(
            chain.produce_block(miner, 1_000),
            Err(TvmError::UnknownValidator)
        );
    }

    #[test]
    fn blocks_advance_height_and_commit_state() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let proposer = address(b"proposer");
        chain.register_validator(proposer, 10_000).unwrap();
        let block = chain.produce_block(proposer, 1_000).unwrap();
        assert_eq!(block.height, 0);
        assert_eq!(chain.state.height, 1);
        assert_eq!(chain.blocks.len(), 1);
    }

    #[test]
    fn block_finality_requires_two_thirds_validator_stake() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let validators: Vec<_> = (0..3)
            .map(|i| address(format!("finality-validator-{i}").as_bytes()))
            .collect();
        for validator in &validators {
            chain.register_validator(*validator, 10_000).unwrap();
        }
        let block = chain.produce_block(validators[0], 1_000).unwrap();
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
        assert!(!Chain::new(beacon).has_block_finality(&hash_bytes(b"test", &[b"no-stake"])));

        let mut chain = Chain::new(beacon);
        let validators: Vec<_> = (0..3)
            .map(|i| address(format!("invalid-finality-validator-{i}").as_bytes()))
            .collect();
        for validator in &validators {
            chain.register_validator(*validator, 10_000).unwrap();
        }
        let block = chain.produce_block(validators[0], 1_000).unwrap();
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
    fn block_votes_reject_invalid_useful_pow_and_checks_root() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let validator = address(b"block-validity-validator");
        chain.register_validator(validator, 10_000).unwrap();
        let block = chain.produce_block(validator, 1_000).unwrap();

        let mut bad_target = block.clone();
        bad_target.difficulty_target = [0; 32];
        resign_test_block(&mut bad_target);
        chain.blocks.push(bad_target.clone());
        assert_eq!(
            chain.submit_block_vote(BlockVote::new(validator, 10_000, &bad_target)),
            Err(TvmError::InvalidReceipt("block difficulty target mismatch"))
        );
        chain.blocks.pop();

        let mut bad_checks = block.clone();
        bad_checks.checks_root = hash_bytes(b"test", &[b"bad-block-checks"]);
        mine_test_block(&mut bad_checks);
        chain.blocks.push(bad_checks.clone());
        assert_eq!(
            chain.submit_block_vote(BlockVote::new(validator, 10_000, &bad_checks)),
            Err(TvmError::InvalidReceipt("block checks root mismatch"))
        );
        chain.blocks.pop();

        let mut bad_state_root = block.clone();
        bad_state_root.state_root = hash_bytes(b"test", &[b"bad-block-state-root"]);
        mine_test_block(&mut bad_state_root);
        chain.blocks.push(bad_state_root.clone());
        assert_eq!(
            chain.submit_block_vote(BlockVote::new(validator, 10_000, &bad_state_root)),
            Err(TvmError::InvalidReceipt("block state root mismatch"))
        );
        chain.blocks.pop();

        let mut bad_receipts = block.clone();
        bad_receipts.settled_receipt_set_root = hash_bytes(b"test", &[b"bad-receipt-set"]);
        mine_test_block(&mut bad_receipts);
        chain.blocks.push(bad_receipts.clone());
        assert_eq!(
            chain.submit_block_vote(BlockVote::new(validator, 10_000, &bad_receipts)),
            Err(TvmError::InvalidReceipt("noncanonical settled receipt set"))
        );
    }

    #[test]
    fn produced_blocks_mark_selected_settled_receipts_included_once() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let miner = address(b"included-receipt-miner");
        let validator = address(b"included-receipt-validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();

        let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        chain
            .state
            .receipts
            .insert(receipt.receipt_id, ReceiptState::TensorOp(receipt.clone()));
        chain.state.settled_receipts.insert(receipt.receipt_id);

        let first = chain.produce_block(validator, 1_000).unwrap();
        assert_eq!(
            chain.selected_receipts_for_block(&first),
            vec![receipt.receipt_id]
        );
        assert!(chain.state.included_receipts.contains(&receipt.receipt_id));

        let second = chain.produce_block(validator, 2_000).unwrap();
        assert!(chain.selected_receipts_for_block(&second).is_empty());
        assert_eq!(
            second.settled_receipt_set_root,
            selected_receipt_root(&BTreeSet::new())
        );
    }

    #[test]
    fn block_roots_commit_to_canonical_receipts_checks_attestations_and_state_values() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
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

        chain.state.settled_receipts.insert(receipt.receipt_id);
        let parent_hash = chain
            .blocks
            .last()
            .map(TensorBlock::hash)
            .unwrap_or([0; 32]);
        let expected_selection =
            chain.canonical_blockspace(&parent_hash, &chain.state.finalized_randomness);
        let expected_settled_receipt_set_root =
            selected_receipt_root(&expected_selection.receipt_set());
        let expected_checks_root =
            block_checks_root(&expected_selection.receipt_ids, &chain.state.attestations);
        let expected_attestation_root = attestation_root(&chain.state.attestations);
        let expected_state_root = chain.state_root();
        let block = chain.produce_block(validator, 1_000).unwrap();
        assert_eq!(
            block.settled_receipt_set_root,
            expected_settled_receipt_set_root
        );
        assert_eq!(block.checks_root, expected_checks_root);
        assert_eq!(block.attestation_root, expected_attestation_root);
        assert_eq!(block.state_root, expected_state_root);
        assert!(block.pow_valid());

        let mut altered_miners = chain.state.miners.clone();
        altered_miners.get_mut(&miner).unwrap().stake += 1;
        assert_ne!(miner_root(&chain.state.miners), miner_root(&altered_miners));

        let mut altered_receipts = chain.state.receipts.clone();
        match altered_receipts.get_mut(&receipt.receipt_id).unwrap() {
            ReceiptState::TensorOp(receipt) => receipt.execution_time_ms += 1,
            ReceiptState::LinearTrainingStep(_) => unreachable!("test inserts tensor op receipt"),
        }
        assert_ne!(
            receipt_root(&chain.state.receipts),
            receipt_root(&altered_receipts)
        );
    }

    #[test]
    fn model_transition_enforces_single_sequential_weight_root() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let model_id = hash_bytes(b"test", &[b"model"]);
        let architecture = hash_bytes(b"test", &[b"architecture"]);
        let config = hash_bytes(b"test", &[b"config"]);
        let before = hash_bytes(b"test", &[b"weights-before"]);
        let after = hash_bytes(b"test", &[b"weights-after"]);
        let conflicting = hash_bytes(b"test", &[b"conflicting"]);

        chain
            .register_model(model_id, architecture, before, config)
            .unwrap();
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
        let transitioned_model = chain.state.model_states.get(&model_id).unwrap().clone();
        assert_eq!(
            chain.register_model(model_id, architecture, before, config),
            Err(TvmError::InvalidReceipt("duplicate model"))
        );
        assert_eq!(
            chain.state.model_states.get(&model_id),
            Some(&transitioned_model)
        );
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
        let mut chain = Chain::new(beacon);
        let miner = address(b"miner");
        chain.register_miner(miner, 100).unwrap();
        assert_eq!(
            chain
                .apply_command(ChainCommand::ApplyChallengeOutcome(
                    ChallengeOutcome::ProvenInvalid {
                        dishonest_party: miner,
                        slash_amount: 25,
                        reason: "invalid receipt".to_owned(),
                    },
                ))
                .unwrap(),
            vec![ChainEvent::ChallengeProvenInvalid {
                dishonest_party: miner,
                slash_amount: 25,
                reason: "invalid receipt".to_owned(),
            }]
        );
        chain
            .apply_challenge_outcome(ChallengeOutcome::ProvenInvalid {
                dishonest_party: miner,
                slash_amount: 5,
                reason: "invalid receipt again".to_owned(),
            })
            .unwrap();
        assert_eq!(chain.state.miners.get(&miner).unwrap().stake, 70);
        assert_eq!(chain.state.miners.get(&miner).unwrap().reputation, -20);
        assert_eq!(chain.state.rewards.treasury(), 30);
    }
}
