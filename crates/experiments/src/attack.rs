//! Regression helpers for unsafe cuPoW parameterizations.
//!
//! The paper's Algorithm 6.4 uses the same parameter for the transcript tile size
//! and the injected noise rank. This crate originally allowed `rank < tile`; in that
//! generalized setting a zero-matrix miner could compute the exact transcript from the
//! public low-rank factors with much less arithmetic than honest tiled multiplication.
//! Current consensus validation rejects that configuration.

use crate::cupow::{
    CuPowParams, CuPowProof, CuPowSolution, job_commitment, matmul_with_transcript,
};
use crate::error::{PearlError, Result};
use crate::field::{self, Elem};
use crate::hash::Sha256;
use crate::matrix::Matrix;
use crate::oracle::OracleRng;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShortcutWorkEstimate {
    pub honest_tile_mul_terms: u128,
    pub shortcut_mul_terms: u128,
}

impl ShortcutWorkEstimate {
    pub fn estimated_speedup(self) -> f64 {
        self.honest_tile_mul_terms as f64 / self.shortcut_mul_terms.max(1) as f64
    }
}

#[derive(Clone)]
struct PublicFactors {
    el: Matrix,
    er: Matrix,
    fl: Matrix,
    fr: Matrix,
}

pub fn zero_job_shortcut_work(n: usize, params: CuPowParams) -> Result<ShortcutWorkEstimate> {
    validate_square_params(n, params)?;
    let blocks = n / params.tile;
    let tile = params.tile as u128;
    let rank = params.rank as u128;
    let block_count = blocks as u128;

    let honest_tile_mul_terms = block_count.pow(3) * tile.pow(3);
    let prefix_terms = block_count * rank.pow(2) * tile;
    let left_prefix_terms = block_count.pow(2) * tile * rank.pow(2);
    let tile_terms = block_count.pow(3) * tile.pow(2) * rank;

    Ok(ShortcutWorkEstimate {
        honest_tile_mul_terms,
        shortcut_mul_terms: prefix_terms + left_prefix_terms + tile_terms,
    })
}

pub fn solve_zero_job_shortcut(
    seed: &[u8; 32],
    n: usize,
    params: CuPowParams,
) -> Result<CuPowSolution> {
    validate_square_params(n, params)?;
    let factors = derive_zero_job_factors(seed, n, params);
    let transcript_hash = zero_job_transcript_hash(n, params, &factors)?;
    let product = Matrix::zeros(n, n);
    let output_hash = product.commitment();

    Ok(CuPowSolution {
        product,
        proof: CuPowProof {
            transcript_hash,
            output_hash,
        },
    })
}

pub fn solve_zero_job_no_decode(
    seed: &[u8; 32],
    n: usize,
    params: CuPowParams,
) -> Result<CuPowSolution> {
    validate_square_params(n, params)?;
    let factors = derive_zero_job_factors(seed, n, params);
    let e = factors.el.matmul(&factors.er)?;
    let f = factors.fl.matmul(&factors.fr)?;
    let (_, transcript_hash) = matmul_with_transcript(&e, &f, params.tile)?;
    let product = Matrix::zeros(n, n);
    let output_hash = product.commitment();

    Ok(CuPowSolution {
        product,
        proof: CuPowProof {
            transcript_hash,
            output_hash,
        },
    })
}

fn validate_square_params(n: usize, params: CuPowParams) -> Result<()> {
    let zero = Matrix::zeros(n, n);
    params.validate(&zero, &zero)?;
    if params.rank > params.tile {
        return Err(PearlError::InvalidParams(
            "shortcut POC is for rank <= tile".into(),
        ));
    }
    Ok(())
}

fn derive_zero_job_factors(seed: &[u8; 32], n: usize, params: CuPowParams) -> PublicFactors {
    let left = Matrix::zeros(n, n);
    let right = Matrix::zeros(n, n);
    let job_hash = job_commitment(b"noise", &left, &right, params);
    let mut rng = OracleRng::new(b"pearl-chain-cupow-noise-v1", &[seed, &job_hash]);

    PublicFactors {
        el: Matrix::random(n, params.rank, &mut rng),
        er: Matrix::random(params.rank, n, &mut rng),
        fl: Matrix::random(n, params.rank, &mut rng),
        fr: Matrix::random(params.rank, n, &mut rng),
    }
}

fn zero_job_transcript_hash(
    n: usize,
    params: CuPowParams,
    factors: &PublicFactors,
) -> Result<[u8; 32]> {
    let tile = params.tile;
    let rank = params.rank;
    let blocks = n / tile;

    let prefix = prefix_rank_products(blocks, tile, rank, &factors.er, &factors.fl);
    let left_prefix = left_prefix_products(blocks, tile, rank, &factors.el, &prefix);

    let mut hasher = Sha256::new();
    hasher.update(b"pearl-chain-transcript-v1");
    hasher.update_usize(n);
    hasher.update_usize(n);
    hasher.update_usize(n);
    hasher.update_usize(tile);

    for bi in 0..blocks {
        for bj in 0..blocks {
            for bk in 0..blocks {
                hash_shortcut_tile(
                    &mut hasher,
                    ShortcutTile {
                        block_i: bi,
                        block_j: bj,
                        block_k: bk,
                        tile,
                        rank,
                        blocks,
                    },
                    &left_prefix,
                    &factors.fr,
                );
            }
        }
    }

    Ok(hasher.finalize())
}

