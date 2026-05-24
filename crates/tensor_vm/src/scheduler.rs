use crate::chain::{Chain, JobState};
use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec, MatmulJob};
use crate::tensor::{DType, Tensor};
use crate::types::{Address, Hash, hash_bytes, hash_to_u128};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorAssignment {
    pub receipt_id: Hash,
    pub validators: Vec<Address>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinerAssignment {
    pub job_id: Hash,
    pub miners: Vec<Address>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobScheduler {
    pub small_matmul: (usize, usize, usize),
    pub medium_matmul: (usize, usize, usize),
}

pub trait JobSource {
    fn next_job(&mut self, chain: &Chain) -> Option<JobState>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntheticLocalJobSource {
    scheduler: JobScheduler,
}

impl SyntheticLocalJobSource {
    pub fn new(scheduler: JobScheduler) -> Self {
        Self { scheduler }
    }

    pub fn next_matmul_job(&mut self, chain: &Chain) -> MatmulJob {
        self.scheduler.generate_small_matmul(
            chain.state().epoch(),
            chain.state().height(),
            &chain.state().finalized_randomness(),
            chain
                .state()
                .height()
                .saturating_add(chain.params.receipt_submission_window),
        )
    }

    pub fn next_linear_training_job(&mut self, chain: &Chain) -> LinearTrainingStepJob {
        let weights = Self::linear_training_weights();
        let height = chain.state().height().to_le_bytes();
        LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(
                b"tensor-vm-local-linear-model-v1",
                &[&chain.state().finalized_randomness(), &height],
            ),
            step: 0,
            batch_seed: hash_bytes(
                b"tensor-vm-local-linear-batch-v1",
                &[&chain.state().finalized_randomness(), &height],
            ),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 2,
            deadline_block: chain
                .state()
                .height()
                .saturating_add(chain.params.receipt_submission_window),
        })
    }

    pub fn linear_training_weights() -> Tensor {
        Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6])
            .expect("static synthetic linear weights must be valid")
    }

    pub fn linear_training_architecture_hash() -> Hash {
        hash_bytes(b"tensor-vm-local-linear-architecture-v1", &[])
    }

    pub fn linear_training_config_hash() -> Hash {
        hash_bytes(b"tensor-vm-local-linear-config-v1", &[])
    }
}

impl Default for SyntheticLocalJobSource {
    fn default() -> Self {
        Self::new(JobScheduler::with_small_shape((8, 8, 8)))
    }
}

impl JobSource for SyntheticLocalJobSource {
    fn next_job(&mut self, chain: &Chain) -> Option<JobState> {
        if chain.state().height().is_multiple_of(2) {
            Some(JobState::TensorOp(self.next_matmul_job(chain)))
        } else {
            Some(JobState::LinearTrainingStep(
                self.next_linear_training_job(chain),
            ))
        }
    }
}

impl Default for JobScheduler {
    fn default() -> Self {
        Self {
            small_matmul: (1024, 1024, 1024),
            medium_matmul: (4096, 4096, 4096),
        }
    }
}

impl JobScheduler {
    pub fn with_small_shape(shape: (usize, usize, usize)) -> Self {
        Self {
            small_matmul: shape,
            ..Self::default()
        }
    }

    pub fn generate_small_matmul(
        &self,
        epoch: u64,
        nonce: u64,
        beacon: &Hash,
        deadline_block: u64,
    ) -> MatmulJob {
        let (m, k, n) = self.small_matmul;
        MatmulJob::synthetic(epoch, nonce, m, k, n, beacon, deadline_block)
    }

    pub fn assign_validators(
        &self,
        chain: &Chain,
        receipt_id: Hash,
        seed: &Hash,
    ) -> ValidatorAssignment {
        let mut validators: Vec<_> = chain.state().validators().keys().copied().collect();
        validators.sort_by_key(|validator| {
            let draw = hash_bytes(
                b"tensor-vm-validator-assignment-v1",
                &[seed, &receipt_id, validator],
            );
            hash_to_u128(&draw)
        });
        validators.truncate(
            chain
                .params
                .freivalds
                .validators_per_job
                .min(validators.len()),
        );
        ValidatorAssignment {
            receipt_id,
            validators,
        }
    }

