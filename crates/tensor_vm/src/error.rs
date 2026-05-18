use std::fmt;

pub type Result<T> = std::result::Result<T, TvmError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TvmError {
    EmptyShape,
    UnsupportedRank { rank: usize },
    InvalidTensorData { expected: usize, actual: usize },
    ShapeMismatch { left: Vec<usize>, right: Vec<usize> },
    DimensionMismatch { left: Vec<usize>, right: Vec<usize> },
    InvalidIndex { index: usize, len: usize },
    InvalidAxis { axis: usize, rank: usize },
    InvalidChunk { chunk_index: u64 },
    InvalidMerkleProof,
    InvalidReceipt(&'static str),
    VerificationFailed(&'static str),
    Storage(&'static str),
    UnknownMiner,
    UnknownValidator,
    UnknownReceipt,
    InsufficientStake,
}

impl fmt::Display for TvmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyShape => write!(f, "tensor shape must not be empty"),
            Self::UnsupportedRank { rank } => write!(f, "unsupported tensor rank {rank}"),
            Self::InvalidTensorData { expected, actual } => {
                write!(
                    f,
                    "invalid tensor data length: expected {expected}, got {actual}"
                )
            }
            Self::ShapeMismatch { left, right } => {
                write!(f, "shape mismatch: {left:?} != {right:?}")
            }
            Self::DimensionMismatch { left, right } => {
                write!(f, "dimension mismatch: {left:?} cannot multiply {right:?}")
            }
            Self::InvalidIndex { index, len } => {
                write!(f, "invalid index {index} for length {len}")
            }
            Self::InvalidAxis { axis, rank } => write!(f, "invalid axis {axis} for rank {rank}"),
            Self::InvalidChunk { chunk_index } => write!(f, "invalid tensor chunk {chunk_index}"),
            Self::InvalidMerkleProof => write!(f, "invalid Merkle proof"),
            Self::InvalidReceipt(reason) => write!(f, "invalid receipt: {reason}"),
            Self::VerificationFailed(reason) => write!(f, "verification failed: {reason}"),
            Self::Storage(reason) => write!(f, "storage error: {reason}"),
            Self::UnknownMiner => write!(f, "unknown miner"),
            Self::UnknownValidator => write!(f, "unknown validator"),
            Self::UnknownReceipt => write!(f, "unknown receipt"),
            Self::InsufficientStake => write!(f, "insufficient stake"),
        }
    }
}

impl std::error::Error for TvmError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_covers_public_variants() {
        let errors = [
            (TvmError::EmptyShape, "tensor shape must not be empty"),
            (
                TvmError::UnsupportedRank { rank: 3 },
                "unsupported tensor rank 3",
            ),
            (
                TvmError::InvalidTensorData {
                    expected: 2,
                    actual: 1,
                },
                "invalid tensor data length: expected 2, got 1",
            ),
            (
                TvmError::ShapeMismatch {
                    left: vec![1, 2],
                    right: vec![2, 1],
                },
                "shape mismatch: [1, 2] != [2, 1]",
            ),
            (
                TvmError::DimensionMismatch {
                    left: vec![1, 2],
                    right: vec![3, 4],
                },
                "dimension mismatch: [1, 2] cannot multiply [3, 4]",
            ),
            (
                TvmError::InvalidIndex { index: 4, len: 2 },
                "invalid index 4 for length 2",
            ),
            (
                TvmError::InvalidAxis { axis: 2, rank: 1 },
                "invalid axis 2 for rank 1",
            ),
            (
                TvmError::InvalidChunk { chunk_index: 9 },
                "invalid tensor chunk 9",
            ),
            (TvmError::InvalidMerkleProof, "invalid Merkle proof"),
            (TvmError::InvalidReceipt("bad"), "invalid receipt: bad"),
            (
                TvmError::VerificationFailed("bad"),
                "verification failed: bad",
            ),
            (TvmError::Storage("bad"), "storage error: bad"),
            (TvmError::UnknownMiner, "unknown miner"),
            (TvmError::UnknownValidator, "unknown validator"),
            (TvmError::UnknownReceipt, "unknown receipt"),
            (TvmError::InsufficientStake, "insufficient stake"),
        ];

        for (error, expected) in errors {
            assert_eq!(error.to_string(), expected);
        }
    }
}
