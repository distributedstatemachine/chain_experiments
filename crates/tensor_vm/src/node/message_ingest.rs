use super::{
    NetworkBlockPayloadApply, NetworkEventContext, NetworkEventIngest, NetworkPayloadApply,
    NetworkPayloadError, PendingNetworkPayloads,
    payload_application::{
        apply_network_attestation_payload, apply_network_block_vote_payload,
        apply_network_job_payload, apply_network_receipt_payload,
    },
    payload_processor,
};
use crate::api::P2pMessage;

pub fn ingest_network_messages<C: NetworkEventContext + ?Sized>(
    context: &mut C,
    messages: Vec<P2pMessage>,
    _local_producer: bool,
    pending_payloads: &mut PendingNetworkPayloads,
) -> std::result::Result<NetworkEventIngest, String> {
    let mut ingested = NetworkEventIngest::default();
    for message in network_ingest_order(messages) {
        ingested.events = ingested.events.saturating_add(1);
        match message {
            P2pMessage::NewBlock(block_hash) => {
                ingested.block_announcements = ingested.block_announcements.saturating_add(1);
                if block_hash == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::NewBlockHeader { height, block_hash } => {
                ingested.block_announcements = ingested.block_announcements.saturating_add(1);
                ingested.block_headers = ingested.block_headers.saturating_add(1);
                if height == 0 || block_hash == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    continue;
                }
            }
            P2pMessage::NewBlockPayload {
                height,
                block_hash,
                payload,
            } => {
                ingested.block_announcements = ingested.block_announcements.saturating_add(1);
                ingested.block_payloads = ingested.block_payloads.saturating_add(1);
                if height == 0 || block_hash == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    continue;
                }
                match context.apply_block_payload(height, block_hash, &payload) {
                    NetworkBlockPayloadApply::Applied { appended } => {
                        ingested.block_payloads_applied =
                            ingested.block_payloads_applied.saturating_add(1);
                        ingested.applied_blocks = ingested.applied_blocks.saturating_add(appended);
                    }
                    NetworkBlockPayloadApply::Pending => {
                        pending_payloads.queue_block(height, block_hash, payload);
                    }
                    NetworkBlockPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::NewBlockVotePayload {
                block_hash,
                validator,
                payload,
            } => {
                ingested.block_announcements = ingested.block_announcements.saturating_add(1);
                ingested.block_votes = ingested.block_votes.saturating_add(1);
                if block_hash == [0; 32] || validator == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    continue;
                }
                match apply_network_block_vote_payload(
                    context.chain(),
                    block_hash,
                    validator,
                    &payload,
                ) {
                    NetworkPayloadApply::Applied => {
                        ingested.block_votes_applied =
                            ingested.block_votes_applied.saturating_add(1);
                    }
                    NetworkPayloadApply::Pending => {
                        pending_payloads.queue_block_vote(block_hash, validator, payload);
                    }
                    NetworkPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::NewJob(job_id) => {
                ingested.jobs = ingested.jobs.saturating_add(1);
                if job_id == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::NewJobPayload { job_id, payload } => {
                ingested.jobs = ingested.jobs.saturating_add(1);
                ingested.job_payloads = ingested.job_payloads.saturating_add(1);
                match apply_network_job_payload(context.chain(), job_id, &payload) {
                    Ok(()) => {
                        ingested.job_payloads_applied =
                            ingested.job_payloads_applied.saturating_add(1);
                    }
                    Err(NetworkPayloadError::Invalid) => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::NewReceipt(receipt_id) => {
                ingested.receipts = ingested.receipts.saturating_add(1);
                if receipt_id == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::NewReceiptPayload {
                receipt_id,
                payload,
            } => {
                ingested.receipts = ingested.receipts.saturating_add(1);
                ingested.receipt_payloads = ingested.receipt_payloads.saturating_add(1);
                match apply_network_receipt_payload(context.chain(), receipt_id, &payload) {
                    NetworkPayloadApply::Applied => {
                        ingested.receipt_payloads_applied =
                            ingested.receipt_payloads_applied.saturating_add(1);
                    }
                    NetworkPayloadApply::Pending => {
                        pending_payloads.queue_receipt(receipt_id, payload);
                    }
                    NetworkPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::NewAttestation(attestation_id) => {
                ingested.attestations = ingested.attestations.saturating_add(1);
                if attestation_id == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::NewAttestationPayload {
                attestation_id,
                payload,
            } => {
                ingested.attestations = ingested.attestations.saturating_add(1);
                ingested.attestation_payloads = ingested.attestation_payloads.saturating_add(1);
                match apply_network_attestation_payload(context.chain(), attestation_id, &payload) {
                    NetworkPayloadApply::Applied => {
                        ingested.attestation_payloads_applied =
                            ingested.attestation_payloads_applied.saturating_add(1);
                    }
                    NetworkPayloadApply::Pending => {
                        pending_payloads.queue_attestation(attestation_id, payload);
                    }
                    NetworkPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::PeerInfo { address } => {
                ingested.peers = ingested.peers.saturating_add(1);
                if address == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::RequestTensorChunk { .. }
            | P2pMessage::TensorChunkResponse { .. }
            | P2pMessage::RequestTensorRow { .. }
            | P2pMessage::TensorRowResponse { .. }
            | P2pMessage::RequestTensorByCommitmentRoot { .. }
            | P2pMessage::TensorByCommitmentRootResponse { .. }
            | P2pMessage::RequestProgram(_)
            | P2pMessage::ProgramResponse { .. } => {
                ingested.invalid_events = ingested.invalid_events.saturating_add(1);
            }
        }
    }
    let mut processor = payload_processor::ContextNetworkPayloadProcessor { context };
    ingested.accumulate(pending_payloads.retry_with(&mut processor));
    Ok(ingested)
}

pub fn network_ingest_order(messages: Vec<P2pMessage>) -> Vec<P2pMessage> {
    let mut other_messages = Vec::new();
    let mut block_payloads = Vec::new();
    let mut block_announcements = Vec::new();
    for message in messages {
        if is_block_payload(&message) {
            block_payloads.push(message);
        } else if is_block_announcement(&message) {
            block_announcements.push(message);
        } else {
            other_messages.push(message);
        }
    }
    other_messages.append(&mut block_payloads);
    other_messages.append(&mut block_announcements);
    other_messages
}

fn is_block_announcement(message: &P2pMessage) -> bool {
    matches!(
        message,
        P2pMessage::NewBlock(_) | P2pMessage::NewBlockHeader { .. }
    )
}

fn is_block_payload(message: &P2pMessage) -> bool {
    matches!(message, P2pMessage::NewBlockPayload { .. })
}
