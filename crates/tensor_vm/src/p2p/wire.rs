use crate::api::P2pMessage;
use crate::chain::{BlockVote, JobState, ReceiptState, TensorBlock};
use crate::codec::{self, CodecError};
use crate::error::{Result as TvmResult, TvmError};
use crate::tensor::{DType, Tensor};
use crate::types::Hash;
use crate::verify::ValidatorAttestation;
use libp2p::StreamProtocol;

use super::{GossipTopic, RequestResponseProtocol};

pub(super) const MAX_JOB_SHAPE_DIMS: usize = 16;
pub(super) const MAX_RECEIPT_HASHES: usize = 16;
pub(super) const MAX_TENSOR_SHAPE_DIMS: usize = 16;
pub(super) const MAX_TENSOR_VALUES: usize = 1_000_000;
const MAX_WIRE_BYTES: usize = 16 * 1024 * 1024;
const BLOCK_PAYLOAD_LEN: usize = codec::TENSOR_BLOCK_PAYLOAD_LEN;
const BLOCK_VOTE_PAYLOAD_LEN: usize = codec::BLOCK_VOTE_PAYLOAD_LEN;

pub fn gossip_topic_for_message(message: &P2pMessage) -> Option<GossipTopic> {
    match message {
        P2pMessage::NewBlock(_)
        | P2pMessage::NewBlockHeader { .. }
        | P2pMessage::NewBlockPayload { .. }
        | P2pMessage::NewBlockVotePayload { .. } => Some(GossipTopic::Blocks),
        P2pMessage::NewJob(_) | P2pMessage::NewJobPayload { .. } => Some(GossipTopic::Jobs),
        P2pMessage::NewReceipt(_) | P2pMessage::NewReceiptPayload { .. } => {
            Some(GossipTopic::Receipts)
        }
        P2pMessage::NewAttestation(_) | P2pMessage::NewAttestationPayload { .. } => {
            Some(GossipTopic::Attestations)
        }
        P2pMessage::PeerInfo { .. } => Some(GossipTopic::Peers),
        P2pMessage::RequestTensorChunk { .. }
        | P2pMessage::TensorChunkResponse { .. }
        | P2pMessage::RequestTensorRow { .. }
        | P2pMessage::TensorRowResponse { .. }
        | P2pMessage::RequestTensorByCommitmentRoot { .. }
        | P2pMessage::TensorByCommitmentRootResponse { .. }
        | P2pMessage::RequestProgram(_)
        | P2pMessage::ProgramResponse { .. } => None,
    }
}

pub fn request_response_protocol_for_message(
    message: &P2pMessage,
) -> Option<RequestResponseProtocol> {
    match message {
        P2pMessage::RequestTensorChunk { .. } | P2pMessage::TensorChunkResponse { .. } => {
            Some(RequestResponseProtocol::TensorChunk)
        }
        P2pMessage::RequestTensorRow { .. } | P2pMessage::TensorRowResponse { .. } => {
            Some(RequestResponseProtocol::TensorRow)
        }
        P2pMessage::RequestTensorByCommitmentRoot { .. }
        | P2pMessage::TensorByCommitmentRootResponse { .. } => {
            Some(RequestResponseProtocol::TensorByRoot)
        }
        P2pMessage::RequestProgram(_) | P2pMessage::ProgramResponse { .. } => {
            Some(RequestResponseProtocol::Program)
        }
        P2pMessage::NewBlock(_)
        | P2pMessage::NewBlockHeader { .. }
        | P2pMessage::NewBlockPayload { .. }
        | P2pMessage::NewBlockVotePayload { .. }
        | P2pMessage::NewJob(_)
        | P2pMessage::NewJobPayload { .. }
        | P2pMessage::NewReceipt(_)
        | P2pMessage::NewReceiptPayload { .. }
        | P2pMessage::NewAttestation(_)
        | P2pMessage::NewAttestationPayload { .. }
        | P2pMessage::PeerInfo { .. } => None,
    }
}

pub(super) fn is_request_response_request(message: &P2pMessage) -> bool {
    matches!(
        message,
        P2pMessage::RequestTensorChunk { .. }
            | P2pMessage::RequestTensorRow { .. }
            | P2pMessage::RequestTensorByCommitmentRoot { .. }
            | P2pMessage::RequestProgram(_)
    )
}

