use crate::error::Result;
#[cfg(feature = "cuda-kernels")]
use crate::error::TvmError;
use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepOutput, MatmulJob};
use crate::tensor::Tensor;
#[cfg(feature = "cuda-kernels")]
use crate::vm;

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

    fn execute_matmul(&self, job: &MatmulJob) -> Result<(Tensor, Tensor, Tensor)> {
        let (a, b) = job.input_tensors()?;
        #[cfg(feature = "cuda-kernels")]
        let c = cuda::field_matmul(self.cuda_device_index()?, &a, &b)?;
        #[cfg(not(feature = "cuda-kernels"))]
        let c = a.matmul(&b)?;
        Ok((a, b, c))
    }

    fn execute_linear_training_step(
        &self,
        job: &LinearTrainingStepJob,
        weights: &Tensor,
    ) -> Result<LinearTrainingStepOutput> {
        #[cfg(feature = "cuda-kernels")]
        {
            if weights.commitment_root() != job.weight_root_before {
                return Err(TvmError::InvalidReceipt("weight root mismatch"));
            }
            let (x, target) = job.batch_tensors()?;
            let device_index = self.cuda_device_index()?;
            let y = cuda::field_matmul(device_index, &x, weights)?;
            let dy = y.sub(&target)?;
            let grad_w = cuda::field_matmul(device_index, &x.transpose()?, &dy)?;
            let weight_after = weights.sub(&grad_w.scalar_mul(job.lr)?)?;
            let loss_commitment = vm::mse_loss(&y, &target)?;
            Ok(LinearTrainingStepOutput {
                x,
                target,
                y,
                dy,
                grad_w,
                weight_after,
                loss_commitment,
            })
        }
        #[cfg(not(feature = "cuda-kernels"))]
        {
            job.execute(weights)
        }
    }
}

#[cfg(feature = "cuda-kernels")]
impl GpuMinerBackend {
    fn cuda_device_index(&self) -> Result<u32> {
        let index = self.device.strip_prefix("cuda:").unwrap_or(&self.device);
        index
            .parse::<u32>()
            .map_err(|_| TvmError::InvalidReceipt("invalid cuda device"))
    }
}

pub fn cuda_kernels_compiled() -> bool {
    cfg!(feature = "cuda-kernels")
}

pub fn cuda_device_count() -> Result<u32> {
    #[cfg(feature = "cuda-kernels")]
    {
        cuda::device_count()
    }
    #[cfg(not(feature = "cuda-kernels"))]
    {
        Ok(0)
    }
}

#[cfg(feature = "cuda-kernels")]
mod cuda {
    use super::*;

    unsafe extern "C" {
        fn tensor_vm_cuda_device_count(out: *mut u32) -> i32;
        fn tensor_vm_cuda_field_matmul(
            device_index: u32,
            lhs: *const u64,
            rhs: *const u64,
            out: *mut u64,
            rows: u64,
            inner: u64,
            cols: u64,
        ) -> i32;
    }

    pub fn device_count() -> Result<u32> {
        let mut count = 0;
        let code = unsafe { tensor_vm_cuda_device_count(&mut count) };
        if code == 0 {
            Ok(count)
        } else {
            Err(cuda_error(code))
        }
    }

    pub fn field_matmul(device_index: u32, lhs: &Tensor, rhs: &Tensor) -> Result<Tensor> {
        let rows = lhs.rows()?;
        let inner = lhs.cols()?;
        rhs.require_rank_for_cuda_matmul()?;
        if inner != rhs.shape()[0] {
            return Err(TvmError::DimensionMismatch {
                left: lhs.shape().to_vec(),
                right: rhs.shape().to_vec(),
            });
        }
        let cols = rhs.shape()[1];
        let mut out = vec![0; rows * cols];
        let code = unsafe {
            tensor_vm_cuda_field_matmul(
                device_index,
                lhs.as_slice().as_ptr(),
                rhs.as_slice().as_ptr(),
                out.as_mut_ptr(),
                rows as u64,
                inner as u64,
                cols as u64,
            )
        };
        if code != 0 {
            return Err(cuda_error(code));
        }
        Tensor::from_vec(vec![rows, cols], lhs.dtype(), out)
    }

    trait CudaMatmulShape {
        fn require_rank_for_cuda_matmul(&self) -> Result<()>;
    }

    impl CudaMatmulShape for Tensor {
        fn require_rank_for_cuda_matmul(&self) -> Result<()> {
            if self.shape().len() == 2 {
                Ok(())
            } else {
                Err(TvmError::UnsupportedRank {
                    rank: self.shape().len(),
                })
            }
        }
    }

    fn cuda_error(code: i32) -> TvmError {
        match code {
            -1 => TvmError::InvalidReceipt("cuda kernel received null pointer"),
            -2 => TvmError::InvalidReceipt("cuda device unavailable or invalid shape"),
            -3 => TvmError::InvalidReceipt("cuda allocation failed"),
            -4 => TvmError::InvalidReceipt("cuda host-device copy failed"),
            -5 => TvmError::InvalidReceipt("cuda kernel execution failed"),
            -6 => TvmError::InvalidReceipt("cuda device index out of range"),
            _ => TvmError::InvalidReceipt("cuda kernel failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "cuda-kernels")]
    use crate::field::MODULUS;
    use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec};
    use crate::tensor::{DType, Tensor};
    use crate::types::hash_bytes;

    #[test]
    fn cuda_kernel_feature_flag_reports_availability() {
        assert_eq!(cuda_kernels_compiled(), cfg!(feature = "cuda-kernels"));
        #[cfg(not(feature = "cuda-kernels"))]
        assert_eq!(cuda_device_count().unwrap(), 0);
        #[cfg(feature = "cuda-kernels")]
        assert!(cuda_device_count().unwrap() > 0);
    }

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

    #[cfg(feature = "cuda-kernels")]
    #[test]
    fn cuda_kernel_matches_canonical_field_matmul_edges() {
        assert!(cuda_kernels_compiled());
        assert!(cuda_device_count().unwrap() > 0);
        let lhs = Tensor::from_vec(
            vec![2, 3],
            DType::FieldElement,
            vec![MODULUS - 1, 2, 3, 4, MODULUS - 2, 6],
        )
        .unwrap();
        let rhs = Tensor::from_vec(
            vec![3, 2],
            DType::FieldElement,
            vec![7, 8, MODULUS - 3, 10, 11, MODULUS - 4],
        )
        .unwrap();
        let expected = lhs.matmul(&rhs).unwrap();
        let actual = cuda::field_matmul(0, &lhs, &rhs).unwrap();
        assert_eq!(actual.as_slice(), expected.as_slice());
        assert_eq!(actual.commitment_root(), expected.commitment_root());
    }
}
