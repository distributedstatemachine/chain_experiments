use crate::error::Result;
use crate::jobs::{
    LinearTrainingStepJob, LinearTrainingStepOutput, LinearTrainingStepReceipt, MatmulJob,
    PrimitiveType, TensorOpReceipt,
};
use crate::tensor::Tensor;
use crate::tensor_server::TensorServer;
use crate::types::{Address, Hash};
use crate::verify::{
    AttestationStatement, FreivaldsParams, ValidatorAttestation, VerificationResult,
    verify_linear_training_step, verify_tensor_op,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorNode {
    pub address: Address,
    pub stake: u64,
}

pub struct MatmulVerificationInput<'a> {
    pub job: &'a MatmulJob,
    pub receipt: &'a TensorOpReceipt,
    pub a: &'a Tensor,
    pub b: &'a Tensor,
    pub c: &'a Tensor,
    pub validation_seed: &'a Hash,
    pub params: &'a FreivaldsParams,
}

impl ValidatorNode {
    pub fn new(address: Address, stake: u64) -> Self {
        Self { address, stake }
    }

    pub fn verify_matmul(
        &self,
        input: MatmulVerificationInput<'_>,
    ) -> Result<ValidatorAttestation> {
        let report = verify_tensor_op(
            input.job,
            input.receipt,
            input.a,
            input.b,
            input.c,
            input.validation_seed,
            input.params,
        )?;
        Ok(ValidatorAttestation::new(
            self.address,
            self.stake,
            AttestationStatement {
                receipt_id: input.receipt.receipt_id,
                job_id: input.receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: report.result,
                checks_root: report.checks_root,
                data_availability_passed: report.data_availability_passed,
            },
        ))
    }

    pub fn verify_matmul_from_server(
        &self,
        job: &MatmulJob,
        receipt: &TensorOpReceipt,
        server: &TensorServer,
        validation_seed: &Hash,
        params: &FreivaldsParams,
    ) -> Result<ValidatorAttestation> {
        let unavailable = || {
            ValidatorAttestation::new(
                self.address,
                self.stake,
                AttestationStatement {
                    receipt_id: receipt.receipt_id,
                    job_id: receipt.job_id,
                    primitive_type: PrimitiveType::TensorOp,
                    result: VerificationResult::Unavailable,
                    checks_root: crate::types::hash_bytes(
                        b"tensor-vm-unavailable-checks-v1",
                        &[&receipt.receipt_id],
                    ),
                    data_availability_passed: false,
                },
            )
        };
        let Some(a_root) = receipt.input_roots.first() else {
            return Ok(unavailable());
        };
        let Some(b_root) = receipt.input_roots.get(1) else {
            return Ok(unavailable());
        };
        let Some(c_root) = receipt.output_roots.first() else {
            return Ok(unavailable());
        };
        let Some(a) = server.get_by_commitment_root(a_root) else {
            return Ok(unavailable());
        };
        let Some(b) = server.get_by_commitment_root(b_root) else {
            return Ok(unavailable());
        };
        let Some(c) = server.get_by_commitment_root(c_root) else {
            return Ok(unavailable());
        };
        self.verify_matmul(MatmulVerificationInput {
            job,
            receipt,
            a,
            b,
            c,
            validation_seed,
            params,
        })
    }

