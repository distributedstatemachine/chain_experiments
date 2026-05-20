use crate::chain::{BlockVote, Chain, JobState, TensorBlock};
use crate::error::Result;
use crate::jobs::{LinearTrainingStepJob, MatmulJob};
use crate::miner::MinerNode;
use crate::profile::ChainProfile;
use crate::runtime::CpuReferenceBackend;
use crate::scheduler::{JobScheduler, JobSource, SyntheticLocalJobSource};
use crate::tensor::Tensor;
use crate::validator::{MatmulVerificationInput, ValidatorNode};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntheticCpuRoundResult {
    pub height: u64,
    pub tensors: Vec<Tensor>,
}

pub fn produce_synthetic_cpu_round(chain: &mut Chain) -> Result<Option<u64>> {
    Ok(produce_synthetic_cpu_round_with_tensors(chain)?.map(|round| round.height))
}

pub fn produce_synthetic_cpu_round_with_tensors(
    chain: &mut Chain,
) -> Result<Option<SyntheticCpuRoundResult>> {
    produce_synthetic_cpu_round_with_profile(chain, &ChainProfile::local_cpu())
}

pub fn produce_synthetic_cpu_round_with_profile(
    chain: &mut Chain,
    profile: &ChainProfile,
) -> Result<Option<SyntheticCpuRoundResult>> {
    let Some(mut job_source) = profile.synthetic_job_source() else {
        return Ok(None);
    };
    produce_synthetic_cpu_round_from_source(chain, &mut job_source)
}

fn produce_synthetic_cpu_round_from_source(
    chain: &mut Chain,
    job_source: &mut impl JobSource,
) -> Result<Option<SyntheticCpuRoundResult>> {
    if chain.state.miners.is_empty() || chain.state.validators.is_empty() {
        return Ok(None);
    }
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    match job_source.next_job(chain) {
        Some(JobState::TensorOp(job)) => produce_synthetic_matmul_round(chain, &scheduler, job),
        Some(JobState::LinearTrainingStep(job)) => {
            produce_synthetic_linear_training_round(chain, &scheduler, job)
        }
        None => Ok(None),
    }
}

fn produce_synthetic_matmul_round(
    chain: &mut Chain,
    scheduler: &JobScheduler,
    job: MatmulJob,
) -> Result<Option<SyntheticCpuRoundResult>> {
    let beacon = chain.state.finalized_randomness;
    chain.submit_job(JobState::TensorOp(job.clone()));
    let miner_assignment = scheduler.assign_miners(chain, job.job_id, &beacon);
    let mut receipts = Vec::new();
    for (index, miner_address) in miner_assignment.miners.iter().copied().enumerate() {
        let mut miner = MinerNode::new(miner_address, CpuReferenceBackend);
        let (receipt, a, b, c) =
            miner.solve_matmul_job(&job, chain.state.height, 1 + index as u64)?;
        chain.submit_tensor_op_receipt(receipt.clone())?;
        receipts.push((receipt, a, b, c));
    }
    for (receipt, a, b, c) in &receipts {
        let validation_seed = chain.validation_seed(&receipt.receipt_id);
        let validator_assignment = scheduler.assign_validators(chain, receipt.receipt_id, &beacon);
        for validator_address in validator_assignment.validators {
            let stake = chain
                .state
                .validators
                .get(&validator_address)
                .map(|validator| validator.stake)
                .unwrap_or_default();
            let validator = ValidatorNode::new(validator_address, stake);
            let attestation = validator.verify_matmul(MatmulVerificationInput {
                job: &job,
                receipt,
                a,
                b,
                c,
                validation_seed: &validation_seed,
                params: &chain.params.freivalds,
            })?;
            chain.submit_attestation(attestation)?;
        }
    }
    let Some((canonical_receipt, canonical_a, canonical_b, canonical_c)) = receipts.first() else {
        return Ok(None);
    };
    if !chain.has_attestation_quorum(&canonical_receipt.receipt_id)
        || !chain.has_redundant_agreement(&canonical_receipt.receipt_id)
    {
        return Ok(None);
    }
    chain.settle_epoch(1_000, 500);
    let proposer = chain.proposer_for_next_epoch(&beacon).unwrap_or_default();
    let timestamp = chain
        .blocks
        .last()
        .map(|block| {
            block
                .timestamp
                .saturating_add(chain.params.block_time_seconds)
        })
        .unwrap_or(0);
    let block = chain.produce_block(proposer, timestamp);
    finalize_local_cpu_block(chain, &block)?;
    Ok(Some(SyntheticCpuRoundResult {
        height: chain.state.height,
        tensors: vec![
            canonical_a.clone(),
            canonical_b.clone(),
            canonical_c.clone(),
        ],
    }))
}

