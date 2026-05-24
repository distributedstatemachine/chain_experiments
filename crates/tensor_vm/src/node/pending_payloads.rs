use super::{
    NetworkBlockPayloadApply, NetworkEventIngest, NetworkPayloadApply, NetworkPayloadProcessor,
};
use crate::types::Hash;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct PendingNetworkPayloads {
    jobs: BTreeMap<Hash, Vec<u8>>,
    blocks: BTreeMap<(u64, Hash), Vec<u8>>,
    block_votes: BTreeMap<(Hash, Hash), Vec<u8>>,
    receipts: BTreeMap<Hash, Vec<u8>>,
    attestations: BTreeMap<Hash, Vec<u8>>,
}

impl PendingNetworkPayloads {
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
            && self.blocks.is_empty()
            && self.block_votes.is_empty()
            && self.receipts.is_empty()
            && self.attestations.is_empty()
    }

    pub fn pending_job_count(&self) -> usize {
        self.jobs.len()
    }

    pub fn pending_block_count(&self) -> usize {
        self.blocks.len()
    }
    pub fn queue_job(&mut self, job_id: Hash, payload: Vec<u8>) {
        self.jobs.entry(job_id).or_insert(payload);
    }

    pub fn pending_block_vote_count(&self) -> usize {
        self.block_votes.len()
    }

    pub fn pending_receipt_count(&self) -> usize {
        self.receipts.len()
    }

    pub fn pending_attestation_count(&self) -> usize {
        self.attestations.len()
    }

    pub fn queue_receipt(&mut self, receipt_id: Hash, payload: Vec<u8>) {
        self.receipts.entry(receipt_id).or_insert(payload);
    }

    pub fn queue_block(&mut self, height: u64, block_hash: Hash, payload: Vec<u8>) {
        self.blocks.entry((height, block_hash)).or_insert(payload);
    }

    pub fn queue_block_vote(&mut self, block_hash: Hash, validator: Hash, payload: Vec<u8>) {
        self.block_votes
            .entry((block_hash, validator))
            .or_insert(payload);
    }

    pub fn queue_attestation(&mut self, attestation_id: Hash, payload: Vec<u8>) {
        self.attestations.entry(attestation_id).or_insert(payload);
    }

    pub fn retry_with<P: NetworkPayloadProcessor + ?Sized>(
        &mut self,
        processor: &mut P,
    ) -> NetworkEventIngest {
        let mut ingested = NetworkEventIngest::default();
        loop {
            let mut progressed = false;
            for job_id in self.jobs.keys().copied().collect::<Vec<_>>() {
                let payload = self
                    .jobs
                    .get(&job_id)
                    .expect("queued job payload must exist")
                    .clone();
                match processor.apply_job(job_id, &payload) {
                    NetworkPayloadApply::Applied => {
                        self.jobs.remove(&job_id);
                        ingested.job_payloads_applied =
                            ingested.job_payloads_applied.saturating_add(1);
                        progressed = true;
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        self.jobs.remove(&job_id);
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                        progressed = true;
                    }
                }
            }
            for receipt_id in self.receipts.keys().copied().collect::<Vec<_>>() {
                let payload = self
                    .receipts
                    .get(&receipt_id)
                    .expect("queued receipt payload must exist")
                    .clone();
                match processor.apply_receipt(receipt_id, &payload) {
                    NetworkPayloadApply::Applied => {
                        self.receipts.remove(&receipt_id);
                        ingested.receipt_payloads_applied =
                            ingested.receipt_payloads_applied.saturating_add(1);
                        progressed = true;
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        self.receipts.remove(&receipt_id);
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                        progressed = true;
                    }
                }
            }
            for attestation_id in self.attestations.keys().copied().collect::<Vec<_>>() {
                let payload = self
                    .attestations
                    .get(&attestation_id)
                    .expect("queued attestation payload must exist")
                    .clone();
                match processor.apply_attestation(attestation_id, &payload) {
                    NetworkPayloadApply::Applied => {
                        self.attestations.remove(&attestation_id);
                        ingested.attestation_payloads_applied =
                            ingested.attestation_payloads_applied.saturating_add(1);
                        progressed = true;
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        self.attestations.remove(&attestation_id);
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                        progressed = true;
                    }
                }
            }
            for (height, block_hash) in self.blocks.keys().copied().collect::<Vec<_>>() {
                let payload = self
                    .blocks
                    .get(&(height, block_hash))
                    .expect("queued block payload must exist")
                    .clone();
                match processor.apply_block(height, block_hash, &payload) {
                    NetworkBlockPayloadApply::Applied { appended } => {
                        self.blocks.remove(&(height, block_hash));
                        ingested.block_payloads_applied =
                            ingested.block_payloads_applied.saturating_add(1);
                        ingested.applied_blocks = ingested.applied_blocks.saturating_add(appended);
                        progressed = true;
                    }
                    NetworkBlockPayloadApply::Pending => {}
                    NetworkBlockPayloadApply::Invalid => {
                        self.blocks.remove(&(height, block_hash));
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                        progressed = true;
                    }
                }
            }
            for (block_hash, validator) in self.block_votes.keys().copied().collect::<Vec<_>>() {
                let payload = self
                    .block_votes
                    .get(&(block_hash, validator))
                    .expect("queued block vote payload must exist")
                    .clone();
                match processor.apply_block_vote(block_hash, validator, &payload) {
                    NetworkPayloadApply::Applied => {
                        self.block_votes.remove(&(block_hash, validator));
                        ingested.block_votes_applied =
                            ingested.block_votes_applied.saturating_add(1);
                        progressed = true;
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        self.block_votes.remove(&(block_hash, validator));
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                        progressed = true;
                    }
                }
            }
            if !progressed {
                break;
            }
        }
        ingested
    }
}
