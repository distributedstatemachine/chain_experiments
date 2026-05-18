use crate::chain::{BlockVote, ChainParams, JobState, LocalChain, TensorBlock, Transaction};
use crate::explorer::ExplorerSummary;
use crate::faucet::Faucet;
use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec};
use crate::miner::MinerNode;
use crate::runtime::CpuReferenceBackend;
use crate::scheduler::JobScheduler;
use crate::telemetry::TelemetrySnapshot;
use crate::tensor::{DType, Tensor};
use crate::tensor_server::TensorServer;
use crate::txpool::TxPool;
use crate::types::{Address, Hash, address, hash_bytes};
use crate::validator::ValidatorNode;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestnetConfig {
    pub miner_count: usize,
    pub validator_count: usize,
    pub miner_stake: u64,
    pub validator_stake: u64,
    pub faucet_balance: u64,
    pub faucet_drip: u64,
}

impl Default for TestnetConfig {
    fn default() -> Self {
        Self {
            miner_count: 10,
            validator_count: 5,
            miner_stake: 100,
            validator_stake: 10_000,
            faucet_balance: 1_000_000,
            faucet_drip: 100,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetCriteria {
    pub min_miners: usize,
    pub min_validators: usize,
    pub duration_days: u64,
    pub min_finality_rate_bps: u64,
    pub min_data_availability_bps: u64,
}

impl Default for PublicTestnetCriteria {
    fn default() -> Self {
        Self {
            min_miners: 10,
            min_validators: 5,
            duration_days: 7,
            min_finality_rate_bps: 10_000,
            min_data_availability_bps: 9_500,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetEvidence {
    pub miner_count: usize,
    pub validator_count: usize,
    pub observed_blocks: u64,
    pub required_blocks: u64,
    pub finality_rate_bps: u64,
    pub data_availability_bps: u64,
    pub external_operator_evidence: bool,
    pub has_required_miners: bool,
    pub has_required_validators: bool,
    pub has_required_block_count: bool,
    pub has_required_finality: bool,
    pub has_required_data_availability: bool,
    pub public_criterion_met: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicNodeRole {
    Miner,
    Validator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicNodeEvidence {
    pub address: Address,
    pub operator_id: Hash,
    pub role: PublicNodeRole,
    pub first_seen_block: u64,
    pub last_seen_block: u64,
    pub signed_heartbeat_count: u64,
}

impl PublicNodeEvidence {
    pub fn miner(
        address: Address,
        operator_id: Hash,
        first_seen_block: u64,
        last_seen_block: u64,
        signed_heartbeat_count: u64,
    ) -> Self {
        Self::new(
            address,
            operator_id,
            PublicNodeRole::Miner,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        )
    }

    pub fn validator(
        address: Address,
        operator_id: Hash,
        first_seen_block: u64,
        last_seen_block: u64,
        signed_heartbeat_count: u64,
    ) -> Self {
        Self::new(
            address,
            operator_id,
            PublicNodeRole::Validator,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        )
    }

    pub fn covers_run(&self, observed_blocks: u64) -> bool {
        observed_blocks == 0
            || (self.first_seen_block == 0
                && self.last_seen_block.saturating_add(1) >= observed_blocks)
    }

    pub fn has_external_operator_proof(&self) -> bool {
        self.operator_id != [0; 32] && self.signed_heartbeat_count > 0
    }

    fn new(
        address: Address,
        operator_id: Hash,
        role: PublicNodeRole,
        first_seen_block: u64,
        last_seen_block: u64,
        signed_heartbeat_count: u64,
    ) -> Self {
        Self {
            address,
            operator_id,
            role,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetRunEvidence {
    pub nodes: Vec<PublicNodeEvidence>,
    pub observed_blocks: u64,
    pub finalized_blocks: u64,
    pub checked_receipts: u64,
    pub available_receipts: u64,
}

impl PublicTestnetRunEvidence {
    pub fn evaluate(
        &self,
        criteria: &PublicTestnetCriteria,
        block_time_seconds: u64,
        external_operator_evidence: bool,
    ) -> PublicTestnetEvidence {
        let (miner_count, validator_count) = self.independent_operator_counts();
        let required_blocks =
            required_blocks_for_days(criteria.duration_days, block_time_seconds.max(1));
        let finality_rate_bps = ratio_parts_to_bps(self.finalized_blocks, self.observed_blocks);
        let data_availability_bps =
            ratio_parts_to_bps(self.available_receipts, self.checked_receipts);
        let has_required_miners = miner_count >= criteria.min_miners;
        let has_required_validators = validator_count >= criteria.min_validators;
        let has_required_block_count = self.observed_blocks >= required_blocks;
        let has_required_finality = finality_rate_bps >= criteria.min_finality_rate_bps;
        let has_required_data_availability =
            data_availability_bps >= criteria.min_data_availability_bps;
        let external_operator_evidence =
            external_operator_evidence && miner_count > 0 && validator_count > 0;
        let public_criterion_met = has_required_miners
            && has_required_validators
            && has_required_block_count
            && has_required_finality
            && has_required_data_availability
            && external_operator_evidence;
        PublicTestnetEvidence {
            miner_count,
            validator_count,
            observed_blocks: self.observed_blocks,
            required_blocks,
            finality_rate_bps,
            data_availability_bps,
            external_operator_evidence,
            has_required_miners,
            has_required_validators,
            has_required_block_count,
            has_required_finality,
            has_required_data_availability,
            public_criterion_met,
        }
    }

    fn independent_operator_counts(&self) -> (usize, usize) {
        let mut miners = BTreeSet::new();
        let mut validators = BTreeSet::new();
        for node in &self.nodes {
            if !node.covers_run(self.observed_blocks) || !node.has_external_operator_proof() {
                continue;
            }
            match node.role {
                PublicNodeRole::Miner => {
                    miners.insert(node.operator_id);
                }
                PublicNodeRole::Validator => {
                    validators.insert(node.operator_id);
                }
            }
        }
        (miners.len(), validators.len())
    }
}

#[derive(Clone, Debug)]
pub struct LocalTestnet {
    pub chain: LocalChain,
    pub faucet: Faucet,
    pub miners: Vec<Address>,
    pub validators: Vec<Address>,
}

impl LocalTestnet {
    pub fn new(config: TestnetConfig, finalized_randomness: Hash) -> Self {
        let params = ChainParams::default();
        let mut chain = LocalChain::with_params(params, finalized_randomness);
        let mut miners = Vec::with_capacity(config.miner_count);
        let mut validators = Vec::with_capacity(config.validator_count);
        for i in 0..config.miner_count {
            let miner = address(format!("testnet-miner-{i}").as_bytes());
            chain.register_miner(miner, config.miner_stake).unwrap();
            miners.push(miner);
        }
        for i in 0..config.validator_count {
            let validator = address(format!("testnet-validator-{i}").as_bytes());
            chain
                .register_validator(validator, config.validator_stake)
                .unwrap();
            validators.push(validator);
        }
        Self {
            chain,
            faucet: Faucet::new(config.faucet_balance, config.faucet_drip),
            miners,
            validators,
        }
    }

    pub fn run_blocks(&mut self, count: u64) {
        for i in 0..count {
            let beacon = self.chain.state.finalized_randomness;
            let proposer = self
                .chain
                .proposer_for_next_epoch(&beacon)
                .or_else(|| self.miners.first().copied())
                .or_else(|| self.validators.first().copied())
                .unwrap_or([0; 32]);
            let timestamp = i.saturating_mul(self.chain.params.block_time_seconds);
            let block = self.chain.produce_block(proposer, timestamp);
            self.finalize_block(&block);
        }
    }

    pub fn run_matmul_round(&mut self, scheduler: &JobScheduler) {
        let beacon = self.chain.state.finalized_randomness;
        let job = scheduler.generate_small_matmul(
            self.chain.state.epoch,
            self.chain.state.height,
            &beacon,
            self.chain.state.height + self.chain.params.receipt_submission_window,
        );
        let mut txpool = TxPool::default();
        self.chain.submit_job(JobState::TensorOp(job.clone()));
        let miner_assignment = scheduler.assign_miners(&self.chain, job.job_id, &beacon);
        let mut receipts = Vec::new();
        for (index, miner_address) in miner_assignment.miners.iter().copied().enumerate() {
            let mut miner = MinerNode::new(miner_address, CpuReferenceBackend);
            let (receipt, _a, _b, _c) = miner
                .solve_matmul_job(&job, self.chain.state.height, 1 + index as u64)
                .expect("reference miner should solve generated job");
            assert!(txpool.submit(Transaction::SubmitTensorOpReceipt(receipt.receipt_id)));
            self.chain
                .submit_tensor_op_receipt(receipt.clone())
                .expect("registered miner receipt should be accepted");
            receipts.push((receipt, miner.tensor_server.clone()));
        }

        self.attest_matmul_receipts(scheduler, &job, &receipts, &beacon, &mut txpool);

        self.chain.settle_epoch(1_000, 500);
        let proposer = self
            .chain
            .proposer_for_next_epoch(&beacon)
            .unwrap_or_else(|| self.miners[0]);
        let block = self.chain.produce_block(
            proposer,
            self.chain.state.height * self.chain.params.block_time_seconds,
        );
        self.finalize_block(&block);
    }

    pub fn run_linear_training_round(&mut self, scheduler: &JobScheduler) {
        let beacon = self.chain.state.finalized_randomness;
        let model_id = hash_bytes(b"tensor-vm-testnet-model-v1", &[&beacon]);
        let architecture = hash_bytes(b"tensor-vm-testnet-architecture-v1", &[]);
        let config = hash_bytes(b"tensor-vm-testnet-config-v1", &[]);
        let weights = Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6])
            .expect("static weights should be valid");
        self.chain
            .register_model(model_id, architecture, weights.commitment_root(), config);
        let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id,
            step: 0,
            batch_seed: hash_bytes(b"tensor-vm-testnet-batch-v1", &[&beacon]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 2,
            deadline_block: self.chain.state.height + self.chain.params.receipt_submission_window,
        });
        let mut txpool = TxPool::default();
        self.chain
            .submit_job(JobState::LinearTrainingStep(job.clone()));
        let miner_assignment = scheduler.assign_miners(&self.chain, job.job_id, &beacon);
        let mut receipts = Vec::new();
        for (index, miner_address) in miner_assignment.miners.iter().copied().enumerate() {
            let mut miner = MinerNode::new(miner_address, CpuReferenceBackend);
            let (receipt, output) = miner
                .solve_linear_training_step(
                    &job,
                    &weights,
                    self.chain.state.height,
                    1 + index as u64,
                )
                .expect("reference miner should solve generated training step");
            assert!(txpool.submit(Transaction::SubmitLinearTrainingStepReceipt(
                receipt.receipt_id
            )));
            self.chain
                .submit_linear_receipt(receipt.clone())
                .expect("registered miner linear receipt should be accepted");
            receipts.push((receipt, output));
        }

        for (receipt, output) in &receipts {
            let validation_seed = self.chain.validation_seed(&receipt.receipt_id);
            let assignment = scheduler.assign_validators(&self.chain, receipt.receipt_id, &beacon);
            for validator_address in assignment.validators {
                let stake = self
                    .chain
                    .state
                    .validators
                    .get(&validator_address)
                    .map(|validator| validator.stake)
                    .unwrap_or_default();
                let validator = ValidatorNode::new(validator_address, stake);
                let attestation = validator
                    .verify_linear_training_step(
                        &job,
                        receipt,
                        &weights,
                        output,
                        &validation_seed,
                        &self.chain.params.freivalds,
                    )
                    .expect("reference validator should verify generated training step");
                assert!(txpool.submit(Transaction::SubmitAttestation(attestation.receipt_id)));
                self.chain
                    .submit_attestation(attestation)
                    .expect("registered validator attestation should be accepted");
            }
        }

        let canonical_receipt = &receipts[0].0;
        assert!(
            self.chain
                .has_attestation_quorum(&canonical_receipt.receipt_id)
        );
        assert!(
            self.chain
                .has_redundant_agreement(&canonical_receipt.receipt_id)
        );
        self.chain.settle_epoch(1_000, 500);
        assert!(
            self.chain
                .state
                .settled_receipts
                .contains(&canonical_receipt.receipt_id)
        );
        self.chain
            .apply_model_transition(
                &model_id,
                0,
                &weights.commitment_root(),
                canonical_receipt.weight_root_after,
            )
            .expect("verified training receipt should advance model state");
        let proposer = self
            .chain
            .proposer_for_next_epoch(&beacon)
            .unwrap_or_else(|| self.miners[0]);
        let block = self.chain.produce_block(
            proposer,
            self.chain.state.height * self.chain.params.block_time_seconds,
        );
        self.finalize_block(&block);
    }

    pub fn expected_blocks_for_days(&self, days: u64) -> u64 {
        required_blocks_for_days(days, self.chain.params.block_time_seconds.max(1))
    }

    pub fn telemetry(&self) -> TelemetrySnapshot {
        TelemetrySnapshot::from_chain(&self.chain)
    }

    pub fn explorer_summary(&self) -> ExplorerSummary {
        ExplorerSummary::from_chain(&self.chain)
    }

    pub fn public_testnet_evidence(
        &self,
        criteria: &PublicTestnetCriteria,
        external_operator_evidence: bool,
    ) -> PublicTestnetEvidence {
        let telemetry = self.telemetry();
        let required_blocks = self.expected_blocks_for_days(criteria.duration_days);
        let observed_blocks = self.chain.blocks.len() as u64;
        let finality_rate_bps = ratio_to_bps(telemetry.block_finality_rate);
        let data_availability_bps = ratio_to_bps(telemetry.data_availability_rate);
        let has_required_miners = self.miners.len() >= criteria.min_miners;
        let has_required_validators = self.validators.len() >= criteria.min_validators;
        let has_required_block_count = observed_blocks >= required_blocks;
        let has_required_finality = finality_rate_bps >= criteria.min_finality_rate_bps;
        let has_required_data_availability =
            data_availability_bps >= criteria.min_data_availability_bps;
        let public_criterion_met = has_required_miners
            && has_required_validators
            && has_required_block_count
            && has_required_finality
            && has_required_data_availability
            && external_operator_evidence;
        PublicTestnetEvidence {
            miner_count: self.miners.len(),
            validator_count: self.validators.len(),
            observed_blocks,
            required_blocks,
            finality_rate_bps,
            data_availability_bps,
            external_operator_evidence,
            has_required_miners,
            has_required_validators,
            has_required_block_count,
            has_required_finality,
            has_required_data_availability,
            public_criterion_met,
        }
    }

    fn attest_matmul_receipts(
        &mut self,
        scheduler: &JobScheduler,
        job: &crate::jobs::MatmulJob,
        receipts: &[(crate::jobs::TensorOpReceipt, TensorServer)],
        beacon: &Hash,
        txpool: &mut TxPool,
    ) {
        for (receipt, tensor_server) in receipts {
            let validation_seed = self.chain.validation_seed(&receipt.receipt_id);
            let assignment = scheduler.assign_validators(&self.chain, receipt.receipt_id, beacon);
            for validator_address in assignment.validators {
                let stake = self
                    .chain
                    .state
                    .validators
                    .get(&validator_address)
                    .map(|validator| validator.stake)
                    .unwrap_or_default();
                let validator = ValidatorNode::new(validator_address, stake);
                let attestation = validator
                    .verify_matmul_from_server(
                        job,
                        receipt,
                        tensor_server,
                        &validation_seed,
                        &self.chain.params.freivalds,
                    )
                    .expect("reference validator should verify generated job");
                assert!(txpool.submit(Transaction::SubmitAttestation(attestation.receipt_id)));
                self.chain
                    .submit_attestation(attestation)
                    .expect("registered validator attestation should be accepted");
            }
        }
    }

    fn finalize_block(&mut self, block: &TensorBlock) {
        for validator in self.validators.clone() {
            let stake = self
                .chain
                .state
                .validators
                .get(&validator)
                .map(|validator| validator.stake)
                .unwrap_or_default();
            self.chain
                .submit_block_vote(BlockVote::new(validator, stake, block))
                .expect("registered validator vote should finalize local block");
            if self.chain.is_block_finalized(&block.hash()) {
                break;
            }
        }
    }
}

fn ratio_to_bps(value: f64) -> u64 {
    (value.clamp(0.0, 1.0) * 10_000.0).round() as u64
}

fn ratio_parts_to_bps(numerator: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        return 0;
    }
    let numerator = u128::from(numerator.min(denominator));
    let denominator = u128::from(denominator);
    (((numerator * 10_000) + (denominator / 2)) / denominator) as u64
}

fn required_blocks_for_days(days: u64, block_time_seconds: u64) -> u64 {
    days.saturating_mul(24)
        .saturating_mul(60)
        .saturating_mul(60)
        / block_time_seconds.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::hash_bytes;

    #[test]
    fn local_testnet_bootstraps_required_public_shape() {
        let mut testnet =
            LocalTestnet::new(TestnetConfig::default(), hash_bytes(b"test", &[b"beacon"]));
        assert_eq!(testnet.miners.len(), 10);
        assert_eq!(testnet.validators.len(), 5);
        testnet.run_blocks(12);
        let summary = testnet.explorer_summary();
        assert_eq!(summary.block_count, 12);
        assert_eq!(testnet.expected_blocks_for_days(7), 100_800);
        assert_eq!(testnet.telemetry().block_finality_rate, 1.0);
        let public_evidence =
            testnet.public_testnet_evidence(&PublicTestnetCriteria::default(), false);
        assert_eq!(public_evidence.required_blocks, 100_800);
        assert!(public_evidence.has_required_miners);
        assert!(public_evidence.has_required_validators);
        assert!(!public_evidence.has_required_block_count);
        assert!(!public_evidence.external_operator_evidence);
        assert!(!public_evidence.public_criterion_met);
    }

    #[test]
    fn local_testnet_runs_full_matmul_receipt_attestation_settlement_round() {
        let mut testnet =
            LocalTestnet::new(TestnetConfig::default(), hash_bytes(b"test", &[b"beacon"]));
        let scheduler = JobScheduler::with_small_shape((8, 8, 8));
        testnet.run_matmul_round(&scheduler);

        assert_eq!(
            testnet.chain.state.receipts.len(),
            testnet.chain.params.replication_factor
        );
        assert_eq!(
            testnet.chain.state.settled_receipts.len(),
            testnet.chain.params.replication_factor
        );
        assert_eq!(testnet.chain.blocks.len(), 1);
        assert!(testnet.telemetry().total_tensor_work > 0);
        let rewarded_miners = testnet
            .miners
            .iter()
            .filter(|miner| testnet.chain.state.rewards.balance(miner) > 0)
            .count();
        assert!(rewarded_miners >= testnet.chain.params.agreement_quorum);

        let evidence = testnet.public_testnet_evidence(
            &PublicTestnetCriteria {
                duration_days: 0,
                min_finality_rate_bps: 10_000,
                min_data_availability_bps: 9_500,
                ..PublicTestnetCriteria::default()
            },
            true,
        );
        assert_eq!(evidence.observed_blocks, 1);
        assert_eq!(evidence.required_blocks, 0);
        assert_eq!(evidence.finality_rate_bps, 10_000);
        assert_eq!(evidence.data_availability_bps, 10_000);
        assert!(evidence.public_criterion_met);
    }

    #[test]
    fn local_testnet_runs_linear_training_receipt_state_transition_round() {
        let mut testnet =
            LocalTestnet::new(TestnetConfig::default(), hash_bytes(b"test", &[b"beacon"]));
        let scheduler = JobScheduler::with_small_shape((8, 8, 8));
        testnet.run_linear_training_round(&scheduler);

        assert_eq!(
            testnet.chain.state.receipts.len(),
            testnet.chain.params.replication_factor
        );
        assert_eq!(
            testnet.chain.state.settled_receipts.len(),
            testnet.chain.params.replication_factor
        );
        assert_eq!(testnet.chain.blocks.len(), 1);
        assert_eq!(testnet.chain.state.model_states.len(), 1);
        assert_eq!(
            testnet
                .chain
                .state
                .model_states
                .values()
                .next()
                .unwrap()
                .step,
            1
        );
        let rewarded_miners = testnet
            .miners
            .iter()
            .filter(|miner| testnet.chain.state.rewards.balance(miner) > 0)
            .count();
        assert!(rewarded_miners >= testnet.chain.params.agreement_quorum);
    }

    #[test]
    fn public_testnet_run_evidence_requires_independent_external_operators() {
        let criteria = PublicTestnetCriteria {
            min_miners: 2,
            min_validators: 1,
            duration_days: 0,
            min_finality_rate_bps: 9_000,
            min_data_availability_bps: 9_500,
        };
        let shared_operator = hash_bytes(b"test", &[b"shared-operator"]);
        let validator_operator = hash_bytes(b"test", &[b"validator-operator"]);
        let mut run = PublicTestnetRunEvidence {
            nodes: vec![
                PublicNodeEvidence::miner(address(b"miner-a"), shared_operator, 0, 9, 10),
                PublicNodeEvidence::miner(address(b"miner-b"), shared_operator, 0, 9, 10),
                PublicNodeEvidence::validator(
                    address(b"validator-a"),
                    validator_operator,
                    0,
                    9,
                    10,
                ),
            ],
            observed_blocks: 10,
            finalized_blocks: 10,
            checked_receipts: 20,
            available_receipts: 19,
        };

        let insufficient = run.evaluate(&criteria, 6, true);
        assert_eq!(insufficient.miner_count, 1);
        assert_eq!(insufficient.validator_count, 1);
        assert_eq!(insufficient.required_blocks, 0);
        assert_eq!(insufficient.finality_rate_bps, 10_000);
        assert_eq!(insufficient.data_availability_bps, 9_500);
        assert!(insufficient.external_operator_evidence);
        assert!(!insufficient.has_required_miners);
        assert!(!insufficient.public_criterion_met);

        run.nodes[1].operator_id = hash_bytes(b"test", &[b"miner-b-operator"]);
        let no_external_flag = run.evaluate(&criteria, 6, false);
        assert!(!no_external_flag.external_operator_evidence);
        assert!(!no_external_flag.public_criterion_met);

        let sufficient = run.evaluate(&criteria, 6, true);
        assert_eq!(sufficient.miner_count, 2);
        assert!(sufficient.has_required_miners);
        assert!(sufficient.has_required_validators);
        assert!(sufficient.has_required_block_count);
        assert!(sufficient.has_required_finality);
        assert!(sufficient.has_required_data_availability);
        assert!(sufficient.public_criterion_met);
    }

    #[test]
    fn public_testnet_run_evidence_filters_unsigned_and_short_lived_nodes() {
        let criteria = PublicTestnetCriteria {
            min_miners: 1,
            min_validators: 1,
            duration_days: 1,
            min_finality_rate_bps: 1,
            min_data_availability_bps: 1,
        };
        let run = PublicTestnetRunEvidence {
            nodes: vec![
                PublicNodeEvidence::miner(
                    address(b"unsigned-miner"),
                    hash_bytes(b"test", &[b"unsigned-miner-operator"]),
                    0,
                    9,
                    0,
                ),
                PublicNodeEvidence::miner(
                    address(b"late-miner"),
                    hash_bytes(b"test", &[b"late-miner-operator"]),
                    1,
                    9,
                    8,
                ),
                PublicNodeEvidence::validator(
                    address(b"zero-operator-validator"),
                    [0; 32],
                    0,
                    9,
                    10,
                ),
            ],
            observed_blocks: 10,
            finalized_blocks: 11,
            checked_receipts: 0,
            available_receipts: 0,
        };

        assert!(run.nodes[0].covers_run(0));
        assert!(!run.nodes[0].has_external_operator_proof());
        assert!(!run.nodes[1].covers_run(run.observed_blocks));
        assert!(!run.nodes[2].has_external_operator_proof());

        let report = run.evaluate(&criteria, 6, true);
        assert_eq!(report.miner_count, 0);
        assert_eq!(report.validator_count, 0);
        assert_eq!(report.required_blocks, 14_400);
        assert_eq!(report.finality_rate_bps, 10_000);
        assert_eq!(report.data_availability_bps, 0);
        assert!(!report.external_operator_evidence);
        assert!(!report.public_criterion_met);
    }
}
