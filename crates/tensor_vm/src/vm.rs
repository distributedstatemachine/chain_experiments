use crate::error::Result;
use crate::field::Elem;
use crate::tensor::{DType, Tensor};
use crate::types::{Hash, hash_bytes};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TensorOp {
    RandomTensor {
        seed: Hash,
        shape: Vec<usize>,
        dtype: DType,
    },
    Matmul,
    Transpose,
    Add,
    Sub,
    Mul,
    ReduceSum {
        axis: usize,
    },
    ScalarMul {
        scalar: Elem,
    },
    CommitTensor,
    HashTensor,
    MseLoss,
    LinearBackward,
    SgdUpdate {
        lr: Elem,
    },
}

pub fn program_hash(ops: &[TensorOp]) -> Hash {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&(ops.len() as u64).to_le_bytes());
    for op in ops {
        encode_op(op, &mut encoded);
    }
    hash_bytes(b"tensor-vm-program-v1", &[&encoded])
}

pub fn random_tensor(seed: &Hash, shape: Vec<usize>, dtype: DType) -> Result<Tensor> {
    Tensor::random(seed, shape, dtype)
}

pub fn matmul(a: &Tensor, b: &Tensor) -> Result<Tensor> {
    a.matmul(b)
}

pub fn transpose(a: &Tensor) -> Result<Tensor> {
    a.transpose()
}

pub fn add(a: &Tensor, b: &Tensor) -> Result<Tensor> {
    a.add(b)
}

pub fn sub(a: &Tensor, b: &Tensor) -> Result<Tensor> {
    a.sub(b)
}

pub fn mul(a: &Tensor, b: &Tensor) -> Result<Tensor> {
    a.mul(b)
}

pub fn reduce_sum(a: &Tensor, axis: usize) -> Result<Tensor> {
    a.reduce_sum(axis)
}

pub fn scalar_mul(a: &Tensor, scalar: Elem) -> Result<Tensor> {
    a.scalar_mul(scalar)
}

pub fn commit_tensor(a: &Tensor) -> Hash {
    a.commitment_root()
}

pub fn hash_tensor(a: &Tensor) -> Hash {
    a.hash_tensor()
}

pub fn mse_loss(y: &Tensor, target: &Tensor) -> Result<Hash> {
    let sum = y.squared_error_sum(target)?;
    Ok(hash_bytes(
        b"tensor-vm-mse-loss-v1",
        &[&sum.to_le_bytes(), &(y.len() as u64).to_le_bytes()],
    ))
}

pub fn linear_backward(x: &Tensor, dy: &Tensor) -> Result<Tensor> {
    x.transpose()?.matmul(dy)
}

pub fn sgd_update(w: &Tensor, grad: &Tensor, lr: Elem) -> Result<Tensor> {
    w.sub(&grad.scalar_mul(lr)?)
}