fn prefix_rank_products(
    blocks: usize,
    tile: usize,
    rank: usize,
    er: &Matrix,
    fl: &Matrix,
) -> Vec<Elem> {
    let mut prefix = vec![0; blocks * rank * rank];
    let mut running = vec![0; rank * rank];

    for bk in 0..blocks {
        let row_base = bk * tile;
        for a in 0..rank {
            for b in 0..rank {
                let mut acc = 0_u128;
                for t in 0..tile {
                    acc += er.get(a, row_base + t) as u128 * fl.get(row_base + t, b) as u128;
                }
                let idx = a * rank + b;
                running[idx] = field::add(running[idx], field::reduce_u128(acc));
            }
        }
        let dst = bk * rank * rank;
        prefix[dst..dst + rank * rank].copy_from_slice(&running);
    }

    prefix
}

fn left_prefix_products(
    blocks: usize,
    tile: usize,
    rank: usize,
    el: &Matrix,
    prefix: &[Elem],
) -> Vec<Elem> {
    let mut out = vec![0; blocks * blocks * tile * rank];

    for bi in 0..blocks {
        let row_base = bi * tile;
        for bk in 0..blocks {
            let s = &prefix[bk * rank * rank..(bk + 1) * rank * rank];
            for ii in 0..tile {
                for b in 0..rank {
                    let mut acc = 0_u128;
                    for a in 0..rank {
                        acc += el.get(row_base + ii, a) as u128 * s[a * rank + b] as u128;
                    }
                    out[left_prefix_index(blocks, tile, rank, bi, bk, ii, b)] =
                        field::reduce_u128(acc);
                }
            }
        }
    }

    out
}

#[derive(Clone, Copy)]
struct ShortcutTile {
    block_i: usize,
    block_j: usize,
    block_k: usize,
    tile: usize,
    rank: usize,
    blocks: usize,
}

fn hash_shortcut_tile(
    hasher: &mut Sha256,
    tile_desc: ShortcutTile,
    left_prefix: &[Elem],
    fr: &Matrix,
) {
    hasher.update_usize(tile_desc.block_i);
    hasher.update_usize(tile_desc.block_j);
    hasher.update_usize(tile_desc.block_k);

    let col_base = tile_desc.block_j * tile_desc.tile;
    for ii in 0..tile_desc.tile {
        for jj in 0..tile_desc.tile {
            let mut acc = 0_u128;
            for b in 0..tile_desc.rank {
                let lhs = left_prefix[left_prefix_index(
                    tile_desc.blocks,
                    tile_desc.tile,
                    tile_desc.rank,
                    tile_desc.block_i,
                    tile_desc.block_k,
                    ii,
                    b,
                )];
                acc += lhs as u128 * fr.get(b, col_base + jj) as u128;
            }
            hasher.update_u64(field::reduce_u128(acc));
        }
    }
}

fn left_prefix_index(
    blocks: usize,
    tile: usize,
    rank: usize,
    block_i: usize,
    block_k: usize,
    ii: usize,
    b: usize,
) -> usize {
    (((block_i * blocks + block_k) * tile + ii) * rank) + b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cupow::{solve, verify};

    #[test]
    fn zero_job_shortcut_matches_honest_proof_for_paper_params() {
        let n = 8;
        let params = CuPowParams {
            tile: 4,
            rank: 4,
            difficulty_bits: 0,
        };
        let seed = [3_u8; 32];
        let zero = Matrix::zeros(n, n);
        let honest = solve(&seed, &zero, &zero, params).unwrap();
        let shortcut = solve_zero_job_shortcut(&seed, n, params).unwrap();

        assert_eq!(shortcut, honest);
        assert!(verify(&seed, &zero, &zero, &shortcut, params).unwrap());
    }

    #[test]
    fn zero_job_no_decode_matches_honest_proof_for_paper_params() {
        let n = 8;
        let params = CuPowParams {
            tile: 4,
            rank: 4,
            difficulty_bits: 0,
        };
        let seed = [11_u8; 32];
        let zero = Matrix::zeros(n, n);
        let honest = solve(&seed, &zero, &zero, params).unwrap();
        let no_decode = solve_zero_job_no_decode(&seed, n, params).unwrap();

        assert_eq!(no_decode, honest);
        assert!(verify(&seed, &zero, &zero, &no_decode, params).unwrap());
    }

    #[test]
    fn unsafe_rank_below_tile_is_rejected() {
        let err = zero_job_shortcut_work(
            64,
            CuPowParams {
                tile: 16,
                rank: 1,
                difficulty_bits: 0,
            },
        )
        .unwrap_err();

        assert!(err.to_string().contains("rank must equal tile"));
    }
}
