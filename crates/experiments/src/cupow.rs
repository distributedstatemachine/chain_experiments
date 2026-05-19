use crate::error::{PearlError, Result};
use crate::field;
use crate::hash::{Sha256, meets_difficulty};
use crate::matrix::Matrix;
use crate::oracle::OracleRng;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CuPowParams {
    pub tile: usize,
    pub rank: usize,
    pub difficulty_bits: u32,
}

impl CuPowParams {
    pub fn validate(&self, left: &Matrix, right: &Matrix) -> Result<()> {
        if left.cols() != right.rows() {
            return Err(PearlError::DimensionMismatch {
                left: (left.rows(), left.cols()),
                right: (right.rows(), right.cols()),
            });
        }
        if self.tile == 0 {
            return Err(PearlError::InvalidParams("tile must be non-zero".into()));
        }
        if self.rank == 0 {
            return Err(PearlError::InvalidParams("rank must be non-zero".into()));
        }
        if self.rank != self.tile {
            return Err(PearlError::InvalidParams(
                "rank must equal tile; Algorithm 6.4 uses one parameter r for both".into(),
            ));
        }
        if self.difficulty_bits > 256 {
            return Err(PearlError::InvalidParams(
                "difficulty_bits must be at most 256".into(),
            ));
        }
        if !left.rows().is_multiple_of(self.tile)
            || !left.cols().is_multiple_of(self.tile)
            || !right.cols().is_multiple_of(self.tile)
        {
            return Err(PearlError::InvalidParams(
                "tile must divide rows, shared dimension, and columns".into(),
            ));
        }
        let max_rank = left.rows().min(left.cols()).min(right.cols());
        if self.rank > max_rank {
            return Err(PearlError::InvalidParams(format!(
                "rank {} exceeds max usable rank {}",
                self.rank, max_rank
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CuPowProof {
    pub transcript_hash: [u8; 32],
    pub output_hash: [u8; 32],
}

impl CuPowProof {
    pub fn meets_difficulty(&self, difficulty_bits: u32) -> bool {
        meets_difficulty(&self.transcript_hash, difficulty_bits)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CuPowSolution {
    pub product: Matrix,
    pub proof: CuPowProof,
}

#[derive(Clone)]
struct LowRankNoise {
    el: Matrix,
    er: Matrix,
    fl: Matrix,
    fr: Matrix,
    e: Matrix,
    f: Matrix,
}

pub fn job_commitment(
    seed_domain: &[u8],
    left: &Matrix,
    right: &Matrix,
    params: CuPowParams,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"pearl-chain-job-v1");
    hasher.update_len_prefixed(seed_domain);
    hasher.update_usize(params.tile);
    hasher.update_usize(params.rank);
    hasher.update_u32(params.difficulty_bits);
    left.hash_into(&mut hasher);
    right.hash_into(&mut hasher);
    hasher.finalize()
}

pub fn solve(
    seed: &[u8; 32],
    left: &Matrix,
    right: &Matrix,
    params: CuPowParams,
) -> Result<CuPowSolution> {
    params.validate(left, right)?;

    let noise = derive_noise(seed, left, right, params)?;
    let left_prime = left.add(&noise.e)?;
    let right_prime = right.add(&noise.f)?;

    let (noisy_product, transcript_hash) =
        matmul_with_transcript(&left_prime, &right_prime, params.tile)?;
    let product = decode(left, &right_prime, &noisy_product, &noise)?;
    let output_hash = product.commitment();

    Ok(CuPowSolution {
        product,
        proof: CuPowProof {
            transcript_hash,
            output_hash,
        },
    })
}

pub fn verify(
    seed: &[u8; 32],
    left: &Matrix,
    right: &Matrix,
    solution: &CuPowSolution,
    params: CuPowParams,
) -> Result<bool> {
    let expected = solve(seed, left, right, params)?;
    Ok(expected == *solution && solution.proof.meets_difficulty(params.difficulty_bits))
}

fn derive_noise(
    seed: &[u8; 32],
    left: &Matrix,
    right: &Matrix,
    params: CuPowParams,
) -> Result<LowRankNoise> {
    let job_hash = job_commitment(b"noise", left, right, params);
    let mut rng = OracleRng::new(b"pearl-chain-cupow-noise-v1", &[seed, &job_hash]);

    let el = Matrix::random(left.rows(), params.rank, &mut rng);
    let er = Matrix::random(params.rank, left.cols(), &mut rng);
    let fl = Matrix::random(right.rows(), params.rank, &mut rng);
    let fr = Matrix::random(params.rank, right.cols(), &mut rng);

    let e = el.matmul(&er)?;
    let f = fl.matmul(&fr)?;

    Ok(LowRankNoise {
        el,
        er,
        fl,
        fr,
        e,
        f,
    })
}

fn decode(
    left: &Matrix,
    right_prime: &Matrix,
    noisy_product: &Matrix,
    noise: &LowRankNoise,
) -> Result<Matrix> {
    let a_fl = left.matmul(&noise.fl)?;
    let a_f = a_fl.matmul(&noise.fr)?;
    let er_b_prime = noise.er.matmul(right_prime)?;
    let e_b_prime = noise.el.matmul(&er_b_prime)?;
    let correction = a_f.add(&e_b_prime)?;
    noisy_product.sub(&correction)
}

pub(crate) fn matmul_with_transcript(
    left: &Matrix,
    right: &Matrix,
    tile: usize,
) -> Result<(Matrix, [u8; 32])> {
    if left.cols() != right.rows() {
        return Err(PearlError::DimensionMismatch {
            left: (left.rows(), left.cols()),
            right: (right.rows(), right.cols()),
        });
    }
    if !left.rows().is_multiple_of(tile)
        || !left.cols().is_multiple_of(tile)
        || !right.cols().is_multiple_of(tile)
    {
        return Err(PearlError::InvalidParams(
            "tile must divide rows, shared dimension, and columns".into(),
        ));
    }

    let mut product = Matrix::zeros(left.rows(), right.cols());
    let mut hasher = Sha256::new();
    hasher.update(b"pearl-chain-transcript-v1");
    hasher.update_usize(left.rows());
    hasher.update_usize(left.cols());
    hasher.update_usize(right.cols());
    hasher.update_usize(tile);

    let block_rows = left.rows() / tile;
    let block_inner = left.cols() / tile;
    let block_cols = right.cols() / tile;
    let mut scratch = vec![0_u128; tile * tile];

    for bi in 0..block_rows {
        for bj in 0..block_cols {
            for bk in 0..block_inner {
                add_tile_product(
                    &mut product,
                    left,
                    right,
                    TileBlock {
                        i: bi,
                        j: bj,
                        k: bk,
                        size: tile,
                    },
                    &mut scratch,
                );
                hash_tile(&mut hasher, &product, bi, bj, bk, tile);
            }
        }
    }

    Ok((product, hasher.finalize()))
}

#[derive(Clone, Copy)]
struct TileBlock {
    i: usize,
    j: usize,
    k: usize,
    size: usize,
}

fn add_tile_product(
    product: &mut Matrix,
    left: &Matrix,
    right: &Matrix,
    block: TileBlock,
    scratch: &mut [u128],
) {
    let tile = block.size;
    debug_assert_eq!(scratch.len(), tile * tile);
    scratch.fill(0);

    let row_base = block.i * tile;
    let col_base = block.j * tile;
    let inner_base = block.k * tile;

    for ii in 0..tile {
        for jj in 0..tile {
            scratch[ii * tile + jj] = product.raw_get(row_base + ii, col_base + jj) as u128;
        }
    }

    for kk in 0..tile {
        let inner = inner_base + kk;
        for ii in 0..tile {
            let lhs = left.raw_get(row_base + ii, inner) as u128;
            if lhs == 0 {
                continue;
            }
            for jj in 0..tile {
                scratch[ii * tile + jj] += lhs * right.raw_get(inner, col_base + jj) as u128;
            }
        }
    }

    for ii in 0..tile {
        for jj in 0..tile {
            product.raw_set(
                row_base + ii,
                col_base + jj,
                field::reduce_u128(scratch[ii * tile + jj]),
            );
        }
    }
}

fn hash_tile(
    hasher: &mut Sha256,
    product: &Matrix,
    block_i: usize,
    block_j: usize,
    block_k: usize,
    tile: usize,
) {
    hasher.update_usize(block_i);
    hasher.update_usize(block_j);
    hasher.update_usize(block_k);
    for ii in 0..tile {
        for jj in 0..tile {
            hasher.update_u64(product.raw_get(block_i * tile + ii, block_j * tile + jj));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle::OracleRng;

    fn sample_job() -> (Matrix, Matrix, CuPowParams) {
        let mut rng = OracleRng::new(b"sample", &[b"matrices"]);
        let left = Matrix::random(4, 4, &mut rng);
        let right = Matrix::random(4, 4, &mut rng);
        let params = CuPowParams {
            tile: 2,
            rank: 2,
            difficulty_bits: 0,
        };
        (left, right, params)
    }

    #[test]
    fn cupow_solves_useful_product() {
        let (left, right, params) = sample_job();
        let seed = [7_u8; 32];
        let solution = solve(&seed, &left, &right, params).unwrap();
        let direct = left.matmul(&right).unwrap();
        assert_eq!(solution.product, direct);
        assert!(verify(&seed, &left, &right, &solution, params).unwrap());
    }

    #[test]
    fn cupow_rejects_tampered_product() {
        let (left, right, params) = sample_job();
        let seed = [9_u8; 32];
        let mut solution = solve(&seed, &left, &right, params).unwrap();
        solution.product.set(0, 0, solution.product.get(0, 0) + 1);
        assert!(!verify(&seed, &left, &right, &solution, params).unwrap());
    }

    #[test]
    fn transcript_matmul_matches_direct_multiplication() {
        let (left, right, params) = sample_job();
        let (product, _) = matmul_with_transcript(&left, &right, params.tile).unwrap();
        assert_eq!(product, left.matmul(&right).unwrap());
    }
}