    pub fn verify_linear_training_step(
        &self,
        job: &LinearTrainingStepJob,
        receipt: &LinearTrainingStepReceipt,
        weights_before: &Tensor,
        output: &LinearTrainingStepOutput,
        validation_seed: &Hash,
        params: &FreivaldsParams,
    ) -> Result<ValidatorAttestation> {
        let report = verify_linear_training_step(
            job,
            receipt,
            weights_before,
            output,
            validation_seed,
            params,
        )?;
        Ok(ValidatorAttestation::new(
            self.address,
            self.stake,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::LinearTrainingStep,
                result: report.result,
                checks_root: report.checks_root,
                data_availability_passed: report.data_availability_passed,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::MatmulJob;
    use crate::miner::MinerNode;
    use crate::runtime::CpuReferenceBackend;
    use crate::tensor_server::TensorServer;
    use crate::types::{address, hash_bytes};
    use crate::verify::VerificationResult;

    #[test]
    fn validator_attests_valid_matmul_receipt() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let mut miner = MinerNode::new(address(b"miner"), CpuReferenceBackend);
        let (receipt, a, b, c) = miner.solve_matmul_job(&job, 1, 5).unwrap();
        let validator = ValidatorNode::new(address(b"validator"), 10_000);
        let attestation = validator
            .verify_matmul(MatmulVerificationInput {
                job: &job,
                receipt: &receipt,
                a: &a,
                b: &b,
                c: &c,
                validation_seed: &hash_bytes(b"test", &[b"validation"]),
                params: &FreivaldsParams::default(),
            })
            .unwrap();
        assert_eq!(attestation.result, VerificationResult::Valid);
        assert!(attestation.verify_signature());
    }

    #[test]
    fn validator_attests_unavailable_when_server_lacks_tensor_roots() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let mut miner = MinerNode::new(address(b"miner"), CpuReferenceBackend);
        let (receipt, a, b, _c) = miner.solve_matmul_job(&job, 1, 5).unwrap();
        let mut server = TensorServer::default();
        server.insert(a);
        server.insert(b);

        let validator = ValidatorNode::new(address(b"validator"), 10_000);
        let attestation = validator
            .verify_matmul_from_server(
                &job,
                &receipt,
                &server,
                &hash_bytes(b"test", &[b"validation"]),
                &FreivaldsParams::default(),
            )
            .unwrap();

        assert_eq!(attestation.result, VerificationResult::Unavailable);
        assert!(!attestation.data_availability_passed);
        assert!(attestation.verify_signature());
    }

    #[test]
    fn validator_attests_unavailable_for_each_missing_receipt_root() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let mut miner = MinerNode::new(address(b"miner"), CpuReferenceBackend);
        let (receipt, a, _b, _c) = miner.solve_matmul_job(&job, 1, 5).unwrap();
        let validator = ValidatorNode::new(address(b"validator"), 10_000);
        let seed = hash_bytes(b"test", &[b"validation"]);
        let params = FreivaldsParams::default();

        for mutate in [
            |receipt: &mut TensorOpReceipt| receipt.input_roots.clear(),
            |receipt: &mut TensorOpReceipt| receipt.input_roots.truncate(1),
            |receipt: &mut TensorOpReceipt| receipt.output_roots.clear(),
        ] {
            let mut mutated = receipt.clone();
            mutate(&mut mutated);
            let attestation = validator
                .verify_matmul_from_server(&job, &mutated, &miner.tensor_server, &seed, &params)
                .unwrap();
            assert_eq!(attestation.result, VerificationResult::Unavailable);
            assert!(!attestation.data_availability_passed);
        }

        let empty_server = TensorServer::default();
        let missing_a = validator
            .verify_matmul_from_server(&job, &receipt, &empty_server, &seed, &params)
            .unwrap();
        assert_eq!(missing_a.result, VerificationResult::Unavailable);

        let mut only_a_server = TensorServer::default();
        only_a_server.insert(a);
        let missing_b = validator
            .verify_matmul_from_server(&job, &receipt, &only_a_server, &seed, &params)
            .unwrap();
        assert_eq!(missing_b.result, VerificationResult::Unavailable);
    }

    #[test]
    fn validator_verifies_matmul_from_tensor_server() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let mut miner = MinerNode::new(address(b"miner"), CpuReferenceBackend);
        let (receipt, _a, _b, _c) = miner.solve_matmul_job(&job, 1, 5).unwrap();

        let validator = ValidatorNode::new(address(b"validator"), 10_000);
        let attestation = validator
            .verify_matmul_from_server(
                &job,
                &receipt,
                &miner.tensor_server,
                &hash_bytes(b"test", &[b"validation"]),
                &FreivaldsParams::default(),
            )
            .unwrap();

        assert_eq!(attestation.result, VerificationResult::Valid);
        assert!(attestation.data_availability_passed);
    }
}