pub fn gossipsub_ident_topic(topic: GossipTopic) -> libp2p::gossipsub::IdentTopic {
    libp2p::gossipsub::IdentTopic::new(topic.as_str())
}

pub fn request_response_stream_protocol(
    protocol: RequestResponseProtocol,
) -> TvmResult<StreamProtocol> {
    StreamProtocol::try_from_owned(protocol.as_str().to_owned())
        .map_err(|_| TvmError::InvalidReceipt("invalid libp2p stream protocol"))
}

pub fn encode_gossipsub_message(
    message: &P2pMessage,
) -> TvmResult<(libp2p::gossipsub::IdentTopic, Vec<u8>)> {
    let topic = gossip_topic_for_message(message).ok_or(TvmError::InvalidReceipt(
        "message is not a gossipsub announcement",
    ))?;
    Ok((gossipsub_ident_topic(topic), encode_message(message)))
}

pub fn encode_message(message: &P2pMessage) -> Vec<u8> {
    let mut out = Vec::new();
    match message {
        P2pMessage::NewBlock(hash) => {
            out.push(1);
            write_hash(&mut out, hash);
        }
        P2pMessage::NewBlockHeader { height, block_hash } => {
            out.push(12);
            write_u64(&mut out, *height);
            write_hash(&mut out, block_hash);
        }
        P2pMessage::NewBlockPayload {
            height,
            block_hash,
            payload,
        } => {
            out.push(18);
            write_u64(&mut out, *height);
            write_hash(&mut out, block_hash);
            write_bytes(&mut out, payload);
        }
        P2pMessage::NewBlockVotePayload {
            block_hash,
            validator,
            payload,
        } => {
            out.push(19);
            write_hash(&mut out, block_hash);
            write_hash(&mut out, validator);
            write_bytes(&mut out, payload);
        }
        P2pMessage::NewJob(hash) => {
            out.push(2);
            write_hash(&mut out, hash);
        }
        P2pMessage::NewJobPayload { job_id, payload } => {
            out.push(13);
            write_hash(&mut out, job_id);
            write_bytes(&mut out, payload);
        }
        P2pMessage::NewReceipt(hash) => {
            out.push(3);
            write_hash(&mut out, hash);
        }
        P2pMessage::NewReceiptPayload {
            receipt_id,
            payload,
        } => {
            out.push(14);
            write_hash(&mut out, receipt_id);
            write_bytes(&mut out, payload);
        }
        P2pMessage::NewAttestation(hash) => {
            out.push(4);
            write_hash(&mut out, hash);
        }
        P2pMessage::NewAttestationPayload {
            attestation_id,
            payload,
        } => {
            out.push(15);
            write_hash(&mut out, attestation_id);
            write_bytes(&mut out, payload);
        }
        P2pMessage::RequestTensorChunk {
            tensor_id,
            chunk_index,
        } => {
            out.push(5);
            write_hash(&mut out, tensor_id);
            write_u64(&mut out, *chunk_index);
        }
        P2pMessage::TensorChunkResponse {
            tensor_id,
            chunk_index,
            bytes,
        } => {
            out.push(6);
            write_hash(&mut out, tensor_id);
            write_u64(&mut out, *chunk_index);
            write_bytes(&mut out, bytes);
        }
        P2pMessage::RequestTensorRow {
            tensor_id,
            row_index,
        } => {
            out.push(7);
            write_hash(&mut out, tensor_id);
            write_u64(&mut out, *row_index);
        }
        P2pMessage::TensorRowResponse {
            tensor_id,
            row_index,
            values,
        } => {
            out.push(8);
            write_hash(&mut out, tensor_id);
            write_u64(&mut out, *row_index);
            write_u64(&mut out, values.len() as u64);
            for value in values {
                write_u64(&mut out, *value);
            }
        }
        P2pMessage::RequestTensorByCommitmentRoot { commitment_root } => {
            out.push(16);
            write_hash(&mut out, commitment_root);
        }
        P2pMessage::TensorByCommitmentRootResponse {
            commitment_root,
            payload,
        } => {
            out.push(17);
            write_hash(&mut out, commitment_root);
            write_optional_bytes(&mut out, payload.as_deref());
        }
        P2pMessage::RequestProgram(hash) => {
            out.push(9);
            write_hash(&mut out, hash);
        }
        P2pMessage::ProgramResponse {
            program_hash,
            bytes,
        } => {
            out.push(10);
            write_hash(&mut out, program_hash);
            write_bytes(&mut out, bytes);
        }
        P2pMessage::PeerInfo { address } => {
            out.push(11);
            write_hash(&mut out, address);
        }
    }
    out
}

