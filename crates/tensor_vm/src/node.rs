use crate::{
    chain::{ChainCommand, ChainEngine, LocalChain},
    p2p::{decode_attestation_payload, decode_job_payload, decode_receipt_payload},
    types::{Hash, hash_bytes},
    verify::ValidatorAttestation,
};
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

pub fn apply_network_job_payload(
    chain: &mut LocalChain,
    job_id: Hash,
    payload: &[u8],
) -> std::result::Result<(), ()> {
    if job_id == [0; 32] {
        return Err(());
    }
    let job = decode_job_payload(payload).map_err(|_| ())?;
    if job.job_id() != job_id {
        return Err(());
    }
    if let Some(existing) = chain.state.jobs.get(&job_id) {
        if existing == &job {
            return Ok(());
        }
        return Err(());
    }
    chain
        .apply_command(ChainCommand::SubmitJob(job))
        .map_err(|_| ())?;
    Ok(())
}

pub fn apply_network_receipt_payload(
    chain: &mut LocalChain,
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
    if let Some(existing) = chain.state.receipts.get(&receipt_id) {
        if existing == &receipt {
            return NetworkPayloadApply::Applied;
        }
        return NetworkPayloadApply::Invalid;
    }
    if !chain.state.jobs.contains_key(&receipt.job_id())
        || !chain.state.miners.contains_key(&receipt.miner())
    {
        return NetworkPayloadApply::Pending;
    }
    chain
        .apply_command(ChainCommand::SubmitReceipt(receipt))
        .map(|_| NetworkPayloadApply::Applied)
        .unwrap_or(NetworkPayloadApply::Invalid)
}

pub fn apply_network_attestation_payload(
    chain: &mut LocalChain,
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
        .state
        .attestations
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
    if !chain.state.validators.contains_key(&attestation.validator)
        || !chain.state.receipts.contains_key(&attestation.receipt_id)
    {
        return NetworkPayloadApply::Pending;
    }
    chain
        .apply_command(ChainCommand::SubmitAttestation(attestation))
        .map(|_| NetworkPayloadApply::Applied)
        .unwrap_or(NetworkPayloadApply::Invalid)
}

pub struct ChainNetworkPayloadProcessor<'a> {
    chain: &'a mut LocalChain,
}

impl<'a> ChainNetworkPayloadProcessor<'a> {
    pub fn new(chain: &'a mut LocalChain) -> Self {
        Self { chain }
    }
}

