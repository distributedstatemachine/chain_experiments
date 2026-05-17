use crate::error::{PearlError, Result};
use crate::field::{self, Elem};
use crate::hash::Sha256;
use crate::oracle::OracleRng;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    data: Vec<Elem>,
}

impl Matrix {
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0; rows * cols],
        }
    }

    pub fn from_vec(rows: usize, cols: usize, data: Vec<Elem>) -> Result<Self> {
        if rows * cols != data.len() {
            return Err(PearlError::InvalidMatrixData {
                rows,
                cols,
                len: data.len(),
            });
        }
        Ok(Self {
            rows,
            cols,
            data: data.into_iter().map(field::normalize).collect(),
        })
    }

    pub fn random(rows: usize, cols: usize, rng: &mut OracleRng) -> Self {
        let mut data = Vec::with_capacity(rows * cols);
        for _ in 0..rows * cols {
            data.push(rng.next_field());
        }
        Self { rows, cols, data }
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn as_slice(&self) -> &[Elem] {
        &self.data
    }

    pub fn get(&self, row: usize, col: usize) -> Elem {
        self.data[row * self.cols + col]
    }

    pub fn set(&mut self, row: usize, col: usize, value: Elem) {
        self.data[row * self.cols + col] = field::normalize(value);
    }

    pub fn commitment(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"pearl-chain-matrix-v1");
        self.hash_into(&mut hasher);
        hasher.finalize()
    }

    pub fn hash_into(&self, hasher: &mut Sha256) {
        hasher.update_usize(self.rows);
        hasher.update_usize(self.cols);
        for value in &self.data {
            hasher.update_u64(*value);
        }
    }

    pub fn add(&self, rhs: &Self) -> Result<Self> {
        self.check_same_shape(rhs)?;
        let data = self
            .data
            .iter()
            .zip(&rhs.data)
            .map(|(lhs, rhs)| field::add(*lhs, *rhs))
            .collect();
        Ok(Self {
            rows: self.rows,
            cols: self.cols,
            data,
        })
    }

    pub fn sub(&self, rhs: &Self) -> Result<Self> {
        self.check_same_shape(rhs)?;
        let data = self
            .data
            .iter()
            .zip(&rhs.data)
            .map(|(lhs, rhs)| field::sub(*lhs, *rhs))
            .collect();
        Ok(Self {
            rows: self.rows,
            cols: self.cols,
            data,
        })
    }

    pub fn matmul(&self, rhs: &Self) -> Result<Self> {
        if self.cols != rhs.rows {
            return Err(PearlError::DimensionMismatch {
                left: (self.rows, self.cols),
                right: (rhs.rows, rhs.cols),
            });
        }

        let rhs_t = rhs.transpose();
        let mut data = vec![0; self.rows * rhs.cols];

        for i in 0..self.rows {
            let lhs_row = &self.data[i * self.cols..(i + 1) * self.cols];
            for j in 0..rhs.cols {
                let rhs_row = &rhs_t.data[j * rhs.rows..(j + 1) * rhs.rows];
                let mut acc = 0_u128;
                for k in 0..self.cols {
                    acc += lhs_row[k] as u128 * rhs_row[k] as u128;
                }
                data[i * rhs.cols + j] = field::reduce_u128(acc);
            }
        }

        Ok(Self {
            rows: self.rows,
            cols: rhs.cols,
            data,
        })
    }

    pub fn transpose(&self) -> Self {
        let mut out = vec![0; self.data.len()];
        for row in 0..self.rows {
            for col in 0..self.cols {
                out[col * self.rows + row] = self.get(row, col);
            }
        }
        Self {
            rows: self.cols,
            cols: self.rows,
            data: out,
        }
    }

    pub(crate) fn raw_get(&self, row: usize, col: usize) -> Elem {
        self.data[row * self.cols + col]
    }

    pub(crate) fn raw_set(&mut self, row: usize, col: usize, value: Elem) {
        self.data[row * self.cols + col] = value;
    }

    fn check_same_shape(&self, rhs: &Self) -> Result<()> {
        if self.rows != rhs.rows || self.cols != rhs.cols {
            return Err(PearlError::DimensionMismatch {
                left: (self.rows, self.cols),
                right: (rhs.rows, rhs.cols),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_multiply_small_case() {
        let a = Matrix::from_vec(2, 3, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let b = Matrix::from_vec(3, 2, vec![7, 8, 9, 10, 11, 12]).unwrap();
        let c = a.matmul(&b).unwrap();
        assert_eq!(c, Matrix::from_vec(2, 2, vec![58, 64, 139, 154]).unwrap());
    }

    #[test]
    fn add_and_sub_roundtrip() {
        let a = Matrix::from_vec(2, 2, vec![1, 2, 3, 4]).unwrap();
        let b = Matrix::from_vec(2, 2, vec![9, 8, 7, 6]).unwrap();
        assert_eq!(a.add(&b).unwrap().sub(&b).unwrap(), a);
    }
}
