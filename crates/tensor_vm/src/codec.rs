use crate::chain::{BlockVote, JobState, TensorBlock};
use crate::jobs::{LinearTrainingStepJob, MatmulJob, PrimitiveType};
use crate::tensor::DType;
use crate::types::Hash;
use crate::verify::VerificationResult;

pub(crate) const TENSOR_BLOCK_PAYLOAD_LEN: usize = 8 * 4 + 32 * 11;
pub(crate) const BLOCK_VOTE_PAYLOAD_LEN: usize = 32 * 3 + 8 * 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CodecError {
    Truncated,
    TrailingBytes,
    UnknownJobTag,
    UnknownDType,
    InvalidOptionalU64,
    UsizeOverflow,
    ShapeVectorTooLarge,
}

pub(crate) fn dtype_tag(dtype: DType) -> u8 {
    dtype.tag()
}

pub(crate) fn dtype_from_tag(tag: u8) -> Option<DType> {
    match tag {
        1 => Some(DType::Int32),
        2 => Some(DType::Int64),
        3 => Some(DType::Fixed32),
        4 => Some(DType::FieldElement),
        _ => None,
    }
}

pub(crate) fn primitive_type_tag(primitive_type: PrimitiveType) -> u8 {
    match primitive_type {
        PrimitiveType::TensorOp => 1,
        PrimitiveType::LinearTrainingStep => 2,
    }
}

pub(crate) fn primitive_type_from_tag(tag: u8) -> Option<PrimitiveType> {
    match tag {
        1 => Some(PrimitiveType::TensorOp),
        2 => Some(PrimitiveType::LinearTrainingStep),
        _ => None,
    }
}

pub(crate) fn verification_result_tag(result: VerificationResult) -> u8 {
    match result {
        VerificationResult::Valid => 1,
        VerificationResult::Invalid => 2,
        VerificationResult::Unavailable => 3,
    }
}

pub(crate) fn verification_result_from_tag(tag: u8) -> Option<VerificationResult> {
    match tag {
        1 => Some(VerificationResult::Valid),
        2 => Some(VerificationResult::Invalid),
        3 => Some(VerificationResult::Unavailable),
        _ => None,
    }
}

pub(crate) fn encode_tensor_block_payload(block: &TensorBlock) -> Vec<u8> {
    let mut out = Vec::with_capacity(TENSOR_BLOCK_PAYLOAD_LEN);
    write_u64(&mut out, block.height);
    write_hash(&mut out, &block.parent_hash);
    write_u64(&mut out, block.epoch);
    write_hash(&mut out, &block.proposer);
    write_hash(&mut out, &block.settled_receipt_set_root);
    write_hash(&mut out, &block.checks_root);
    write_hash(&mut out, &block.attestation_root);
    write_hash(&mut out, &block.state_root);
    write_hash(&mut out, &block.reward_root);
    write_hash(&mut out, &block.beacon);
    write_hash(&mut out, &block.difficulty_target);
    write_u64(&mut out, block.nonce);
    write_u64(&mut out, block.timestamp);
    write_hash(&mut out, &block.proposer_signature);
    write_hash(&mut out, &block.validator_signature_aggregate);
    out
}

pub(crate) fn decode_tensor_block_payload(input: &[u8]) -> Option<TensorBlock> {
    if input.len() != TENSOR_BLOCK_PAYLOAD_LEN {
        return None;
    }
    let mut offset = 0;
    let block = TensorBlock {
        height: read_u64(input, &mut offset).ok()?,
        parent_hash: read_hash(input, &mut offset).ok()?,
        epoch: read_u64(input, &mut offset).ok()?,
        proposer: read_hash(input, &mut offset).ok()?,
        settled_receipt_set_root: read_hash(input, &mut offset).ok()?,
        checks_root: read_hash(input, &mut offset).ok()?,
        attestation_root: read_hash(input, &mut offset).ok()?,
        state_root: read_hash(input, &mut offset).ok()?,
        reward_root: read_hash(input, &mut offset).ok()?,
        beacon: read_hash(input, &mut offset).ok()?,
        difficulty_target: read_hash(input, &mut offset).ok()?,
        nonce: read_u64(input, &mut offset).ok()?,
        timestamp: read_u64(input, &mut offset).ok()?,
        proposer_signature: read_hash(input, &mut offset).ok()?,
        validator_signature_aggregate: read_hash(input, &mut offset).ok()?,
    };
    (offset == input.len()).then_some(block)
}

