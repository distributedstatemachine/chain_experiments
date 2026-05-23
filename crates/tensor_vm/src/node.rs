use crate::types::Hash;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NetworkEventIngest {
    pub events: usize,
    pub block_announcements: usize,
    pub block_headers: usize,
    pub jobs: usize,
    pub job_payloads: usize,
    pub job_payloads_applied: usize,
    pub receipts: usize,
    pub receipt_payloads: usize,
    pub receipt_payloads_applied: usize,
    pub attestations: usize,
    pub attestation_payloads: usize,
    pub attestation_payloads_applied: usize,
    pub peers: usize,
    pub invalid_events: usize,
    pub applied_blocks: usize,
}

impl NetworkEventIngest {
    pub fn has_activity(self) -> bool {
        self.events > 0
            || self.job_payloads_applied > 0
            || self.receipt_payloads_applied > 0
            || self.attestation_payloads_applied > 0
            || self.invalid_events > 0
            || self.applied_blocks > 0
    }

    pub fn accumulate(&mut self, other: Self) {
        self.events = self.events.saturating_add(other.events);
        self.block_announcements = self
            .block_announcements
            .saturating_add(other.block_announcements);
        self.block_headers = self.block_headers.saturating_add(other.block_headers);
        self.jobs = self.jobs.saturating_add(other.jobs);
        self.job_payloads = self.job_payloads.saturating_add(other.job_payloads);
        self.job_payloads_applied = self
            .job_payloads_applied
            .saturating_add(other.job_payloads_applied);
        self.receipts = self.receipts.saturating_add(other.receipts);
        self.receipt_payloads = self.receipt_payloads.saturating_add(other.receipt_payloads);
        self.receipt_payloads_applied = self
            .receipt_payloads_applied
            .saturating_add(other.receipt_payloads_applied);
        self.attestations = self.attestations.saturating_add(other.attestations);
        self.attestation_payloads = self
            .attestation_payloads
            .saturating_add(other.attestation_payloads);
        self.attestation_payloads_applied = self
            .attestation_payloads_applied
            .saturating_add(other.attestation_payloads_applied);
        self.peers = self.peers.saturating_add(other.peers);
        self.invalid_events = self.invalid_events.saturating_add(other.invalid_events);
        self.applied_blocks = self.applied_blocks.saturating_add(other.applied_blocks);
    }
}

#[derive(Debug, Default)]
pub struct NodeRuntimeState {
    served_requests: usize,
    produced_blocks: usize,
    network_applied_blocks: usize,
    network_events: NetworkEventIngest,
    pending_network_payloads: PendingNetworkPayloads,
}

impl NodeRuntimeState {
    pub fn served_requests(&self) -> usize {
        self.served_requests
    }

    pub fn produced_blocks(&self) -> usize {
        self.produced_blocks
    }

    pub fn network_applied_blocks(&self) -> usize {
        self.network_applied_blocks
    }

    pub fn network_events(&self) -> NetworkEventIngest {
        self.network_events
    }

    pub fn pending_payloads(&self) -> &PendingNetworkPayloads {
        &self.pending_network_payloads
    }

    pub fn pending_payloads_mut(&mut self) -> &mut PendingNetworkPayloads {
        &mut self.pending_network_payloads
    }

    pub fn record_served_request(&mut self) {
        self.served_requests = self.served_requests.saturating_add(1);
    }

    pub fn record_produced_block(&mut self) {
        self.produced_blocks = self.produced_blocks.saturating_add(1);
    }

