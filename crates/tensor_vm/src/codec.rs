use crate::chain::{BlockVote, TensorBlock};
use crate::jobs::PrimitiveType;
use crate::tensor::DType;
use crate::types::Hash;
use crate::verify::VerificationResult;

pub(crate) const TENSOR_BLOCK_PAYLOAD_LEN: usize = 8 * 4 + 32 * 11;
pub(crate) const BLOCK_VOTE_PAYLOAD_LEN: usize = 32 * 3 + 8 * 2;

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
        height: read_u64(input, &mut offset)?,
        parent_hash: read_hash(input, &mut offset)?,
        epoch: read_u64(input, &mut offset)?,
        proposer: read_hash(input, &mut offset)?,
        settled_receipt_set_root: read_hash(input, &mut offset)?,
        checks_root: read_hash(input, &mut offset)?,
        attestation_root: read_hash(input, &mut offset)?,
        state_root: read_hash(input, &mut offset)?,
        reward_root: read_hash(input, &mut offset)?,
        beacon: read_hash(input, &mut offset)?,
        difficulty_target: read_hash(input, &mut offset)?,
        nonce: read_u64(input, &mut offset)?,
        timestamp: read_u64(input, &mut offset)?,
        proposer_signature: read_hash(input, &mut offset)?,
        validator_signature_aggregate: read_hash(input, &mut offset)?,
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
        validator: read_hash(input, &mut offset)?,
        block_hash: read_hash(input, &mut offset)?,
        block_height: read_u64(input, &mut offset)?,
        stake: read_u64(input, &mut offset)?,
        signature: read_hash(input, &mut offset)?,
    };
    (offset == input.len()).then_some(vote)
}

fn write_hash(out: &mut Vec<u8>, hash: &Hash) {
    out.extend_from_slice(hash);
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_hash(input: &[u8], offset: &mut usize) -> Option<Hash> {
    let bytes = input.get(*offset..(*offset).checked_add(32)?)?;
    let mut out = [0_u8; 32];
    out.copy_from_slice(bytes);
    *offset += 32;
    Some(out)
}

fn read_u64(input: &[u8], offset: &mut usize) -> Option<u64> {
    let bytes = input.get(*offset..(*offset).checked_add(8)?)?;
    let mut out = [0_u8; 8];
    out.copy_from_slice(bytes);
    *offset += 8;
    Some(u64::from_le_bytes(out))
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

    fn hash(byte: u8) -> Hash {
        [byte; 32]
    }
}