pub(crate) fn encode_block_vote_payload(vote: &BlockVote) -> Vec<u8> {
    let mut out = Vec::with_capacity(BLOCK_VOTE_PAYLOAD_LEN);
    write_hash(&mut out, &vote.validator);
    write_hash(&mut out, &vote.block_hash);
    write_u64(&mut out, vote.block_height);
    write_u64(&mut out, vote.stake);
    write_hash(&mut out, &vote.signature);
    out
}

pub(crate) fn decode_block_vote_payload(input: &[u8]) -> Option<BlockVote> {
    if input.len() != BLOCK_VOTE_PAYLOAD_LEN {
        return None;
    }
    let mut offset = 0;
    let vote = BlockVote {
        validator: read_hash(input, &mut offset).ok()?,
        block_hash: read_hash(input, &mut offset).ok()?,
        block_height: read_u64(input, &mut offset).ok()?,
        stake: read_u64(input, &mut offset).ok()?,
        signature: read_hash(input, &mut offset).ok()?,
    };
    (offset == input.len()).then_some(vote)
}

pub(crate) fn encode_job_payload(job: &JobState) -> Vec<u8> {
    let mut out = Vec::new();
    match job {
        JobState::TensorOp(job) => {
            out.push(1);
            write_hash(&mut out, &job.job_id);
            write_u64(&mut out, job.epoch);
            write_usize(&mut out, job.m);
            write_usize(&mut out, job.k);
            write_usize(&mut out, job.n);
            out.push(dtype_tag(job.dtype));
            write_optional_u64(&mut out, job.modulus);
            write_hash(&mut out, &job.seed_a);
            write_hash(&mut out, &job.seed_b);
            write_u64(&mut out, job.deadline_block);
            write_u64(&mut out, job.reward_weight);
        }
        JobState::LinearTrainingStep(job) => {
            out.push(2);
            write_hash(&mut out, &job.job_id);
            write_hash(&mut out, &job.model_id);
            write_u64(&mut out, job.step);
            write_hash(&mut out, &job.batch_seed);
            write_hash(&mut out, &job.weight_root_before);
            write_usize_vec(&mut out, &job.input_shape);
            write_usize_vec(&mut out, &job.weight_shape);
            write_usize_vec(&mut out, &job.target_shape);
            write_u64(&mut out, job.lr);
            out.push(dtype_tag(job.dtype));
            write_u64(&mut out, job.deadline_block);
            write_u64(&mut out, job.reward_weight);
        }
    }
    out
}

pub(crate) fn decode_job_payload(
    input: &[u8],
    max_shape_dims: Option<usize>,
) -> Result<JobState, CodecError> {
    let mut offset = 0;
    let job = decode_job_payload_from(input, &mut offset, max_shape_dims)?;
    if offset != input.len() {
        return Err(CodecError::TrailingBytes);
    }
    Ok(job)
}

pub(crate) fn decode_job_payload_from(
    input: &[u8],
    offset: &mut usize,
    max_shape_dims: Option<usize>,
) -> Result<JobState, CodecError> {
    match read_u8(input, offset)? {
        1 => Ok(JobState::TensorOp(MatmulJob {
            job_id: read_hash(input, offset)?,
            epoch: read_u64(input, offset)?,
            m: read_usize(input, offset)?,
            k: read_usize(input, offset)?,
            n: read_usize(input, offset)?,
            dtype: dtype_from_tag(read_u8(input, offset)?).ok_or(CodecError::UnknownDType)?,
            modulus: read_optional_u64(input, offset)?,
            seed_a: read_hash(input, offset)?,
            seed_b: read_hash(input, offset)?,
            deadline_block: read_u64(input, offset)?,
            reward_weight: read_u64(input, offset)?,
        })),
        2 => Ok(JobState::LinearTrainingStep(LinearTrainingStepJob {
            job_id: read_hash(input, offset)?,
            model_id: read_hash(input, offset)?,
            step: read_u64(input, offset)?,
            batch_seed: read_hash(input, offset)?,
            weight_root_before: read_hash(input, offset)?,
            input_shape: read_usize_vec(input, offset, max_shape_dims)?,
            weight_shape: read_usize_vec(input, offset, max_shape_dims)?,
            target_shape: read_usize_vec(input, offset, max_shape_dims)?,
            lr: read_u64(input, offset)?,
            dtype: dtype_from_tag(read_u8(input, offset)?).ok_or(CodecError::UnknownDType)?,
            deadline_block: read_u64(input, offset)?,
            reward_weight: read_u64(input, offset)?,
        })),
        _ => Err(CodecError::UnknownJobTag),
    }
}

