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
        chain::{BlockVote, Chain, JobState, ReceiptState},
        p2p::encode_block_payload,
        p2p::{
            encode_attestation_payload, encode_block_vote_payload, encode_job_payload,
            encode_receipt_payload,
        },
        scheduler::JobScheduler,
        testnet::{LocalTestnet, TestnetConfig},
        types::{hash_bytes, sign},
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
    fn job_payload_application_validates_submit_duplicates_and_invalid_edges() {
        let testnet = local_matmul_round(b"job");
        let job = testnet
            .chain
            .state()
            .jobs()
            .values()
            .next()
            .expect("local round must produce a job")
            .clone();
        let job_id = job.job_id();
        let payload = encode_job_payload(&job);
        let mut chain = testnet.chain.clone();
        chain.remove_job_for_testing(&job_id);

        assert_eq!(
            apply_network_job_payload(&mut chain, job_id, &payload),
            Ok(())
        );
        assert_eq!(chain.state().jobs().get(&job_id), Some(&job));
        assert_eq!(
            apply_network_job_payload(&mut chain, job_id, &payload),
            Ok(())
        );
        assert_eq!(
            apply_network_job_payload(&mut chain, [0; 32], &payload),
            Err(NetworkPayloadError::Invalid)
        );
        assert_eq!(
            apply_network_job_payload(&mut chain, hash_bytes(b"test", &[b"wrong-job"]), &payload),
            Err(NetworkPayloadError::Invalid)
        );
        assert_eq!(
            apply_network_job_payload(&mut chain, job_id, &[1, 2, 3]),
            Err(NetworkPayloadError::Invalid)
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
            Err(NetworkPayloadError::Invalid)
        );
    }

    #[test]
    fn block_payload_application_admits_next_head_and_rejects_bad_edges() {
        let seed = hash_bytes(b"test", &[b"network-block-payload"]);
        let validator = hash_bytes(b"test", &[b"network-block-validator"]);
        let mut producer = Chain::new(seed);
        producer.register_validator(validator, 10_000).unwrap();
        producer.produce_block(validator, 1_000).unwrap();
        let mut consumer = producer.clone();
        let parent_chain = consumer.clone();
        let block = producer.produce_block(validator, 1_006).unwrap();
        let block_hash = block.hash();
        let payload = encode_block_payload(&block);

        assert_eq!(
            apply_network_block_payload(&mut consumer, block.height, block_hash, &payload),
            NetworkBlockPayloadApply::Applied { appended: 1 }
        );
        assert_eq!(consumer.blocks, producer.blocks);
        assert!(!consumer.state().finalized_blocks().contains(&block_hash));
        assert!(!consumer.has_block_finality(&block_hash));
        let vote = BlockVote::new(validator, 10_000, &block);
        assert_eq!(
            apply_network_block_vote_payload(
                &mut parent_chain.clone(),
                block_hash,
                vote.validator,
                &encode_block_vote_payload(&vote),
            ),
            NetworkPayloadApply::Pending
        );
        assert_eq!(
            apply_network_block_vote_payload(
                &mut consumer,
                block_hash,
                vote.validator,
                &encode_block_vote_payload(&vote),
            ),
            NetworkPayloadApply::Applied
        );
        assert!(consumer.state().finalized_blocks().contains(&block_hash));
        assert!(consumer.has_block_finality(&block_hash));
        assert_eq!(
            apply_network_block_vote_payload(
                &mut consumer,
                block_hash,
                vote.validator,
                &encode_block_vote_payload(&vote),
            ),
            NetworkPayloadApply::Applied
        );
        let mut conflicting_vote = vote.clone();
        conflicting_vote.signature = [8; 32];
        assert_eq!(
            apply_network_block_vote_payload(
                &mut consumer,
                block_hash,
                conflicting_vote.validator,
                &encode_block_vote_payload(&conflicting_vote),
            ),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_block_payload(&mut consumer, block.height, block_hash, &payload),
            NetworkBlockPayloadApply::Applied { appended: 0 }
        );
        assert_eq!(
            apply_network_block_payload(&mut consumer, block.height, [0; 32], &payload),
            NetworkBlockPayloadApply::Invalid
        );

        let mut bad_signature = block.clone();
        bad_signature.proposer_signature = [9; 32];
        assert_eq!(
            apply_network_block_payload(
                &mut parent_chain.clone(),
                bad_signature.height,
                bad_signature.hash(),
                &encode_block_payload(&bad_signature),
            ),
            NetworkBlockPayloadApply::Invalid
        );

        let mut bad_state_root = block.clone();
        bad_state_root.state_root = hash_bytes(b"test", &[b"wrong-block-state-root"]);
        while !bad_state_root.pow_valid() {
            bad_state_root.nonce = bad_state_root.nonce.saturating_add(1);
        }
        let bad_state_root_hash = bad_state_root.hash();
        bad_state_root.proposer_signature = sign(&bad_state_root.proposer, &bad_state_root_hash);
        bad_state_root.validator_signature_aggregate =
            hash_bytes(b"tensor-vm-validator-aggregate", &[&bad_state_root_hash]);
        assert_eq!(
            apply_network_block_payload(
                &mut parent_chain.clone(),
                bad_state_root.height,
                bad_state_root_hash,
                &encode_block_payload(&bad_state_root),
            ),
            NetworkBlockPayloadApply::Invalid
        );

        let future = producer.produce_block(validator, 1_012).unwrap();
        let future_hash = future.hash();
        assert_eq!(
            apply_network_block_payload(
                &mut Chain::new(seed),
                future.height,
                future_hash,
                &encode_block_payload(&future),
            ),
            NetworkBlockPayloadApply::Pending
        );

        let mut conflicting = block.clone();
        conflicting.timestamp = conflicting.timestamp.saturating_add(1);
        assert_eq!(
            apply_network_block_payload(
                &mut producer.clone(),
                conflicting.height,
                conflicting.hash(),
                &encode_block_payload(&conflicting),
            ),
            NetworkBlockPayloadApply::Invalid
        );
    }

    #[test]
    fn receipt_payload_application_reports_pending_applied_and_invalid_edges() {
        let testnet = local_matmul_round(b"receipt");
        let receipt = testnet
            .chain
            .state()
            .receipts()
            .values()
            .next()
            .expect("local round must produce a receipt")
            .clone();
        let receipt_id = receipt.receipt_id();
        let payload = encode_receipt_payload(&receipt);

        let mut missing_job_chain = testnet.chain.clone();
        missing_job_chain.remove_job_for_testing(&receipt.job_id());
        missing_job_chain.remove_receipt_for_testing(&receipt_id);
        assert_eq!(
            apply_network_receipt_payload(&mut missing_job_chain, receipt_id, &payload),
            NetworkPayloadApply::Pending
        );

        let mut apply_chain = testnet.chain.clone();
        apply_chain.remove_receipt_for_testing(&receipt_id);
        apply_chain.remove_attestations_for_testing(&receipt_id);
        assert_eq!(
            apply_network_receipt_payload(&mut apply_chain, receipt_id, &payload),
            NetworkPayloadApply::Applied
        );
        assert_eq!(
            apply_chain.state().receipts().get(&receipt_id),
            Some(&receipt)
        );
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
            .state()
            .attestations()
            .values()
            .flat_map(|items| items.iter())
            .next()
            .expect("local round must produce an attestation")
            .clone();
        let attestation_id = attestation_announcement_hash(&attestation);
        let payload = encode_attestation_payload(&attestation);

        let mut missing_receipt_chain = testnet.chain.clone();
        missing_receipt_chain.remove_receipt_for_testing(&attestation.receipt_id);
        missing_receipt_chain.remove_attestations_for_testing(&attestation.receipt_id);
        assert_eq!(
            apply_network_attestation_payload(&mut missing_receipt_chain, attestation_id, &payload,),
            NetworkPayloadApply::Pending
        );

        let mut apply_chain = testnet.chain.clone();
        apply_chain.remove_attestations_for_testing(&attestation.receipt_id);
        assert_eq!(
            apply_network_attestation_payload(&mut apply_chain, attestation_id, &payload),
            NetworkPayloadApply::Applied
        );
        assert_eq!(
            apply_chain
                .state()
                .attestations()
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
