use crate::error::Result;
use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepOutput, MatmulJob};
use crate::tensor::Tensor;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendKind {
    CpuReference,
    GpuMiner { device: String },
}

pub trait ExecutionBackend {
    fn kind(&self) -> BackendKind;

    fn execute_matmul(&self, job: &MatmulJob) -> Result<(Tensor, Tensor, Tensor)> {
        job.execute()
    }

    fn execute_linear_training_step(
        &self,
        job: &LinearTrainingStepJob,
        weights: &Tensor,
    ) -> Result<LinearTrainingStepOutput> {
        job.execute(weights)
    }
}

#[derive(Clone, Debug, Default)]
pub struct CpuReferenceBackend;

impl ExecutionBackend for CpuReferenceBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::CpuReference
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuMinerBackend {
    device: String,
}

impl GpuMinerBackend {
    pub fn new(device: impl Into<String>) -> Self {
        Self {
            device: device.into(),
        }
    }

    pub fn device(&self) -> &str {
        &self.device
    }
}

impl ExecutionBackend for GpuMinerBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::GpuMiner {
            device: self.device.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec};
    use crate::tensor::{DType, Tensor};
    use crate::types::hash_bytes;

    #[test]
    fn cpu_and_gpu_backends_match_canonical_matmul() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let job = MatmulJob::synthetic(0, 0, 8, 4, 5, &beacon, 10);
        let cpu = CpuReferenceBackend;
        let gpu = GpuMinerBackend::new("cuda:0");
        let (_, _, cpu_out) = cpu.execute_matmul(&job).unwrap();
        let (_, _, gpu_out) = gpu.execute_matmul(&job).unwrap();
        assert_eq!(cpu.kind(), BackendKind::CpuReference);
        assert_eq!(
            gpu.kind(),
            BackendKind::GpuMiner {
                device: "cuda:0".to_owned()
            }
        );
        assert_eq!(gpu.device(), "cuda:0");
        assert_eq!(cpu_out.commitment_root(), gpu_out.commitment_root());
    }

    #[test]
    fn cpu_and_gpu_backends_match_linear_step() {
        let weights =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"model"]),
            step: 0,
            batch_seed: hash_bytes(b"test", &[b"batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 2,
            deadline_block: 10,
        });
        let cpu = CpuReferenceBackend;
        let gpu = GpuMinerBackend::new("cuda:0");
        let cpu_out = cpu.execute_linear_training_step(&job, &weights).unwrap();
        let gpu_out = gpu.execute_linear_training_step(&job, &weights).unwrap();
        assert_eq!(
            cpu_out.weight_after.commitment_root(),
            gpu_out.weight_after.commitment_root()
        );
    }
}