    pub fn assign_miners(&self, chain: &Chain, job_id: Hash, seed: &Hash) -> MinerAssignment {
        let mut candidates: Vec<_> = chain.state().miners().values().collect();
        candidates.sort_by_key(|miner| {
            let draw = hash_bytes(
                b"tensor-vm-miner-assignment-v1",
                &[seed, &job_id, &miner.address],
            );
            hash_to_u128(&draw)
        });

        let target = chain.params.replication_factor.min(candidates.len());
        let mut miners = Vec::with_capacity(target);
        let mut selected_addresses = BTreeSet::new();
        let mut selected_operators = BTreeSet::new();

        for miner in &candidates {
            if miners.len() == target {
                break;
            }
            if selected_operators.insert(miner.operator_id) {
                selected_addresses.insert(miner.address);
                miners.push(miner.address);
            }
        }

        for miner in &candidates {
            if miners.len() == target {
                break;
            }
            if selected_addresses.insert(miner.address) {
                miners.push(miner.address);
            }
        }

        MinerAssignment { job_id, miners }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{ChainCommand, ChainEngine};
    use crate::types::{address, hash_bytes};

    fn register_validator(chain: &mut Chain, validator: Address) {
        let stake = chain.params().validator_min_stake;
        chain
            .apply_command(ChainCommand::RegisterValidator {
                address: validator,
                stake,
            })
            .unwrap();
    }

    fn register_miner(chain: &mut Chain, miner: Address) {
        let stake = chain.params().miner_min_stake;
        chain
            .apply_command(ChainCommand::RegisterMiner {
                address: miner,
                stake,
            })
            .unwrap();
    }

    #[test]
    fn scheduler_generates_deterministic_jobs() {
        let scheduler = JobScheduler::with_small_shape((4, 5, 6));
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let a = scheduler.generate_small_matmul(1, 2, &beacon, 10);
        let b = scheduler.generate_small_matmul(1, 2, &beacon, 10);
        assert_eq!(a.job_id, b.job_id);
        assert_eq!((a.m, a.k, a.n), (4, 5, 6));
    }

    #[test]
    fn synthetic_job_source_uses_chain_epoch_height_and_deadline() {
        let beacon = hash_bytes(b"test", &[b"synthetic-source"]);
        let mut chain = Chain::new(beacon);
        chain.set_position_for_testing(10, 7);
        chain.params.receipt_submission_window = 13;
        let mut source = SyntheticLocalJobSource::new(JobScheduler::with_small_shape((2, 3, 4)));

        let Some(JobState::TensorOp(job)) = source.next_job(&chain) else {
            panic!("synthetic local job source must emit TensorOp jobs");
        };

        assert_eq!(job.epoch, 7);
        assert_eq!((job.m, job.k, job.n), (2, 3, 4));
        assert_eq!(job.deadline_block, 23);
        assert_eq!(
            job.job_id,
            JobScheduler::with_small_shape((2, 3, 4))
                .generate_small_matmul(7, 10, &beacon, 23)
                .job_id
        );
    }

    #[test]
    fn synthetic_job_source_default_matches_local_cpu_profile_shape() {
        let beacon = hash_bytes(b"test", &[b"synthetic-default-source"]);
        let chain = Chain::new(beacon);
        let mut source = SyntheticLocalJobSource::default();

        let Some(JobState::TensorOp(job)) = source.next_job(&chain) else {
            panic!("default synthetic local job source must emit TensorOp jobs first");
        };

        assert_eq!((job.m, job.k, job.n), (8, 8, 8));
    }

    #[test]
    fn synthetic_job_source_emits_linear_training_steps_on_odd_heights() {
        let beacon = hash_bytes(b"test", &[b"synthetic-linear-source"]);
        let mut chain = Chain::new(beacon);
        chain.set_position_for_testing(11, 3);
        chain.params.receipt_submission_window = 13;
        let mut source = SyntheticLocalJobSource::new(JobScheduler::with_small_shape((2, 3, 4)));
        let weights = SyntheticLocalJobSource::linear_training_weights();

        let Some(JobState::LinearTrainingStep(job)) = source.next_job(&chain) else {
            panic!("synthetic local job source must emit LinearTrainingStep jobs on odd heights");
        };

        assert_eq!(job.step, 0);
        assert_eq!(job.weight_root_before, weights.commitment_root());
        assert_eq!(job.input_shape, vec![4, 3]);
        assert_eq!(job.weight_shape, vec![3, 2]);
        assert_eq!(job.target_shape, vec![4, 2]);
        assert_eq!(job.deadline_block, 24);
        assert_eq!(
            job.model_id,
            hash_bytes(
                b"tensor-vm-local-linear-model-v1",
                &[&beacon, &11u64.to_le_bytes()]
            )
        );
    }

    #[test]
    fn validator_assignment_is_deterministic_and_bounded() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        for i in 0..12 {
            register_validator(&mut chain, address(format!("validator-{i}").as_bytes()));
        }
        let scheduler = JobScheduler::default();
        let receipt = hash_bytes(b"test", &[b"receipt"]);
        let first = scheduler.assign_validators(&chain, receipt, &beacon);
        let second = scheduler.assign_validators(&chain, receipt, &beacon);
        assert_eq!(first, second);
        assert_eq!(
            first.validators.len(),
            chain.params.freivalds.validators_per_job
        );
        assert_eq!(first.receipt_id, receipt);
    }