fn produce_synthetic_linear_training_round(
    chain: &mut Chain,
    scheduler: &JobScheduler,
    job: LinearTrainingStepJob,
) -> Result<Option<SyntheticCpuRoundResult>> {
    let beacon = chain.state.finalized_randomness;
    let weights = SyntheticLocalJobSource::linear_training_weights();
    register_synthetic_linear_model(chain, &job, &weights);
    chain.submit_job(JobState::LinearTrainingStep(job.clone()));
    let miner_assignment = scheduler.assign_miners(chain, job.job_id, &beacon);
    let mut receipts = Vec::new();
    for (index, miner_address) in miner_assignment.miners.iter().copied().enumerate() {
        let mut miner = MinerNode::new(miner_address, CpuReferenceBackend);
        let (receipt, output) = miner.solve_linear_training_step(
            &job,
            &weights,
            chain.state.height,
            1 + index as u64,
        )?;
        chain.submit_linear_receipt(receipt.clone())?;
        receipts.push((receipt, output));
    }
    for (receipt, output) in &receipts {
        let validation_seed = chain.validation_seed(&receipt.receipt_id);
        let validator_assignment = scheduler.assign_validators(chain, receipt.receipt_id, &beacon);
        for validator_address in validator_assignment.validators {
            let stake = chain
                .state
                .validators
                .get(&validator_address)
                .map(|validator| validator.stake)
                .unwrap_or_default();
            let validator = ValidatorNode::new(validator_address, stake);
            let attestation = validator.verify_linear_training_step(
                &job,
                receipt,
                &weights,
                output,
                &validation_seed,
                &chain.params.freivalds,
            )?;
            chain.submit_attestation(attestation)?;
        }
    }
    let Some((canonical_receipt, canonical_output)) = receipts.first() else {
        return Ok(None);
    };
    if !chain.has_attestation_quorum(&canonical_receipt.receipt_id)
        || !chain.has_redundant_agreement(&canonical_receipt.receipt_id)
    {
        return Ok(None);
    }
    chain.settle_epoch(1_000, 500);
    chain.apply_model_transition(
        &job.model_id,
        job.step,
        &job.weight_root_before,
        canonical_receipt.weight_root_after,
    )?;
    let proposer = chain.proposer_for_next_epoch(&beacon).unwrap_or_default();
    let timestamp = chain
        .blocks
        .last()
        .map(|block| {
            block
                .timestamp
                .saturating_add(chain.params.block_time_seconds)
        })
        .unwrap_or(0);
    let block = chain.produce_block(proposer, timestamp);
    finalize_local_cpu_block(chain, &block)?;
    Ok(Some(SyntheticCpuRoundResult {
        height: chain.state.height,
        tensors: vec![
            canonical_output.x.clone(),
            canonical_output.target.clone(),
            canonical_output.y.clone(),
            canonical_output.dy.clone(),
            canonical_output.grad_w.clone(),
            canonical_output.weight_after.clone(),
        ],
    }))
}

fn register_synthetic_linear_model(
    chain: &mut Chain,
    job: &LinearTrainingStepJob,
    weights: &Tensor,
) {
    if !chain.state.model_states.contains_key(&job.model_id) {
        chain.register_model(
            job.model_id,
            SyntheticLocalJobSource::linear_training_architecture_hash(),
            weights.commitment_root(),
            SyntheticLocalJobSource::linear_training_config_hash(),
        );
    }
}