pub fn decode_message(input: &[u8]) -> TvmResult<P2pMessage> {
    let mut reader = Reader::new(input);
    let tag = reader.read_u8()?;
    let message = match tag {
        1 => P2pMessage::NewBlock(reader.read_hash()?),
        12 => P2pMessage::NewBlockHeader {
            height: reader.read_u64()?,
            block_hash: reader.read_hash()?,
        },
        18 => {
            let height = reader.read_u64()?;
            let block_hash = reader.read_hash()?;
            let payload = reader.read_bytes_with_max(BLOCK_PAYLOAD_LEN)?;
            let block = decode_block_payload(&payload)?;
            if block.height != height || block.hash() != block_hash {
                return Err(TvmError::InvalidReceipt(
                    "block payload announcement mismatch",
                ));
            }
            P2pMessage::NewBlockPayload {
                height,
                block_hash,
                payload,
            }
        }
        19 => {
            let block_hash = reader.read_hash()?;
            let validator = reader.read_hash()?;
            let payload = reader.read_bytes_with_max(BLOCK_VOTE_PAYLOAD_LEN)?;
            let vote = decode_block_vote_payload(&payload)?;
            if vote.block_hash != block_hash || vote.validator != validator {
                return Err(TvmError::InvalidReceipt(
                    "block vote payload announcement mismatch",
                ));
            }
            P2pMessage::NewBlockVotePayload {
                block_hash,
                validator,
                payload,
            }
        }
        2 => P2pMessage::NewJob(reader.read_hash()?),
        13 => P2pMessage::NewJobPayload {
            job_id: reader.read_hash()?,
            payload: reader.read_bytes()?,
        },
        3 => P2pMessage::NewReceipt(reader.read_hash()?),
        14 => P2pMessage::NewReceiptPayload {
            receipt_id: reader.read_hash()?,
            payload: reader.read_bytes()?,
        },
        4 => P2pMessage::NewAttestation(reader.read_hash()?),
        15 => P2pMessage::NewAttestationPayload {
            attestation_id: reader.read_hash()?,
            payload: reader.read_bytes_with_max(codec::ATTESTATION_PAYLOAD_LEN)?,
        },
        5 => P2pMessage::RequestTensorChunk {
            tensor_id: reader.read_hash()?,
            chunk_index: reader.read_u64()?,
        },
        6 => P2pMessage::TensorChunkResponse {
            tensor_id: reader.read_hash()?,
            chunk_index: reader.read_u64()?,
            bytes: reader.read_bytes()?,
        },
        7 => P2pMessage::RequestTensorRow {
            tensor_id: reader.read_hash()?,
            row_index: reader.read_u64()?,
        },
        8 => {
            let tensor_id = reader.read_hash()?;
            let row_index = reader.read_u64()?;
            let len = usize::try_from(reader.read_u64()?)
                .map_err(|_| TvmError::InvalidReceipt("tensor row length overflow"))?;
            if len > MAX_TENSOR_VALUES {
                return Err(TvmError::InvalidReceipt("tensor row response too large"));
            }
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(reader.read_u64()?);
            }
            P2pMessage::TensorRowResponse {
                tensor_id,
                row_index,
                values,
            }
        }
        16 => P2pMessage::RequestTensorByCommitmentRoot {
            commitment_root: reader.read_hash()?,
        },
        17 => P2pMessage::TensorByCommitmentRootResponse {
            commitment_root: reader.read_hash()?,
            payload: read_optional_bytes(&mut reader)?,
        },
        9 => P2pMessage::RequestProgram(reader.read_hash()?),
        10 => P2pMessage::ProgramResponse {
            program_hash: reader.read_hash()?,
            bytes: reader.read_bytes()?,
        },
        11 => P2pMessage::PeerInfo {
            address: reader.read_hash()?,
        },
        _ => return Err(TvmError::InvalidReceipt("unknown p2p message tag")),
    };
    if !reader.is_done() {
        return Err(TvmError::InvalidReceipt("trailing p2p bytes"));
    }
    Ok(message)
}

