use std::time::{SystemTime, UNIX_EPOCH};

use crate::cupow::{CuPowParams, CuPowSolution, job_commitment, solve, verify};
use crate::error::{PearlError, Result};
use crate::hash::{Sha256, meets_difficulty};
use crate::matrix::Matrix;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatrixJob {
    pub left: Matrix,
    pub right: Matrix,
    pub params: CuPowParams,
}

impl MatrixJob {
    pub fn new(left: Matrix, right: Matrix, params: CuPowParams) -> Result<Self> {
        params.validate(&left, &right)?;
        Ok(Self {
            left,
            right,
            params,
        })
    }

    pub fn commitment(&self) -> [u8; 32] {
        job_commitment(b"chain-job", &self.left, &self.right, self.params)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockHeader {
    pub height: u64,
    pub prev_hash: [u8; 32],
    pub timestamp_ms: u64,
    pub nonce: u64,
    pub difficulty_bits: u32,
    pub job_hash: [u8; 32],
    pub product_hash: [u8; 32],
    pub transcript_hash: [u8; 32],
}

impl BlockHeader {
    pub fn mining_seed(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"pearl-chain-mining-seed-v1");
        hasher.update_u64(self.height);
        hasher.update(&self.prev_hash);
        hasher.update_u64(self.timestamp_ms);
        hasher.update_u64(self.nonce);
        hasher.update_u32(self.difficulty_bits);
        hasher.update(&self.job_hash);
        hasher.finalize()
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"pearl-chain-block-header-v1");
        hasher.update_u64(self.height);
        hasher.update(&self.prev_hash);
        hasher.update_u64(self.timestamp_ms);
        hasher.update_u64(self.nonce);
        hasher.update_u32(self.difficulty_bits);
        hasher.update(&self.job_hash);
        hasher.update(&self.product_hash);
        hasher.update(&self.transcript_hash);
        hasher.finalize()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    pub header: BlockHeader,
    pub job: MatrixJob,
    pub solution: CuPowSolution,
}

impl Block {
    pub fn mine(
        height: u64,
        prev_hash: [u8; 32],
        job: MatrixJob,
        timestamp_ms: u64,
        max_nonce: u64,
    ) -> Result<Self> {
        let job_hash = job.commitment();
        for nonce in 0..=max_nonce {
            let mut header = BlockHeader {
                height,
                prev_hash,
                timestamp_ms,
                nonce,
                difficulty_bits: job.params.difficulty_bits,
                job_hash,
                product_hash: [0; 32],
                transcript_hash: [0; 32],
            };
            let seed = header.mining_seed();
            let solution = solve(&seed, &job.left, &job.right, job.params)?;
            if !solution.proof.meets_difficulty(job.params.difficulty_bits) {
                continue;
            }

            header.product_hash = solution.product.commitment();
            header.transcript_hash = solution.proof.transcript_hash;
            return Ok(Self {
                header,
                job,
                solution,
            });
        }
        Err(PearlError::NonceExhausted { max_nonce })
    }

    pub fn validate(&self, expected_height: u64, expected_prev_hash: [u8; 32]) -> Result<bool> {
        if self.header.height != expected_height {
            return Err(PearlError::InvalidBlock(format!(
                "height {} does not match expected {}",
                self.header.height, expected_height
            )));
        }
        if self.header.prev_hash != expected_prev_hash {
            return Err(PearlError::InvalidBlock("previous hash mismatch".into()));
        }
        if self.header.job_hash != self.job.commitment() {
            return Err(PearlError::InvalidBlock("job commitment mismatch".into()));
        }
        if self.header.difficulty_bits != self.job.params.difficulty_bits {
            return Err(PearlError::InvalidBlock("difficulty mismatch".into()));
        }
        if self.header.product_hash != self.solution.product.commitment() {
            return Err(PearlError::InvalidBlock(
                "product commitment mismatch".into(),
            ));
        }
        if self.header.transcript_hash != self.solution.proof.transcript_hash {
            return Err(PearlError::InvalidBlock(
                "transcript commitment mismatch".into(),
            ));
        }
        if self.solution.proof.output_hash != self.header.product_hash {
            return Err(PearlError::InvalidBlock(
                "proof output commitment mismatch".into(),
            ));
        }
        if !meets_difficulty(&self.header.transcript_hash, self.header.difficulty_bits) {
            return Ok(false);
        }

        let seed = self.header.mining_seed();
        verify(
            &seed,
            &self.job.left,
            &self.job.right,
            &self.solution,
            self.job.params,
        )
    }

    pub fn hash(&self) -> [u8; 32] {
        self.header.hash()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Chain {
    blocks: Vec<Block>,
}

impl Chain {
    pub fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    pub fn tip_hash(&self) -> [u8; 32] {
        self.blocks.last().map(Block::hash).unwrap_or([0; 32])
    }

    pub fn append_mined(&mut self, job: MatrixJob, max_nonce: u64) -> Result<&Block> {
        let height = self.blocks.len() as u64;
        let prev_hash = self.tip_hash();
        let block = Block::mine(height, prev_hash, job, now_ms(), max_nonce)?;
        self.append(block)?;
        Ok(self.blocks.last().expect("block just appended"))
    }

    pub fn append(&mut self, block: Block) -> Result<()> {
        let expected_height = self.blocks.len() as u64;
        let expected_prev_hash = self.tip_hash();
        if !block.validate(expected_height, expected_prev_hash)? {
            return Err(PearlError::InvalidBlock(
                "proof does not meet difficulty".into(),
            ));
        }
        self.blocks.push(block);
        Ok(())
    }

    pub fn validate(&self) -> Result<bool> {
        let mut prev_hash = [0_u8; 32];
        for (height, block) in self.blocks.iter().enumerate() {
            if !block.validate(height as u64, prev_hash)? {
                return Ok(false);
            }
            prev_hash = block.hash();
        }
        Ok(true)
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle::OracleRng;

    fn job(difficulty_bits: u32) -> MatrixJob {
        let mut rng = OracleRng::new(b"chain-test", &[&difficulty_bits.to_le_bytes()]);
        let left = Matrix::random(4, 4, &mut rng);
        let right = Matrix::random(4, 4, &mut rng);
        MatrixJob::new(
            left,
            right,
            CuPowParams {
                tile: 2,
                rank: 2,
                difficulty_bits,
            },
        )
        .unwrap()
    }

    #[test]
    fn mines_and_validates_block() {
        let block = Block::mine(0, [0; 32], job(0), 1_700_000_000_000, 0).unwrap();
        assert!(block.validate(0, [0; 32]).unwrap());
    }

    #[test]
    fn appends_and_validates_chain() {
        let mut chain = Chain::new();
        chain.append_mined(job(0), 0).unwrap();
        chain.append_mined(job(0), 0).unwrap();
        assert!(chain.validate().unwrap());
    }
}
