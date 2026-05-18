use crate::chain::LocalChain;
use crate::jobs::MatmulJob;
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
        chain: &LocalChain,
        receipt_id: Hash,
        seed: &Hash,
    ) -> ValidatorAssignment {
        let mut validators: Vec<_> = chain.state.validators.keys().copied().collect();
        validators.sort_by_key(|validator| {
            let draw = hash_bytes(b"tensor-vm-validator-assignment-v1", &[seed, validator]);
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

    pub fn assign_miners(&self, chain: &LocalChain, job_id: Hash, seed: &Hash) -> MinerAssignment {
        let mut candidates: Vec<_> = chain.state.miners.values().collect();
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
    use crate::types::{address, hash_bytes};

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
    fn validator_assignment_is_deterministic_and_bounded() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        for i in 0..12 {
            chain
                .register_validator(address(format!("validator-{i}").as_bytes()), 10_000)
                .unwrap();
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
    }

    #[test]
    fn miner_assignment_uses_replication_factor() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
        for i in 0..10 {
            chain
                .register_miner(address(format!("miner-{i}").as_bytes()), 100)
                .unwrap();
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
        let mut chain = LocalChain::new(beacon);
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
            .map(|miner| chain.state.miners.get(miner).unwrap().operator_id)
            .collect();

        assert_eq!(assignment.miners.len(), 4);
        assert_eq!(operators.len(), 4);
    }

    #[test]
    fn miner_assignment_falls_back_when_operator_diversity_is_insufficient() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = LocalChain::new(beacon);
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
            .map(|miner| chain.state.miners.get(miner).unwrap().operator_id)
            .collect();

        assert_eq!(assignment.miners.len(), 3);
        assert_eq!(operators.len(), 1);
        assert_eq!(operators.into_iter().next(), Some(shared_operator));
    }
}