pub fn encode_block_payload(block: &TensorBlock) -> Vec<u8> {
    codec::encode_tensor_block_payload(block)
}

pub fn decode_block_payload(input: &[u8]) -> TvmResult<TensorBlock> {
    codec::decode_tensor_block_payload(input)
        .ok_or(TvmError::InvalidReceipt("invalid block payload length"))
}

pub fn encode_block_vote_payload(vote: &BlockVote) -> Vec<u8> {
    codec::encode_block_vote_payload(vote)
}

pub fn decode_block_vote_payload(input: &[u8]) -> TvmResult<BlockVote> {
    codec::decode_block_vote_payload(input).ok_or(TvmError::InvalidReceipt(
        "invalid block vote payload length",
    ))
}

pub fn encode_tensor_payload(tensor: &Tensor) -> Vec<u8> {
    let mut out = Vec::new();
    write_usize_vec(&mut out, tensor.shape());
    out.push(tensor.dtype().tag());
    write_u64(&mut out, tensor.as_slice().len() as u64);
    for value in tensor.as_slice() {
        write_u64(&mut out, *value);
    }
    out
}

pub fn decode_tensor_payload(input: &[u8]) -> TvmResult<Tensor> {
    let mut reader = Reader::new(input);
    let shape = read_usize_vec(&mut reader, MAX_TENSOR_SHAPE_DIMS)?;
    let dtype = dtype_from_tag(reader.read_u8()?)?;
    let len = read_usize(&mut reader)?;
    if len > MAX_TENSOR_VALUES {
        return Err(TvmError::InvalidReceipt("tensor payload too large"));
    }
    let mut values = Vec::with_capacity(len);
    for _ in 0..len {
        values.push(reader.read_u64()?);
    }
    if !reader.is_done() {
        return Err(TvmError::InvalidReceipt("trailing tensor payload bytes"));
    }
    Tensor::from_vec(shape, dtype, values)
}

pub fn encode_job_payload(job: &JobState) -> Vec<u8> {
    codec::encode_job_payload(job)
}

pub fn decode_job_payload(input: &[u8]) -> TvmResult<JobState> {
    codec::decode_job_payload(input, Some(MAX_JOB_SHAPE_DIMS))
        .map_err(|error| p2p_codec_error(error, "trailing job payload bytes"))
}

pub fn encode_receipt_payload(receipt: &ReceiptState) -> Vec<u8> {
    codec::encode_receipt_payload(receipt)
}

pub fn decode_receipt_payload(input: &[u8]) -> TvmResult<ReceiptState> {
    codec::decode_receipt_payload(input, Some(MAX_RECEIPT_HASHES))
        .map_err(|error| p2p_codec_error(error, "trailing receipt payload bytes"))
}

pub fn encode_attestation_payload(attestation: &ValidatorAttestation) -> Vec<u8> {
    codec::encode_attestation_payload(attestation)
}

pub fn decode_attestation_payload(input: &[u8]) -> TvmResult<ValidatorAttestation> {
    codec::decode_attestation_payload(input)
        .map_err(|error| p2p_codec_error(error, "trailing attestation payload bytes"))
}

fn p2p_codec_error(error: CodecError, trailing_error: &'static str) -> TvmError {
    match error {
        CodecError::Truncated => TvmError::InvalidReceipt("short p2p message"),
        CodecError::TrailingBytes => TvmError::InvalidReceipt(trailing_error),
        CodecError::UnknownJobTag => TvmError::InvalidReceipt("unknown job payload tag"),
        CodecError::UnknownReceiptTag => TvmError::InvalidReceipt("unknown receipt payload tag"),
        CodecError::UnknownDType => TvmError::InvalidReceipt("unknown dtype tag"),
        CodecError::UnknownPrimitiveType => TvmError::InvalidReceipt("unknown primitive type tag"),
        CodecError::UnknownVerificationResult => {
            TvmError::InvalidReceipt("unknown verification result tag")
        }
        CodecError::InvalidOptionalU64 => TvmError::InvalidReceipt("invalid optional u64 tag"),
        CodecError::InvalidBool => TvmError::InvalidReceipt("invalid bool tag"),
        CodecError::UsizeOverflow => TvmError::InvalidReceipt("usize overflow"),
        CodecError::ShapeVectorTooLarge => TvmError::InvalidReceipt("shape vector too large"),
        CodecError::HashVectorTooLarge => TvmError::InvalidReceipt("hash vector too large"),
    }
}