fn encode_op(op: &TensorOp, out: &mut Vec<u8>) {
    match op {
        TensorOp::RandomTensor { seed, shape, dtype } => {
            out.push(1);
            out.extend_from_slice(seed);
            out.push(dtype.tag());
            out.extend_from_slice(&(shape.len() as u64).to_le_bytes());
            for dim in shape {
                out.extend_from_slice(&(*dim as u64).to_le_bytes());
            }
        }
        TensorOp::Matmul => out.push(2),
        TensorOp::Transpose => out.push(3),
        TensorOp::Add => out.push(4),
        TensorOp::Sub => out.push(5),
        TensorOp::Mul => out.push(6),
        TensorOp::ReduceSum { axis } => {
            out.push(7);
            out.extend_from_slice(&(*axis as u64).to_le_bytes());
        }
        TensorOp::ScalarMul { scalar } => {
            out.push(8);
            out.extend_from_slice(&scalar.to_le_bytes());
        }
        TensorOp::CommitTensor => out.push(9),
        TensorOp::HashTensor => out.push(10),
        TensorOp::MseLoss => out.push(11),
        TensorOp::LinearBackward => out.push(12),
        TensorOp::SgdUpdate { lr } => {
            out.push(13);
            out.extend_from_slice(&lr.to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::hash_bytes;

    #[test]
    fn program_hash_is_canonical() {
        let seed = hash_bytes(b"test", &[b"seed"]);
        let ops = vec![
            TensorOp::RandomTensor {
                seed,
                shape: vec![2, 2],
                dtype: DType::FieldElement,
            },
            TensorOp::Matmul,
        ];
        assert_eq!(program_hash(&ops), program_hash(&ops));
        let mut changed = ops.clone();
        changed.push(TensorOp::CommitTensor);
        assert_ne!(program_hash(&ops), program_hash(&changed));
    }

    #[test]
    fn program_hash_encodes_every_operation_variant() {
        let seed = hash_bytes(b"test", &[b"all-ops"]);
        let ops = vec![
            TensorOp::RandomTensor {
                seed,
                shape: vec![2, 3],
                dtype: DType::FieldElement,
            },
            TensorOp::Matmul,
            TensorOp::Transpose,
            TensorOp::Add,
            TensorOp::Sub,
            TensorOp::Mul,
            TensorOp::ReduceSum { axis: 1 },
            TensorOp::ScalarMul { scalar: 7 },
            TensorOp::CommitTensor,
            TensorOp::HashTensor,
            TensorOp::MseLoss,
            TensorOp::LinearBackward,
            TensorOp::SgdUpdate { lr: 3 },
        ];
        let base_hash = program_hash(&ops);
        assert_eq!(base_hash, program_hash(&ops));

        for index in 0..ops.len() {
            let mut changed = ops.clone();
            changed[index] = TensorOp::ScalarMul {
                scalar: (index as Elem) + 100,
            };
            assert_ne!(base_hash, program_hash(&changed));
        }
    }

    #[test]
    fn vm_wrappers_match_tensor_operations_and_commitments() {
        let seed = hash_bytes(b"test", &[b"vm-wrappers"]);
        let generated = random_tensor(&seed, vec![2, 2], DType::FieldElement).unwrap();
        assert_eq!(
            generated,
            Tensor::random(&seed, vec![2, 2], DType::FieldElement).unwrap()
        );

        let a = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let b = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![5, 6, 7, 8]).unwrap();
        assert_eq!(
            matmul(&a, &b).unwrap(),
            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![19, 22, 43, 50]).unwrap()
        );
        assert_eq!(
            transpose(&a).unwrap(),
            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 3, 2, 4]).unwrap()
        );
        assert_eq!(
            add(&a, &b).unwrap(),
            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![6, 8, 10, 12]).unwrap()
        );
        assert_eq!(
            sub(&b, &a).unwrap(),
            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![4, 4, 4, 4]).unwrap()
        );
        assert_eq!(
            mul(&a, &b).unwrap(),
            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![5, 12, 21, 32]).unwrap()
        );
        assert_eq!(
            reduce_sum(&a, 0).unwrap(),
            Tensor::from_vec(vec![2], DType::FieldElement, vec![4, 6]).unwrap()
        );
        assert_eq!(
            scalar_mul(&a, 3).unwrap(),
            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![3, 6, 9, 12]).unwrap()
        );
        assert_eq!(commit_tensor(&a), a.commitment_root());
        assert_eq!(hash_tensor(&a), a.hash_tensor());

        let target = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 1, 1, 1]).unwrap();
        assert_eq!(
            mse_loss(&a, &target).unwrap(),
            mse_loss(&a, &target).unwrap()
        );
        assert!(
            mse_loss(
                &a,
                &Tensor::from_vec(vec![1], DType::FieldElement, vec![1]).unwrap()
            )
            .is_err()
        );
    }

    #[test]
    fn linear_backward_and_sgd_match_equations() {
        let x = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let dy = Tensor::from_vec(vec![2, 1], DType::FieldElement, vec![5, 6]).unwrap();
        let grad = linear_backward(&x, &dy).unwrap();
        assert_eq!(
            grad,
            Tensor::from_vec(vec![2, 1], DType::FieldElement, vec![23, 34]).unwrap()
        );
        let w = Tensor::from_vec(vec![2, 1], DType::FieldElement, vec![100, 100]).unwrap();
        let next = sgd_update(&w, &grad, 2).unwrap();
        assert_eq!(
            next,
            Tensor::from_vec(vec![2, 1], DType::FieldElement, vec![54, 32]).unwrap()
        );
    }
}
