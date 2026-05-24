use super::{NetworkBlockPayloadApply, NetworkPayloadApply, NetworkPayloadError};
use crate::{
    chain::{BlockAdmission, Chain, ChainCommand, ChainEngine},
    p2p::{
        decode_attestation_payload, decode_block_payload, decode_block_vote_payload,
        decode_job_payload, decode_receipt_payload,
    },
    types::{Hash, hash_bytes},
    verify::ValidatorAttestation,
};

pub fn apply_network_job_payload(
    chain: &mut Chain,
    job_id: Hash,
    payload: &[u8],
) -> std::result::Result<(), NetworkPayloadError> {
    if job_id == [0; 32] {
        return Err(NetworkPayloadError::Invalid);
    }
    let job = decode_job_payload(payload).map_err(|_| NetworkPayloadError::Invalid)?;
    if job.job_id() != job_id {
        return Err(NetworkPayloadError::Invalid);
    }
    if let Some(existing) = chain.state().jobs().get(&job_id) {
        if existing == &job {
            return Ok(());
        }
        return Err(NetworkPayloadError::Invalid);
    }
    chain
        .apply_command(ChainCommand::SubmitJob(job))
        .map_err(|_| NetworkPayloadError::Invalid)?;
    Ok(())
}

pub fn apply_network_block_payload(
    chain: &mut Chain,
    height: u64,
    block_hash: Hash,
    payload: &[u8],
) -> NetworkBlockPayloadApply {
    if height == 0 || block_hash == [0; 32] {
        return NetworkBlockPayloadApply::Invalid;
    }
    let Ok(block) = decode_block_payload(payload) else {
        return NetworkBlockPayloadApply::Invalid;
    };
    if block.height != height || block.hash() != block_hash {
        return NetworkBlockPayloadApply::Invalid;
    }
    if chain
        .blocks
        .iter()
        .any(|existing| existing.hash() == block_hash)
    {
        return NetworkBlockPayloadApply::Applied { appended: 0 };
    }
    if height > chain.state().height() {
        return NetworkBlockPayloadApply::Pending;
    }
    if height < chain.state().height() {
        return NetworkBlockPayloadApply::Invalid;
    }
    let expected_parent = chain
        .blocks
        .last()
        .map(crate::chain::TensorBlock::hash)
        .unwrap_or([0; 32]);
    if block.parent_hash != expected_parent {
        return NetworkBlockPayloadApply::Pending;
    }

    let mut candidate = chain.clone();
    if candidate.prepare_block_parent_state().is_err() {
        return NetworkBlockPayloadApply::Invalid;
    }
    match candidate.admit_block(block) {
        Ok(BlockAdmission::Applied { .. }) => {
            *chain = candidate;
            NetworkBlockPayloadApply::Applied { appended: 1 }
        }
        Ok(BlockAdmission::Duplicate { .. }) => NetworkBlockPayloadApply::Applied { appended: 0 },
        Ok(BlockAdmission::PendingParent { .. }) => NetworkBlockPayloadApply::Pending,
        Ok(BlockAdmission::Invalid { .. }) | Err(_) => NetworkBlockPayloadApply::Invalid,
    }
}

pub fn apply_network_block_vote_payload(
    chain: &mut Chain,
    block_hash: Hash,
    validator: Hash,
    payload: &[u8],
) -> NetworkPayloadApply {
    if block_hash == [0; 32] || validator == [0; 32] {
        return NetworkPayloadApply::Invalid;
    }
    let Ok(vote) = decode_block_vote_payload(payload) else {
        return NetworkPayloadApply::Invalid;
    };
    if vote.block_hash != block_hash || vote.validator != validator {
        return NetworkPayloadApply::Invalid;
    }
    if let Some(existing) = chain
        .state()
        .block_votes()
        .get(&block_hash)
        .and_then(|votes| {
            votes
                .iter()
                .find(|existing| existing.validator == validator)
        })
    {
        return if existing == &vote {
            NetworkPayloadApply::Applied
        } else {
            NetworkPayloadApply::Invalid
        };
    }
    if !chain
        .blocks
        .iter()
        .any(|block| block.height == vote.block_height && block.hash() == block_hash)
    {
        return NetworkPayloadApply::Pending;
    }
    chain
        .apply_command(ChainCommand::SubmitBlockVote(vote))
        .map(|_| NetworkPayloadApply::Applied)
        .unwrap_or(NetworkPayloadApply::Invalid)
}

pub fn apply_network_receipt_payload(
    chain: &mut Chain,
    receipt_id: Hash,
    payload: &[u8],
) -> NetworkPayloadApply {
    if receipt_id == [0; 32] {
        return NetworkPayloadApply::Invalid;
    }
    let Ok(receipt) = decode_receipt_payload(payload) else {
        return NetworkPayloadApply::Invalid;
    };
    if receipt.receipt_id() != receipt_id {
        return NetworkPayloadApply::Invalid;
    }
    if let Some(existing) = chain.state().receipts().get(&receipt_id) {
        if existing == &receipt {
            return NetworkPayloadApply::Applied;
        }
        return NetworkPayloadApply::Invalid;
    }
    if !chain.state().jobs().contains_key(&receipt.job_id())
        || !chain.state().miners().contains_key(&receipt.miner())
    {
        return NetworkPayloadApply::Pending;
    }
    chain
        .apply_command(ChainCommand::SubmitReceipt(receipt))
        .map(|_| NetworkPayloadApply::Applied)
        .unwrap_or(NetworkPayloadApply::Invalid)
}

pub fn apply_network_attestation_payload(
    chain: &mut Chain,
    attestation_id: Hash,
    payload: &[u8],
) -> NetworkPayloadApply {
    if attestation_id == [0; 32] {
        return NetworkPayloadApply::Invalid;
    }
    let Ok(attestation) = decode_attestation_payload(payload) else {
        return NetworkPayloadApply::Invalid;
    };
    if attestation_announcement_hash(&attestation) != attestation_id {
        return NetworkPayloadApply::Invalid;
    }
    if let Some(existing) = chain
        .state()
        .attestations()
        .get(&attestation.receipt_id)
        .and_then(|items| {
            items
                .iter()
                .find(|existing| existing.validator == attestation.validator)
        })
    {
        if existing == &attestation {
            return NetworkPayloadApply::Applied;
        }
        return NetworkPayloadApply::Invalid;
    }
    if !chain
        .state()
        .validators()
        .contains_key(&attestation.validator)
        || !chain
            .state()
            .receipts()
            .contains_key(&attestation.receipt_id)
    {
        return NetworkPayloadApply::Pending;
    }
    chain
        .apply_command(ChainCommand::SubmitAttestation(attestation))
        .map(|_| NetworkPayloadApply::Applied)
        .unwrap_or(NetworkPayloadApply::Invalid)
}

pub fn attestation_announcement_hash(attestation: &ValidatorAttestation) -> Hash {
    hash_bytes(
        b"tensor-vm-attestation-announcement-v1",
        &[
            &attestation.validator,
            &attestation.receipt_id,
            &attestation.job_id,
            &attestation.checks_root,
            &attestation.signature,
        ],
    )
}