pub fn finalize_local_cpu_block(chain: &mut Chain, block: &TensorBlock) -> Result<()> {
    for validator_address in chain.state.validators.keys().copied().collect::<Vec<_>>() {
        let stake = chain
            .state
            .validators
            .get(&validator_address)
            .map(|validator| validator.stake)
            .unwrap_or_default();
        chain.submit_block_vote(BlockVote::new(validator_address, stake, block))?;
        if chain.is_block_finalized(&block.hash()) {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::ChainParams;
    use crate::types::{address, hash_bytes};
    use crate::verify::FreivaldsParams;

    #[test]
    fn synthetic_cpu_round_settles_work_and_advances_finalized_chain() {
        let params = ChainParams {
            replication_factor: 2,
            agreement_quorum: 2,
            freivalds: FreivaldsParams {
                validators_per_job: 2,
                minimum_validators: 2,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"localnet-round"]));
        for index in 0..2 {
            chain
                .register_miner(
                    address(format!("localnet-miner-{index}").as_bytes()),
                    chain.params.miner_min_stake,
                )
                .unwrap();
            chain
                .register_validator(
                    address(format!("localnet-validator-{index}").as_bytes()),
                    chain.params.validator_min_stake,
                )
                .unwrap();
        }

        let height = produce_synthetic_cpu_round(&mut chain).unwrap();
        let first_block = chain.blocks.last().unwrap().clone();

        assert_eq!(height, Some(1));
        assert_eq!(chain.state.height, 1);
        assert_eq!(chain.state.settled_receipts.len(), 2);
        assert!(chain.is_block_finalized(&first_block.hash()));

        let height = produce_synthetic_cpu_round(&mut chain).unwrap();
        let second_block = chain.blocks.last().unwrap().clone();

        assert_eq!(height, Some(2));
        assert_eq!(
            second_block.timestamp,
            first_block
                .timestamp
                .saturating_add(chain.params.block_time_seconds)
        );
        assert!(chain.is_block_finalized(&second_block.hash()));
        assert!(
            chain
                .state
                .jobs
                .values()
                .any(|job| matches!(job, JobState::LinearTrainingStep(_)))
        );
        assert_eq!(chain.state.model_states.len(), 1);
        assert_eq!(chain.state.model_states.values().next().unwrap().step, 1);

        let height = produce_synthetic_cpu_round(&mut chain).unwrap();
        let third_block = chain.blocks.last().unwrap();

        assert_eq!(height, Some(3));
        assert_eq!(
            third_block.timestamp,
            second_block
                .timestamp
                .saturating_add(chain.params.block_time_seconds)
        );
        assert!(chain.is_block_finalized(&third_block.hash()));
    }

    #[test]
    fn synthetic_cpu_round_waits_for_registered_roles() {
        let mut chain = Chain::new(hash_bytes(b"test", &[b"localnet-empty"]));
        assert_eq!(produce_synthetic_cpu_round(&mut chain).unwrap(), None);
    }

    #[test]
    fn synthetic_cpu_round_uses_profile_configured_jobs() {
        let mut profile = ChainProfile::local_cpu();
        profile.synthetic_job_scheduler = Some(JobScheduler::with_small_shape((2, 3, 4)));
        let params = ChainParams {
            replication_factor: 2,
            agreement_quorum: 2,
            freivalds: FreivaldsParams {
                validators_per_job: 2,
                minimum_validators: 2,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"profile-localnet"]));
        for index in 0..2 {
            chain
                .register_miner(
                    address(format!("profile-localnet-miner-{index}").as_bytes()),
                    chain.params.miner_min_stake,
                )
                .unwrap();
            chain
                .register_validator(
                    address(format!("profile-localnet-validator-{index}").as_bytes()),
                    chain.params.validator_min_stake,
                )
                .unwrap();
        }

        let round = produce_synthetic_cpu_round_with_profile(&mut chain, &profile)
            .unwrap()
            .expect("profile-enabled localnet should produce a round");

        assert_eq!(round.height, 1);
        assert!(chain.state.jobs.values().any(
            |job| matches!(job, JobState::TensorOp(job) if (job.m, job.k, job.n) == (2, 3, 4))
        ));
    }

    #[test]
    fn synthetic_cpu_round_waits_when_profile_disables_synthetic_jobs() {
        let mut chain = Chain::new(hash_bytes(b"test", &[b"profile-no-synthetic"]));
        chain
            .register_miner(
                address(b"profile-no-synthetic-miner"),
                chain.params.miner_min_stake,
            )
            .unwrap();
        chain
            .register_validator(
                address(b"profile-no-synthetic-validator"),
                chain.params.validator_min_stake,
            )
            .unwrap();

        assert_eq!(
            produce_synthetic_cpu_round_with_profile(&mut chain, &ChainProfile::public_testnet())
                .unwrap(),
            None
        );
        assert!(chain.blocks.is_empty());
        assert!(chain.state.jobs.is_empty());
    }

    #[test]
    fn synthetic_cpu_round_waits_for_job_source() {
        struct EmptyJobSource;

        impl JobSource for EmptyJobSource {
            fn next_job(&mut self, _chain: &Chain) -> Option<JobState> {
                None
            }
        }

        let mut chain = Chain::new(hash_bytes(b"test", &[b"localnet-no-job"]));
        chain
            .register_miner(
                address(b"localnet-no-job-miner"),
                chain.params.miner_min_stake,
            )
            .unwrap();
        chain
            .register_validator(
                address(b"localnet-no-job-validator"),
                chain.params.validator_min_stake,
            )
            .unwrap();
        let mut job_source = EmptyJobSource;

        assert_eq!(
            produce_synthetic_cpu_round_from_source(&mut chain, &mut job_source).unwrap(),
            None
        );
        assert!(chain.blocks.is_empty());
    }

    #[test]
    fn synthetic_cpu_round_waits_for_miner_assignment() {
        let params = ChainParams {
            replication_factor: 0,
            freivalds: FreivaldsParams {
                validators_per_job: 1,
                minimum_validators: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"localnet-no-miners"]));
        chain
            .register_miner(
                address(b"localnet-assignment-miner"),
                chain.params.miner_min_stake,
            )
            .unwrap();
        chain
            .register_validator(
                address(b"localnet-assignment-validator"),
                chain.params.validator_min_stake,
            )
            .unwrap();

        assert_eq!(produce_synthetic_cpu_round(&mut chain).unwrap(), None);
        assert!(chain.blocks.is_empty());
    }

    #[test]
    fn synthetic_cpu_round_waits_for_redundant_agreement() {
        let params = ChainParams {
            replication_factor: 1,
            agreement_quorum: 2,
            freivalds: FreivaldsParams {
                validators_per_job: 1,
                minimum_validators: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain =
            Chain::with_params(params, hash_bytes(b"test", &[b"localnet-no-agreement"]));
        chain
            .register_miner(
                address(b"localnet-agreement-miner"),
                chain.params.miner_min_stake,
            )
            .unwrap();
        chain
            .register_validator(
                address(b"localnet-agreement-validator"),
                chain.params.validator_min_stake,
            )
            .unwrap();

        assert_eq!(produce_synthetic_cpu_round(&mut chain).unwrap(), None);
        assert!(chain.blocks.is_empty());
        assert!(chain.state.settled_receipts.is_empty());
    }

    #[test]
    fn synthetic_linear_round_waits_for_miner_assignment() {
        let params = ChainParams {
            replication_factor: 0,
            freivalds: FreivaldsParams {
                validators_per_job: 1,
                minimum_validators: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain =
            Chain::with_params(params, hash_bytes(b"test", &[b"localnet-linear-no-miners"]));
        chain.state.height = 1;
        chain
            .register_miner(
                address(b"localnet-linear-assignment-miner"),
                chain.params.miner_min_stake,
            )
            .unwrap();
        chain
            .register_validator(
                address(b"localnet-linear-assignment-validator"),
                chain.params.validator_min_stake,
            )
            .unwrap();

        assert_eq!(produce_synthetic_cpu_round(&mut chain).unwrap(), None);
        assert!(chain.blocks.is_empty());
        assert_eq!(chain.state.model_states.len(), 1);
        assert_eq!(chain.state.model_states.values().next().unwrap().step, 0);
    }

    #[test]
    fn synthetic_linear_round_waits_for_redundant_agreement() {
        let params = ChainParams {
            replication_factor: 1,
            agreement_quorum: 2,
            freivalds: FreivaldsParams {
                validators_per_job: 1,
                minimum_validators: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(
            params,
            hash_bytes(b"test", &[b"localnet-linear-no-agreement"]),
        );
        chain.state.height = 1;
        chain
            .register_miner(
                address(b"localnet-linear-agreement-miner"),
                chain.params.miner_min_stake,
            )
            .unwrap();
        chain
            .register_validator(
                address(b"localnet-linear-agreement-validator"),
                chain.params.validator_min_stake,
            )
            .unwrap();

        assert_eq!(produce_synthetic_cpu_round(&mut chain).unwrap(), None);
        assert!(chain.blocks.is_empty());
        assert!(chain.state.settled_receipts.is_empty());
        assert_eq!(chain.state.model_states.values().next().unwrap().step, 0);
    }
}