fn write_hash(out: &mut Vec<u8>, hash: &Hash) {
    out.extend_from_slice(hash);
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_usize(out: &mut Vec<u8>, value: usize) {
    write_u64(out, value as u64);
}

fn write_usize_vec(out: &mut Vec<u8>, values: &[usize]) {
    write_usize(out, values.len());
    for value in values {
        write_usize(out, *value);
    }
}

fn write_optional_u64(out: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            out.push(1);
            write_u64(out, value);
        }
        None => out.push(0),
    }
}

fn read_u8(input: &[u8], offset: &mut usize) -> Result<u8, CodecError> {
    let byte = input.get(*offset).copied().ok_or(CodecError::Truncated)?;
    *offset += 1;
    Ok(byte)
}

fn read_hash(input: &[u8], offset: &mut usize) -> Result<Hash, CodecError> {
    let end = (*offset).checked_add(32).ok_or(CodecError::Truncated)?;
    let bytes = input.get(*offset..end).ok_or(CodecError::Truncated)?;
    let mut out = [0_u8; 32];
    out.copy_from_slice(bytes);
    *offset = end;
    Ok(out)
}

fn read_u64(input: &[u8], offset: &mut usize) -> Result<u64, CodecError> {
    let end = (*offset).checked_add(8).ok_or(CodecError::Truncated)?;
    let bytes = input.get(*offset..end).ok_or(CodecError::Truncated)?;
    let mut out = [0_u8; 8];
    out.copy_from_slice(bytes);
    *offset = end;
    Ok(u64::from_le_bytes(out))
}

fn read_usize(input: &[u8], offset: &mut usize) -> Result<usize, CodecError> {
    usize::try_from(read_u64(input, offset)?).map_err(|_| CodecError::UsizeOverflow)
}

fn read_usize_vec(
    input: &[u8],
    offset: &mut usize,
    max_shape_dims: Option<usize>,
) -> Result<Vec<usize>, CodecError> {
    let len = read_usize(input, offset)?;
    if let Some(max_shape_dims) = max_shape_dims
        && len > max_shape_dims
    {
        return Err(CodecError::ShapeVectorTooLarge);
    }
    let mut values = Vec::with_capacity(len);
    for _ in 0..len {
        values.push(read_usize(input, offset)?);
    }
    Ok(values)
}

