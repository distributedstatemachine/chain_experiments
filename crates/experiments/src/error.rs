use std::fmt::{Display, Formatter};

pub type Result<T> = std::result::Result<T, PearlError>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PearlError {
    DimensionMismatch {
        left: (usize, usize),
        right: (usize, usize),
    },
    InvalidParams(String),
    InvalidMatrixData {
        rows: usize,
        cols: usize,
        len: usize,
    },
    InvalidBlock(String),
    NonceExhausted {
        max_nonce: u64,
    },
}

impl Display for PearlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PearlError::DimensionMismatch { left, right } => {
                write!(f, "matrix dimension mismatch: {:?} x {:?}", left, right)
            }
            PearlError::InvalidParams(msg) => write!(f, "invalid cuPoW parameters: {msg}"),
            PearlError::InvalidMatrixData { rows, cols, len } => {
                write!(
                    f,
                    "invalid matrix data: rows={rows}, cols={cols}, len={len}"
                )
            }
            PearlError::InvalidBlock(msg) => write!(f, "invalid block: {msg}"),
            PearlError::NonceExhausted { max_nonce } => {
                write!(f, "no valid proof found through nonce {max_nonce}")
            }
        }
    }
}

impl std::error::Error for PearlError {}