pub(super) fn write_hash(out: &mut Vec<u8>, hash: &Hash) {
    out.extend_from_slice(hash);
}

pub(super) fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(super) fn write_usize_vec(out: &mut Vec<u8>, values: &[usize]) {
    write_u64(out, values.len() as u64);
    for value in values {
        write_u64(out, *value as u64);
    }
}

fn write_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    write_u64(out, bytes.len() as u64);
    out.extend_from_slice(bytes);
}

fn write_optional_bytes(out: &mut Vec<u8>, bytes: Option<&[u8]>) {
    match bytes {
        Some(bytes) => {
            out.push(1);
            write_bytes(out, bytes);
        }
        None => out.push(0),
    }
}

struct Reader<'a> {
    input: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    fn read_u8(&mut self) -> TvmResult<u8> {
        let Some(byte) = self.input.get(self.offset).copied() else {
            return Err(TvmError::InvalidReceipt("short p2p message"));
        };
        self.offset += 1;
        Ok(byte)
    }

    fn read_u64(&mut self) -> TvmResult<u64> {
        let bytes = self.read_exact(8)?;
        let mut out = [0_u8; 8];
        out.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(out))
    }

    fn read_hash(&mut self) -> TvmResult<Hash> {
        let bytes = self.read_exact(32)?;
        let mut out = [0_u8; 32];
        out.copy_from_slice(bytes);
        Ok(out)
    }

    fn read_bytes(&mut self) -> TvmResult<Vec<u8>> {
        self.read_bytes_with_max(MAX_WIRE_BYTES)
    }

    fn read_bytes_with_max(&mut self, max_len: usize) -> TvmResult<Vec<u8>> {
        let len = usize::try_from(self.read_u64()?)
            .map_err(|_| TvmError::InvalidReceipt("p2p byte length overflow"))?;
        if len > max_len {
            return Err(TvmError::InvalidReceipt("p2p byte payload too large"));
        }
        Ok(self.read_exact(len)?.to_vec())
    }

    fn read_exact(&mut self, len: usize) -> TvmResult<&'a [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(TvmError::InvalidReceipt("p2p length overflow"))?;
        let Some(bytes) = self.input.get(self.offset..end) else {
            return Err(TvmError::InvalidReceipt("short p2p message"));
        };
        self.offset = end;
        Ok(bytes)
    }

    fn is_done(&self) -> bool {
        self.offset == self.input.len()
    }
}

fn read_optional_bytes(reader: &mut Reader<'_>) -> TvmResult<Option<Vec<u8>>> {
    match reader.read_u8()? {
        0 => Ok(None),
        1 => Ok(Some(reader.read_bytes()?)),
        _ => Err(TvmError::InvalidReceipt("invalid optional bytes tag")),
    }
}

fn read_usize(reader: &mut Reader<'_>) -> TvmResult<usize> {
    usize::try_from(reader.read_u64()?).map_err(|_| TvmError::InvalidReceipt("usize overflow"))
}

fn read_usize_vec(reader: &mut Reader<'_>, max_len: usize) -> TvmResult<Vec<usize>> {
    let len = read_usize(reader)?;
    if len > max_len {
        return Err(TvmError::InvalidReceipt("shape vector too large"));
    }
    let mut values = Vec::with_capacity(len);
    for _ in 0..len {
        values.push(read_usize(reader)?);
    }
    Ok(values)
}

fn dtype_from_tag(tag: u8) -> TvmResult<DType> {
    codec::dtype_from_tag(tag).ok_or(TvmError::InvalidReceipt("unknown dtype tag"))
}