impl NetworkPayloadProcessor for ChainNetworkPayloadProcessor<'_> {
    fn apply_receipt(&mut self, receipt_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_receipt_payload(self.chain, receipt_id, payload)
    }

    fn apply_attestation(&mut self, attestation_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_attestation_payload(self.chain, attestation_id, payload)
    }
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
    use crate::{
        chain::{JobState, ReceiptState},
        p2p::{encode_attestation_payload, encode_job_payload, encode_receipt_payload},
        scheduler::JobScheduler,
        testnet::{LocalTestnet, TestnetConfig},
    };

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

    fn local_matmul_round(seed_label: &[u8]) -> LocalTestnet {
        let mut testnet = LocalTestnet::new(
            TestnetConfig::default(),
            hash_bytes(b"tensor-vm-node-payload-test", &[seed_label]),
        );
        let scheduler = JobScheduler::with_small_shape((8, 8, 8));
        testnet.run_matmul_round(&scheduler);
        testnet
    }

    #[test]
    fn job_payload_application_validates_submit_duplicates_and_invalid_edges() {
        let testnet = local_matmul_round(b"job");
        let job = testnet
            .chain
            .state
            .jobs
            .values()
            .next()
            .expect("local round must produce a job")
            .clone();
        let job_id = job.job_id();
        let payload = encode_job_payload(&job);
        let mut chain = testnet.chain.clone();
        chain.state.jobs.remove(&job_id);

        assert_eq!(
            apply_network_job_payload(&mut chain, job_id, &payload),
            Ok(())
        );
        assert_eq!(chain.state.jobs.get(&job_id), Some(&job));
        assert_eq!(
            apply_network_job_payload(&mut chain, job_id, &payload),
            Ok(())
        );
        assert_eq!(
            apply_network_job_payload(&mut chain, [0; 32], &payload),
            Err(())
        );
        assert_eq!(
            apply_network_job_payload(&mut chain, hash_bytes(b"test", &[b"wrong-job"]), &payload),
            Err(())
        );
        assert_eq!(
            apply_network_job_payload(&mut chain, job_id, &[1, 2, 3]),
            Err(())
        );

        let mut conflicting = job.clone();
        match &mut conflicting {
            JobState::TensorOp(job) => job.reward_weight = job.reward_weight.saturating_add(1),
            JobState::LinearTrainingStep(job) => {
                job.reward_weight = job.reward_weight.saturating_add(1)
            }
        }
        assert_eq!(
            apply_network_job_payload(&mut chain, job_id, &encode_job_payload(&conflicting)),
            Err(())
        );
    }

    #[test]
    fn receipt_payload_application_reports_pending_applied_and_invalid_edges() {
        let testnet = local_matmul_round(b"receipt");
        let receipt = testnet
            .chain
            .state
            .receipts
            .values()
            .next()
            .expect("local round must produce a receipt")
            .clone();
        let receipt_id = receipt.receipt_id();
        let payload = encode_receipt_payload(&receipt);

        let mut missing_job_chain = testnet.chain.clone();
        missing_job_chain.state.jobs.remove(&receipt.job_id());
        missing_job_chain.state.receipts.remove(&receipt_id);
        assert_eq!(
            apply_network_receipt_payload(&mut missing_job_chain, receipt_id, &payload),
            NetworkPayloadApply::Pending
        );

        let mut apply_chain = testnet.chain.clone();
        apply_chain.state.receipts.remove(&receipt_id);
        apply_chain.state.attestations.remove(&receipt_id);
        assert_eq!(
            apply_network_receipt_payload(&mut apply_chain, receipt_id, &payload),
            NetworkPayloadApply::Applied
        );
        assert_eq!(apply_chain.state.receipts.get(&receipt_id), Some(&receipt));
        assert_eq!(
            apply_network_receipt_payload(&mut testnet.chain.clone(), receipt_id, &payload),
            NetworkPayloadApply::Applied
        );
        assert_eq!(
            apply_network_receipt_payload(&mut apply_chain, [0; 32], &payload),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_receipt_payload(
                &mut apply_chain,
                hash_bytes(b"test", &[b"wrong-receipt"]),
                &payload,
            ),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_receipt_payload(&mut apply_chain, receipt_id, &[1, 2, 3]),
            NetworkPayloadApply::Invalid
        );

        let mut conflicting = receipt.clone();
        match &mut conflicting {
            ReceiptState::TensorOp(receipt) => {
                receipt.execution_time_ms = receipt.execution_time_ms.saturating_add(1)
            }
            ReceiptState::LinearTrainingStep(receipt) => {
                receipt.execution_time_ms = receipt.execution_time_ms.saturating_add(1)
            }
        }
        assert_eq!(
            apply_network_receipt_payload(
                &mut testnet.chain.clone(),
                receipt_id,
                &encode_receipt_payload(&conflicting),
            ),
            NetworkPayloadApply::Invalid
        );
    }

    #[test]
    fn attestation_payload_application_reports_pending_applied_and_invalid_edges() {
        let testnet = local_matmul_round(b"attestation");
        let attestation = testnet
            .chain
            .state
            .attestations
            .values()
            .flat_map(|items| items.iter())
            .next()
            .expect("local round must produce an attestation")
            .clone();
        let attestation_id = attestation_announcement_hash(&attestation);
        let payload = encode_attestation_payload(&attestation);

        let mut missing_receipt_chain = testnet.chain.clone();
        missing_receipt_chain
            .state
            .receipts
            .remove(&attestation.receipt_id);
        missing_receipt_chain
            .state
            .attestations
            .remove(&attestation.receipt_id);
        assert_eq!(
            apply_network_attestation_payload(&mut missing_receipt_chain, attestation_id, &payload,),
            NetworkPayloadApply::Pending
        );

        let mut apply_chain = testnet.chain.clone();
        apply_chain
            .state
            .attestations
            .remove(&attestation.receipt_id);
        assert_eq!(
            apply_network_attestation_payload(&mut apply_chain, attestation_id, &payload),
            NetworkPayloadApply::Applied
        );
        assert_eq!(
            apply_chain
                .state
                .attestations
                .get(&attestation.receipt_id)
                .and_then(|items| items.first()),
            Some(&attestation)
        );
        assert_eq!(
            apply_network_attestation_payload(&mut testnet.chain.clone(), attestation_id, &payload,),
            NetworkPayloadApply::Applied
        );
        assert_eq!(
            apply_network_attestation_payload(&mut apply_chain, [0; 32], &payload),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_attestation_payload(
                &mut apply_chain,
                hash_bytes(b"test", &[b"wrong-attestation"]),
                &payload,
            ),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_attestation_payload(&mut apply_chain, attestation_id, &[1, 2, 3]),
            NetworkPayloadApply::Invalid
        );

        let mut conflicting = attestation.clone();
        conflicting.checks_root = hash_bytes(b"test", &[b"conflicting-attestation"]);
        let conflicting_id = attestation_announcement_hash(&conflicting);
        assert_eq!(
            apply_network_attestation_payload(
                &mut testnet.chain.clone(),
                conflicting_id,
                &encode_attestation_payload(&conflicting),
            ),
            NetworkPayloadApply::Invalid
        );
    }

    #[test]
    fn chain_payload_processor_retries_against_chain_state() {
        let testnet = local_matmul_round(b"processor");
        let job = testnet
            .chain
            .state
            .jobs
            .values()
            .next()
            .expect("local round must produce a job")
            .clone();
        let job_id = job.job_id();
        let receipt = testnet
            .chain
            .state
            .receipts
            .values()
            .next()
            .expect("local round must produce a receipt")
            .clone();
        let receipt_id = receipt.receipt_id();
        let attestation = testnet
            .chain
            .state
            .attestations
            .values()
            .flat_map(|items| items.iter())
            .next()
            .expect("local round must produce an attestation")
            .clone();
        let attestation_id = attestation_announcement_hash(&attestation);

        let mut chain = testnet.chain.clone();
        chain.state.jobs.remove(&job_id);
        chain.state.receipts.remove(&receipt_id);
        chain.state.attestations.remove(&receipt_id);
        let mut pending = PendingNetworkPayloads::default();
        pending.queue_receipt(receipt_id, encode_receipt_payload(&receipt));
        pending.queue_attestation(attestation_id, encode_attestation_payload(&attestation));

        apply_network_job_payload(&mut chain, job_id, &encode_job_payload(&job)).unwrap();
        let mut processor = ChainNetworkPayloadProcessor::new(&mut chain);
        let ingested = pending.retry_with(&mut processor);

        assert_eq!(ingested.receipt_payloads_applied, 1);
        assert_eq!(ingested.attestation_payloads_applied, 1);
        assert!(pending.is_empty());
        assert_eq!(chain.state.receipts.get(&receipt_id), Some(&receipt));
        assert_eq!(
            chain
                .state
                .attestations
                .get(&receipt_id)
                .and_then(|items| items.first()),
            Some(&attestation)
        );
    }
}
