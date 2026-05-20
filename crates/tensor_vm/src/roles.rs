use crate::chain::{Chain, JobState, ReceiptState};
use crate::error::{Result, TvmError};
use crate::jobs::{LinearTrainingStepOutput, PrimitiveType};
use crate::miner::MinerNode;
use crate::runtime::CpuReferenceBackend;
use crate::scheduler::SyntheticLocalJobSource;
use crate::tensor::Tensor;
use crate::types::{Address, Hash};
use crate::validator::{MatmulVerificationInput, ValidatorNode};
use crate::verify::{FreivaldsParams, ValidatorAttestation};

#[derive(Clone, Debug)]
pub enum RoleReceiptArtifacts {
    TensorOp {
        a: Tensor,
        b: Tensor,
        c: Tensor,
    },
    LinearTrainingStep {
        weights_before: Tensor,
        output: Box<LinearTrainingStepOutput>,
    },
}

#[derive(Clone, Debug)]
pub struct RoleReceiptBundle {
    pub receipt: ReceiptState,
    pub artifacts: RoleReceiptArtifacts,
}

impl RoleReceiptBundle {
    pub fn receipt_id(&self) -> Hash {
        self.receipt.receipt_id()
    }

    pub fn served_tensors(&self) -> Vec<Tensor> {
        match &self.artifacts {
            RoleReceiptArtifacts::TensorOp { a, b, c } => vec![a.clone(), b.clone(), c.clone()],
            RoleReceiptArtifacts::LinearTrainingStep { output, .. } => vec![
                output.x.clone(),
                output.target.clone(),
                output.y.clone(),
                output.dy.clone(),
                output.grad_w.clone(),
                output.weight_after.clone(),
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuReferenceMinerRole {
    pub address: Address,
}

impl CpuReferenceMinerRole {
    pub fn new(address: Address) -> Self {
        Self { address }
    }

    pub fn execute_job(
        &self,
        job: &JobState,
        submitted_at_block: u64,
        execution_time_ms: u64,
    ) -> Result<RoleReceiptBundle> {
        let mut miner = MinerNode::new(self.address, CpuReferenceBackend);
        match job {
            JobState::TensorOp(job) => {
                let (receipt, a, b, c) =
                    miner.solve_matmul_job(job, submitted_at_block, execution_time_ms)?;
                Ok(RoleReceiptBundle {
                    receipt: ReceiptState::TensorOp(receipt),
                    artifacts: RoleReceiptArtifacts::TensorOp { a, b, c },
                })
            }
            JobState::LinearTrainingStep(job) => {
                let weights_before = SyntheticLocalJobSource::linear_training_weights();
                let (receipt, output) = miner.solve_linear_training_step(
                    job,
                    &weights_before,
                    submitted_at_block,
                    execution_time_ms,
                )?;
                Ok(RoleReceiptBundle {
                    receipt: ReceiptState::LinearTrainingStep(receipt),
                    artifacts: RoleReceiptArtifacts::LinearTrainingStep {
                        weights_before,
                        output: Box::new(output),
                    },
                })
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReferenceValidatorRole {
    pub address: Address,
    pub stake: u64,
}

impl ReferenceValidatorRole {
    pub fn new(address: Address, stake: u64) -> Self {
        Self { address, stake }
    }

    pub fn verify_receipt(
        &self,
        job: &JobState,
        bundle: &RoleReceiptBundle,
        validation_seed: &Hash,
        params: &FreivaldsParams,
    ) -> Result<ValidatorAttestation> {
        let validator = ValidatorNode::new(self.address, self.stake);
        match (job, &bundle.receipt, &bundle.artifacts) {
            (
                JobState::TensorOp(job),
                ReceiptState::TensorOp(receipt),
                RoleReceiptArtifacts::TensorOp { a, b, c },
            ) => validator.verify_matmul(MatmulVerificationInput {
                job,
                receipt,
                a,
                b,
                c,
                validation_seed,
                params,
            }),
            (
                JobState::LinearTrainingStep(job),
                ReceiptState::LinearTrainingStep(receipt),
                RoleReceiptArtifacts::LinearTrainingStep {
                    weights_before,
                    output,
                },
            ) => validator.verify_linear_training_step(
                job,
                receipt,
                weights_before,
                output.as_ref(),
                validation_seed,
                params,
            ),
            _ => Err(TvmError::InvalidReceipt(
                "job and receipt primitive mismatch",
            )),
        }
    }
}

pub fn validator_stake(chain: &Chain, validator: &Address) -> u64 {
    chain
        .state
        .validators
        .get(validator)
        .map(|validator| validator.stake)
        .unwrap_or_default()
}

pub fn primitive_type(job: &JobState) -> PrimitiveType {
    match job {
        JobState::TensorOp(_) => PrimitiveType::TensorOp,
        JobState::LinearTrainingStep(_) => PrimitiveType::LinearTrainingStep,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::ChainParams;
    use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec, MatmulJob};
    use crate::tensor::DType;
    use crate::types::{address, hash_bytes};
    use crate::verify::VerificationResult;

    #[test]
    fn cpu_reference_miner_role_executes_tensor_op_jobs() {
        let beacon = hash_bytes(b"test", &[b"role-miner-matmul"]);
        let job = JobState::TensorOp(MatmulJob::synthetic(0, 0, 2, 3, 4, &beacon, 10));
        let miner = CpuReferenceMinerRole::new(address(b"role-miner"));

        let bundle = miner.execute_job(&job, 7, 11).unwrap();

        assert_eq!(primitive_type(&job), PrimitiveType::TensorOp);
        assert_eq!(bundle.served_tensors().len(), 3);
        assert!(matches!(bundle.receipt, ReceiptState::TensorOp(_)));
    }

    #[test]
    fn cpu_reference_miner_role_executes_linear_training_jobs() {
        let weights = SyntheticLocalJobSource::linear_training_weights();
        let job = JobState::LinearTrainingStep(LinearTrainingStepJob::from_spec(
            LinearTrainingStepSpec {
                model_id: hash_bytes(b"test", &[b"role-linear-model"]),
                step: 0,
                batch_seed: hash_bytes(b"test", &[b"role-linear-batch"]),
                weight_root_before: weights.commitment_root(),
                input_shape: vec![4, 3],
                weight_shape: vec![3, 2],
                target_shape: vec![4, 2],
                lr: 2,
                deadline_block: 10,
            },
        ));
        let miner = CpuReferenceMinerRole::new(address(b"role-linear-miner"));

        let bundle = miner.execute_job(&job, 3, 5).unwrap();

        assert_eq!(primitive_type(&job), PrimitiveType::LinearTrainingStep);
        assert_eq!(bundle.served_tensors().len(), 6);
        assert!(matches!(
            bundle.receipt,
            ReceiptState::LinearTrainingStep(_)
        ));
    }

    #[test]
    fn reference_validator_role_attests_matching_receipt_artifacts() {
        let params = ChainParams::default();
        let beacon = hash_bytes(b"test", &[b"role-validator"]);
        let job = JobState::TensorOp(MatmulJob::synthetic(0, 0, 2, 3, 4, &beacon, 10));
        let miner = CpuReferenceMinerRole::new(address(b"role-validator-miner"));
        let bundle = miner.execute_job(&job, 0, 1).unwrap();
        let validator = ReferenceValidatorRole::new(address(b"role-validator"), 10_000);

        let attestation = validator
            .verify_receipt(
                &job,
                &bundle,
                &hash_bytes(b"test", &[b"role-validator-seed"]),
                &params.freivalds,
            )
            .unwrap();

        assert_eq!(attestation.result, VerificationResult::Valid);
        assert_eq!(attestation.receipt_id, bundle.receipt_id());
        assert!(attestation.verify_signature());
    }

    #[test]
    fn reference_validator_role_rejects_mismatched_artifacts() {
        let params = ChainParams::default();
        let beacon = hash_bytes(b"test", &[b"role-validator-mismatch"]);
        let matmul_job = JobState::TensorOp(MatmulJob::synthetic(0, 0, 2, 3, 4, &beacon, 10));
        let linear_job = JobState::LinearTrainingStep(LinearTrainingStepJob::from_spec(
            LinearTrainingStepSpec {
                model_id: hash_bytes(b"test", &[b"mismatch-model"]),
                step: 0,
                batch_seed: hash_bytes(b"test", &[b"mismatch-batch"]),
                weight_root_before: SyntheticLocalJobSource::linear_training_weights()
                    .commitment_root(),
                input_shape: vec![4, 3],
                weight_shape: vec![3, 2],
                target_shape: vec![4, 2],
                lr: 2,
                deadline_block: 10,
            },
        ));
        let miner = CpuReferenceMinerRole::new(address(b"role-mismatch-miner"));
        let bundle = miner.execute_job(&matmul_job, 0, 1).unwrap();
        let validator = ReferenceValidatorRole::new(address(b"role-mismatch-validator"), 10_000);

        let error = validator
            .verify_receipt(
                &linear_job,
                &bundle,
                &hash_bytes(b"test", &[b"role-mismatch-seed"]),
                &params.freivalds,
            )
            .unwrap_err();

        assert_eq!(
            error,
            TvmError::InvalidReceipt("job and receipt primitive mismatch")
        );
    }

    #[test]
    fn validator_stake_defaults_to_zero_for_unknown_validator() {
        let chain = Chain::new(hash_bytes(b"test", &[b"role-stake"]));

        assert_eq!(validator_stake(&chain, &address(b"missing-validator")), 0);
    }

    #[test]
    fn linear_training_role_artifacts_expose_weight_before() {
        let weights =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let job = JobState::LinearTrainingStep(LinearTrainingStepJob::from_spec(
            LinearTrainingStepSpec {
                model_id: hash_bytes(b"test", &[b"role-artifact-model"]),
                step: 0,
                batch_seed: hash_bytes(b"test", &[b"role-artifact-batch"]),
                weight_root_before: weights.commitment_root(),
                input_shape: vec![4, 3],
                weight_shape: vec![3, 2],
                target_shape: vec![4, 2],
                lr: 2,
                deadline_block: 10,
            },
        ));
        let bundle = CpuReferenceMinerRole::new(address(b"role-artifact-miner"))
            .execute_job(&job, 0, 1)
            .unwrap();

        let RoleReceiptArtifacts::LinearTrainingStep { weights_before, .. } = bundle.artifacts
        else {
            panic!("linear job must return linear artifacts");
        };
        assert_eq!(weights_before.commitment_root(), weights.commitment_root());
    }
}
