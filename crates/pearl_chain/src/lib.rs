//! A self-contained Rust prototype of the matrix-multiplication proof-of-useful-work
//! chain sketched in `pearl.pdf`.
//!
//! The crate intentionally has no third-party dependencies: it includes a small SHA-256
//! implementation, deterministic seed expansion, row-major finite-field matrices, the
//! low-rank-noise cuPoW construction, and a minimal block/chain validator.

pub mod attack;
pub mod chain;
pub mod cupow;
pub mod error;
pub mod field;
pub mod hash;
pub mod matrix;
pub mod oracle;

pub use chain::{Block, BlockHeader, Chain, MatrixJob};
pub use cupow::{CuPowParams, CuPowProof, CuPowSolution, solve, verify};
pub use error::{PearlError, Result};
pub use matrix::Matrix;
