mod message_ingest;
mod payload_application;
mod payload_processor;
mod pending_payloads;
mod runtime_state;

pub use message_ingest::{ingest_network_messages, network_ingest_order};
pub use payload_application::{
    apply_network_attestation_payload, apply_network_block_payload,
    apply_network_block_vote_payload, apply_network_job_payload, apply_network_receipt_payload,
    attestation_announcement_hash,
};
pub use payload_processor::{
    ChainNetworkPayloadProcessor, NetworkBlockPayloadApply, NetworkEventContext,
    NetworkPayloadApply, NetworkPayloadError, NetworkPayloadProcessor,
};
pub use pending_payloads::PendingNetworkPayloads;
pub use runtime_state::{NetworkEventIngest, NodeRuntimeState};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        p2p::{encode_attestation_payload, encode_job_payload, encode_receipt_payload},
        scheduler::JobScheduler,
        testnet::{LocalTestnet, TestnetConfig},
        types::hash_bytes,
    };

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
    fn chain_payload_processor_retries_against_chain_state() {
        let testnet = local_matmul_round(b"processor");
        let job = testnet
            .chain
            .state()
            .jobs()
            .values()
            .next()
            .expect("local round must produce a job")
            .clone();
        let job_id = job.job_id();
        let receipt = testnet
            .chain
            .state()
            .receipts()
            .values()
            .next()
            .expect("local round must produce a receipt")
            .clone();
        let receipt_id = receipt.receipt_id();
        let attestation = testnet
            .chain
            .state()
            .attestations()
            .values()
            .flat_map(|items| items.iter())
            .next()
            .expect("local round must produce an attestation")
            .clone();
        let attestation_id = attestation_announcement_hash(&attestation);

        let mut chain = testnet.chain.clone();
        chain.remove_job_for_testing(&job_id);
        chain.remove_receipt_for_testing(&receipt_id);
        chain.remove_attestations_for_testing(&receipt_id);
        let mut pending = PendingNetworkPayloads::default();
        pending.queue_receipt(receipt_id, encode_receipt_payload(&receipt));
        pending.queue_attestation(attestation_id, encode_attestation_payload(&attestation));

        apply_network_job_payload(&mut chain, job_id, &encode_job_payload(&job)).unwrap();
        let mut processor = ChainNetworkPayloadProcessor::new(&mut chain);
        let ingested = pending.retry_with(&mut processor);

        assert_eq!(ingested.receipt_payloads_applied, 1);
        assert_eq!(ingested.attestation_payloads_applied, 1);
        assert!(pending.is_empty());
        assert_eq!(chain.state().receipts().get(&receipt_id), Some(&receipt));
        assert_eq!(
            chain
                .state()
                .attestations()
                .get(&receipt_id)
                .and_then(|items| items.first()),
            Some(&attestation)
        );
    }
}