    pub fn record_network_ingest(&mut self, ingested: NetworkEventIngest) {
        self.network_applied_blocks = self
            .network_applied_blocks
            .saturating_add(ingested.applied_blocks);
        self.network_events.accumulate(ingested);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkPayloadApply {
    Applied,
    Pending,
    Invalid,
}

pub trait NetworkPayloadProcessor {
    fn apply_receipt(&mut self, receipt_id: Hash, payload: &[u8]) -> NetworkPayloadApply;

    fn apply_attestation(&mut self, attestation_id: Hash, payload: &[u8]) -> NetworkPayloadApply;
}

#[derive(Debug, Default)]
pub struct PendingNetworkPayloads {
    receipts: BTreeMap<Hash, Vec<u8>>,
    attestations: BTreeMap<Hash, Vec<u8>>,
}

impl PendingNetworkPayloads {
    pub fn is_empty(&self) -> bool {
        self.receipts.is_empty() && self.attestations.is_empty()
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
            if !progressed {
                break;
            }
        }
        ingested
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RetryProcessor {
        receipt_result: NetworkPayloadApply,
        attestation_result: NetworkPayloadApply,
        receipt_attempts: usize,
        attestation_attempts: usize,
    }

    impl NetworkPayloadProcessor for RetryProcessor {
        fn apply_receipt(&mut self, _receipt_id: Hash, _payload: &[u8]) -> NetworkPayloadApply {
            self.receipt_attempts = self.receipt_attempts.saturating_add(1);
            self.receipt_result
        }

        fn apply_attestation(
            &mut self,
            _attestation_id: Hash,
            _payload: &[u8],
        ) -> NetworkPayloadApply {
            self.attestation_attempts = self.attestation_attempts.saturating_add(1);
            self.attestation_result
        }
    }

    impl RetryProcessor {
        fn new(
            receipt_result: NetworkPayloadApply,
            attestation_result: NetworkPayloadApply,
        ) -> Self {
            Self {
                receipt_result,
                attestation_result,
                receipt_attempts: 0,
                attestation_attempts: 0,
            }
        }
    }

    #[test]
    fn runtime_state_tracks_loop_counters() {
        let mut state = NodeRuntimeState::default();
        state.pending_payloads_mut().queue_receipt([9; 32], vec![9]);
        state.record_served_request();
        state.record_produced_block();
        state.record_network_ingest(NetworkEventIngest {
            events: 1,
            applied_blocks: 2,
            ..NetworkEventIngest::default()
        });

        assert_eq!(state.served_requests(), 1);
        assert_eq!(state.produced_blocks(), 1);
        assert_eq!(state.network_applied_blocks(), 2);
        assert_eq!(state.network_events().events, 1);
        assert_eq!(state.pending_payloads().pending_receipt_count(), 1);
        assert_eq!(state.pending_payloads().pending_attestation_count(), 0);
    }

    #[test]
    fn network_event_ingest_activity_checks_each_progress_signal() {
        assert!(!NetworkEventIngest::default().has_activity());
        assert!(
            NetworkEventIngest {
                events: 1,
                ..NetworkEventIngest::default()
            }
            .has_activity()
        );
        assert!(
            NetworkEventIngest {
                job_payloads_applied: 1,
                ..NetworkEventIngest::default()
            }
            .has_activity()
        );
        assert!(
            NetworkEventIngest {
                receipt_payloads_applied: 1,
                ..NetworkEventIngest::default()
            }
            .has_activity()
        );
        assert!(
            NetworkEventIngest {
                attestation_payloads_applied: 1,
                ..NetworkEventIngest::default()
            }
            .has_activity()
        );
        assert!(
            NetworkEventIngest {
                invalid_events: 1,
                ..NetworkEventIngest::default()
            }
            .has_activity()
        );
        assert!(
            NetworkEventIngest {
                applied_blocks: 1,
                ..NetworkEventIngest::default()
            }
            .has_activity()
        );
    }

    #[test]
    fn pending_payloads_retry_applies_and_invalidates_until_quiescent() {
        let receipt_id = [1; 32];
        let attestation_id = [2; 32];
        let mut pending = PendingNetworkPayloads::default();
        pending.queue_receipt(receipt_id, vec![10]);
        pending.queue_attestation(attestation_id, vec![20]);
        let mut processor =
            RetryProcessor::new(NetworkPayloadApply::Applied, NetworkPayloadApply::Invalid);

        let ingested = pending.retry_with(&mut processor);

        assert_eq!(ingested.receipt_payloads_applied, 1);
        assert_eq!(ingested.attestation_payloads_applied, 0);
        assert_eq!(ingested.invalid_events, 1);
        assert!(pending.is_empty());
        assert_eq!(processor.receipt_attempts, 1);
        assert_eq!(processor.attestation_attempts, 1);
    }

    #[test]
    fn pending_payloads_retry_handles_invalid_receipts_and_applied_attestations() {
        let mut pending = PendingNetworkPayloads::default();
        pending.queue_receipt([3; 32], vec![30]);
        pending.queue_attestation([4; 32], vec![40]);
        let mut processor =
            RetryProcessor::new(NetworkPayloadApply::Invalid, NetworkPayloadApply::Applied);

        let ingested = pending.retry_with(&mut processor);

        assert_eq!(ingested.receipt_payloads_applied, 0);
        assert_eq!(ingested.attestation_payloads_applied, 1);
        assert_eq!(ingested.invalid_events, 1);
        assert!(pending.is_empty());
        assert_eq!(processor.receipt_attempts, 1);
        assert_eq!(processor.attestation_attempts, 1);
    }

    #[test]
    fn pending_payloads_retry_keeps_pending_payloads() {
        let mut pending = PendingNetworkPayloads::default();
        pending.queue_receipt([5; 32], vec![50]);
        pending.queue_attestation([6; 32], vec![60]);
        let mut processor =
            RetryProcessor::new(NetworkPayloadApply::Pending, NetworkPayloadApply::Pending);

        let ingested = pending.retry_with(&mut processor);

        assert!(!ingested.has_activity());
        assert_eq!(pending.pending_receipt_count(), 1);
        assert_eq!(pending.pending_attestation_count(), 1);
        assert_eq!(processor.receipt_attempts, 1);
        assert_eq!(processor.attestation_attempts, 1);
    }

    #[test]
    fn pending_payloads_keep_first_payload_for_duplicate_ids() {
        struct PayloadCapturingProcessor {
            receipt_payloads: Vec<Vec<u8>>,
            attestation_payloads: Vec<Vec<u8>>,
        }

        impl NetworkPayloadProcessor for PayloadCapturingProcessor {
            fn apply_receipt(&mut self, _receipt_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
                self.receipt_payloads.push(payload.to_vec());
                NetworkPayloadApply::Applied
            }

            fn apply_attestation(
                &mut self,
                _attestation_id: Hash,
                payload: &[u8],
            ) -> NetworkPayloadApply {
                self.attestation_payloads.push(payload.to_vec());
                NetworkPayloadApply::Applied
            }
        }

        let mut pending = PendingNetworkPayloads::default();
        pending.queue_receipt([7; 32], vec![70]);
        pending.queue_receipt([7; 32], vec![71]);
        pending.queue_attestation([8; 32], vec![80]);
        pending.queue_attestation([8; 32], vec![81]);
        let mut processor = PayloadCapturingProcessor {
            receipt_payloads: Vec::new(),
            attestation_payloads: Vec::new(),
        };

        let ingested = pending.retry_with(&mut processor);

        assert_eq!(ingested.receipt_payloads_applied, 1);
        assert_eq!(ingested.attestation_payloads_applied, 1);
        assert_eq!(processor.receipt_payloads, vec![vec![70]]);
        assert_eq!(processor.attestation_payloads, vec![vec![80]]);
        assert!(pending.is_empty());
    }
}