    #[test]
    fn validator_assignment_is_bound_to_receipt_id() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        chain.params.freivalds.validators_per_job = 4;
        for i in 0..32 {
            register_validator(&mut chain, address(format!("validator-{i}").as_bytes()));
        }
        let scheduler = JobScheduler::default();
        let first =
            scheduler.assign_validators(&chain, hash_bytes(b"test", &[b"receipt-0"]), &beacon);
        let mut receipt_bound_assignment = None;
        for i in 1..100 {
            let receipt = hash_bytes(b"test", &[format!("receipt-{i}").as_bytes()]);
            let assignment = scheduler.assign_validators(&chain, receipt, &beacon);
            if assignment.validators != first.validators {
                receipt_bound_assignment = Some(assignment);
                break;
            }
        }

        assert!(
            receipt_bound_assignment.is_some(),
            "receipt id must affect validator assignment ordering"
        );
    }

    #[test]
    fn miner_assignment_uses_replication_factor() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        for i in 0..10 {
            register_miner(&mut chain, address(format!("miner-{i}").as_bytes()));
        }
        chain.params.replication_factor = 4;
        let scheduler = JobScheduler::default();
        let job = hash_bytes(b"test", &[b"job"]);
        let first = scheduler.assign_miners(&chain, job, &beacon);
        let second = scheduler.assign_miners(&chain, job, &beacon);
        assert_eq!(first, second);
        assert_eq!(first.miners.len(), 4);
    }

    #[test]
    fn miner_assignment_prefers_operator_separation() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let shared_operator = address(b"shared-operator");
        for i in 0..4 {
            chain
                .register_miner_with_operator(
                    address(format!("shared-miner-{i}").as_bytes()),
                    100,
                    shared_operator,
                )
                .unwrap();
        }
        for i in 0..4 {
            chain
                .register_miner_with_operator(
                    address(format!("distinct-miner-{i}").as_bytes()),
                    100,
                    address(format!("distinct-operator-{i}").as_bytes()),
                )
                .unwrap();
        }
        chain.params.replication_factor = 4;

        let scheduler = JobScheduler::default();
        let job = hash_bytes(b"test", &[b"operator-separated-job"]);
        let assignment = scheduler.assign_miners(&chain, job, &beacon);
        let operators: BTreeSet<_> = assignment
            .miners
            .iter()
            .map(|miner| chain.state().miners().get(miner).unwrap().operator_id)
            .collect();

        assert_eq!(assignment.miners.len(), 4);
        assert_eq!(operators.len(), 4);
    }

    #[test]
    fn miner_assignment_falls_back_when_operator_diversity_is_insufficient() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let shared_operator = address(b"only-operator");
        for i in 0..5 {
            chain
                .register_miner_with_operator(
                    address(format!("same-operator-miner-{i}").as_bytes()),
                    100,
                    shared_operator,
                )
                .unwrap();
        }
        chain.params.replication_factor = 3;

        let scheduler = JobScheduler::default();
        let job = hash_bytes(b"test", &[b"fallback-job"]);
        let assignment = scheduler.assign_miners(&chain, job, &beacon);
        let operators: BTreeSet<_> = assignment
            .miners
            .iter()
            .map(|miner| chain.state().miners().get(miner).unwrap().operator_id)
            .collect();

        assert_eq!(assignment.miners.len(), 3);
        assert_eq!(operators.len(), 1);
        assert_eq!(operators.into_iter().next(), Some(shared_operator));
    }
}