fn read_optional_u64(input: &[u8], offset: &mut usize) -> Result<Option<u64>, CodecError> {
    match read_u8(input, offset)? {
        0 => Ok(None),
        1 => Ok(Some(read_u64(input, offset)?)),
        _ => Err(CodecError::InvalidOptionalU64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_enum_tags_roundtrip_and_reject_unknown_tags() {
        for dtype in [
            DType::Int32,
            DType::Int64,
            DType::Fixed32,
            DType::FieldElement,
        ] {
            assert_eq!(dtype_from_tag(dtype_tag(dtype)), Some(dtype));
        }

        for primitive_type in [PrimitiveType::TensorOp, PrimitiveType::LinearTrainingStep] {
            assert_eq!(
                primitive_type_from_tag(primitive_type_tag(primitive_type)),
                Some(primitive_type)
            );
        }

        for result in [
            VerificationResult::Valid,
            VerificationResult::Invalid,
            VerificationResult::Unavailable,
        ] {
            assert_eq!(
                verification_result_from_tag(verification_result_tag(result)),
                Some(result)
            );
        }

        assert_eq!(dtype_from_tag(0), None);
        assert_eq!(primitive_type_from_tag(0), None);
        assert_eq!(verification_result_from_tag(0), None);
    }

    #[test]
    fn fixed_block_payloads_roundtrip_and_reject_wrong_lengths() {
        let block = TensorBlock {
            height: 11,
            parent_hash: hash(1),
            epoch: 2,
            proposer: hash(3),
            settled_receipt_set_root: hash(4),
            checks_root: hash(5),
            attestation_root: hash(6),
            state_root: hash(7),
            reward_root: hash(8),
            beacon: hash(9),
            difficulty_target: hash(10),
            nonce: 12,
            timestamp: 13,
            proposer_signature: hash(14),
            validator_signature_aggregate: hash(15),
        };
        let mut payload = encode_tensor_block_payload(&block);
        assert_eq!(payload.len(), TENSOR_BLOCK_PAYLOAD_LEN);
        assert_eq!(decode_tensor_block_payload(&payload), Some(block));
        assert_eq!(
            decode_tensor_block_payload(&payload[..payload.len() - 1]),
            None
        );
        payload.push(0);
        assert_eq!(decode_tensor_block_payload(&payload), None);

        let vote = BlockVote {
            validator: hash(16),
            block_hash: hash(17),
            block_height: 18,
            stake: 19,
            signature: hash(20),
        };
        let mut payload = encode_block_vote_payload(&vote);
        assert_eq!(payload.len(), BLOCK_VOTE_PAYLOAD_LEN);
        assert_eq!(decode_block_vote_payload(&payload), Some(vote));
        assert_eq!(
            decode_block_vote_payload(&payload[..payload.len() - 1]),
            None
        );
        payload.push(0);
        assert_eq!(decode_block_vote_payload(&payload), None);
    }

    #[test]
    fn job_payloads_roundtrip_stream_and_reject_malformed_edges() {
        const TENSOR_DTYPE_OFFSET: usize = 1 + 32 + 8 + 8 + 8 + 8;
        const TENSOR_OPTIONAL_MODULUS_OFFSET: usize = TENSOR_DTYPE_OFFSET + 1;
        const LINEAR_INPUT_SHAPE_LEN_OFFSET: usize = 1 + 32 + 32 + 8 + 32 + 32;

        let tensor_job = JobState::TensorOp(MatmulJob {
            job_id: hash(21),
            epoch: 22,
            m: 2,
            k: 3,
            n: 4,
            dtype: DType::FieldElement,
            modulus: Some(97),
            seed_a: hash(23),
            seed_b: hash(24),
            deadline_block: 25,
            reward_weight: 26,
        });
        let linear_job = JobState::LinearTrainingStep(LinearTrainingStepJob {
            job_id: hash(27),
            model_id: hash(28),
            step: 29,
            batch_seed: hash(30),
            weight_root_before: hash(31),
            input_shape: vec![2, 3],
            weight_shape: vec![3, 4],
            target_shape: vec![2, 4],
            lr: 5,
            dtype: DType::FieldElement,
            deadline_block: 32,
            reward_weight: 33,
        });

        for job in [tensor_job.clone(), linear_job.clone()] {
            let payload = encode_job_payload(&job);
            assert_eq!(decode_job_payload(&payload, Some(16)), Ok(job.clone()));

            let mut offset = 0;
            assert_eq!(
                decode_job_payload_from(&payload, &mut offset, None),
                Ok(job)
            );
            assert_eq!(offset, payload.len());
        }

        let mut unknown_job_tag = encode_job_payload(&tensor_job);
        unknown_job_tag[0] = 9;
        assert_eq!(
            decode_job_payload(&unknown_job_tag, Some(16)),
            Err(CodecError::UnknownJobTag)
        );

        let mut bad_dtype = encode_job_payload(&tensor_job);
        bad_dtype[TENSOR_DTYPE_OFFSET] = 9;
        assert_eq!(
            decode_job_payload(&bad_dtype, Some(16)),
            Err(CodecError::UnknownDType)
        );

        let mut bad_optional = encode_job_payload(&tensor_job);
        bad_optional[TENSOR_OPTIONAL_MODULUS_OFFSET] = 9;
        assert_eq!(
            decode_job_payload(&bad_optional, Some(16)),
            Err(CodecError::InvalidOptionalU64)
        );

        let mut trailing = encode_job_payload(&tensor_job);
        trailing.push(0);
        assert_eq!(
            decode_job_payload(&trailing, Some(16)),
            Err(CodecError::TrailingBytes)
        );

        let mut oversized_shape = encode_job_payload(&linear_job);
        oversized_shape[LINEAR_INPUT_SHAPE_LEN_OFFSET..LINEAR_INPUT_SHAPE_LEN_OFFSET + 8]
            .copy_from_slice(&17_u64.to_le_bytes());
        assert_eq!(
            decode_job_payload(&oversized_shape, Some(16)),
            Err(CodecError::ShapeVectorTooLarge)
        );

        let truncated = encode_job_payload(&linear_job);
        assert_eq!(
            decode_job_payload(&truncated[..truncated.len() - 1], Some(16)),
            Err(CodecError::Truncated)
        );
    }

    fn hash(byte: u8) -> Hash {
        [byte; 32]
    }
}
