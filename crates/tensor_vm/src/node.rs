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
        api::P2pMessage,
        chain::{BlockVote, Chain, JobState, ReceiptState},
        p2p::encode_block_payload,
        p2p::{
            encode_attestation_payload, encode_block_vote_payload, encode_job_payload,
            encode_receipt_payload,
        },
        scheduler::JobScheduler,
        testnet::{LocalTestnet, TestnetConfig},
        types::{Hash, hash_bytes, sign},
    };

    struct RetryProcessor {
        block_result: NetworkBlockPayloadApply,
        receipt_result: NetworkPayloadApply,
        attestation_result: NetworkPayloadApply,
        block_attempts: usize,
        receipt_attempts: usize,
        attestation_attempts: usize,
    }

    impl NetworkPayloadProcessor for RetryProcessor {
        fn apply_job(&mut self, _job_id: Hash, _payload: &[u8]) -> NetworkPayloadApply {
            NetworkPayloadApply::Applied
        }

        fn apply_block(
            &mut self,
            _height: u64,
            _block_hash: Hash,
            _payload: &[u8],
        ) -> NetworkBlockPayloadApply {
            self.block_attempts = self.block_attempts.saturating_add(1);
            self.block_result
        }

        fn apply_block_vote(
            &mut self,
            _block_hash: Hash,
            _validator: Hash,
            _payload: &[u8],
        ) -> NetworkPayloadApply {
            NetworkPayloadApply::Pending
        }

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
                block_result: NetworkBlockPayloadApply::Pending,
                receipt_result,
                attestation_result,
                block_attempts: 0,
                receipt_attempts: 0,
                attestation_attempts: 0,
            }
        }
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
            block_payloads: Vec<Vec<u8>>,
            receipt_payloads: Vec<Vec<u8>>,
            attestation_payloads: Vec<Vec<u8>>,
        }

        impl NetworkPayloadProcessor for PayloadCapturingProcessor {
            fn apply_job(&mut self, _job_id: Hash, _payload: &[u8]) -> NetworkPayloadApply {
                NetworkPayloadApply::Applied
            }

            fn apply_block(
                &mut self,
                _height: u64,
                _block_hash: Hash,
                payload: &[u8],
            ) -> NetworkBlockPayloadApply {
                self.block_payloads.push(payload.to_vec());
                NetworkBlockPayloadApply::Applied { appended: 1 }
            }

            fn apply_block_vote(
                &mut self,
                _block_hash: Hash,
                _validator: Hash,
                _payload: &[u8],
            ) -> NetworkPayloadApply {
                NetworkPayloadApply::Applied
            }

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
            block_payloads: Vec::new(),
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

    struct TestNetworkEventContext {
        chain: Chain,
        applied_payloads: Vec<(u64, Hash)>,
        applied_blocks: usize,
    }

    impl TestNetworkEventContext {
        fn new(seed_label: &[u8]) -> Self {
            Self {
                chain: Chain::new(hash_bytes(
                    b"tensor-vm-node-event-context-test",
                    &[seed_label],
                )),
                applied_payloads: Vec::new(),
                applied_blocks: 2,
            }
        }
    }

    impl NetworkEventContext for TestNetworkEventContext {
        fn chain(&mut self) -> &mut Chain {
            &mut self.chain
        }

        fn apply_block_payload(
            &mut self,
            height: u64,
            block_hash: Hash,
            _payload: &[u8],
        ) -> NetworkBlockPayloadApply {
            self.applied_payloads.push((height, block_hash));
            NetworkBlockPayloadApply::Applied {
                appended: self.applied_blocks,
            }
        }
    }

    #[test]
    fn network_ingest_order_applies_payload_dependencies_before_blocks() {
        let block_hash = hash_bytes(b"test", &[b"announced-block"]);
        let job_id = hash_bytes(b"test", &[b"announced-job"]);
        let receipt_id = hash_bytes(b"test", &[b"announced-receipt"]);
        let messages = network_ingest_order(vec![
            P2pMessage::NewJobPayload {
                job_id,
                payload: vec![1, 2, 3],
            },
            P2pMessage::NewReceipt(receipt_id),
            P2pMessage::NewBlockHeader {
                height: 3,
                block_hash,
            },
            P2pMessage::NewBlockPayload {
                height: 3,
                block_hash,
                payload: vec![4, 5, 6],
            },
            P2pMessage::NewJob(job_id),
            P2pMessage::NewBlock(block_hash),
        ]);

        assert!(matches!(messages[0], P2pMessage::NewJobPayload { .. }));
        assert!(matches!(messages[1], P2pMessage::NewReceipt(_)));
        assert!(matches!(messages[2], P2pMessage::NewJob(_)));
        assert!(matches!(messages[3], P2pMessage::NewBlockPayload { .. }));
        assert!(matches!(messages[4], P2pMessage::NewBlockHeader { .. }));
        assert!(matches!(messages[5], P2pMessage::NewBlock(_)));
    }

    #[test]
    fn network_event_driver_treats_block_headers_as_announcements_only() {
        let block_hash = hash_bytes(b"test", &[b"network-head"]);
        let messages = vec![P2pMessage::NewBlockHeader {
            height: 4,
            block_hash,
        }];
        let mut producer_context = TestNetworkEventContext::new(b"producer");
        let mut pending = PendingNetworkPayloads::default();

        let producer_ingested =
            ingest_network_messages(&mut producer_context, messages.clone(), true, &mut pending)
                .unwrap();

        assert_eq!(producer_ingested.block_headers, 1);
        assert_eq!(producer_ingested.applied_blocks, 0);

        let mut non_producer_context = TestNetworkEventContext::new(b"non-producer");
        let non_producer_ingested = ingest_network_messages(
            &mut non_producer_context,
            messages,
            false,
            &mut PendingNetworkPayloads::default(),
        )
        .unwrap();

        assert_eq!(non_producer_ingested.block_headers, 1);
        assert_eq!(non_producer_ingested.applied_blocks, 0);
    }

    #[test]
    fn network_event_driver_dispatches_block_payloads_for_all_roles() {
        let block_hash = hash_bytes(b"test", &[b"network-payload-head"]);
        let messages = vec![P2pMessage::NewBlockPayload {
            height: 4,
            block_hash,
            payload: vec![7, 8, 9],
        }];
        let mut producer_context = TestNetworkEventContext::new(b"producer-payload");
        let producer_ingested = ingest_network_messages(
            &mut producer_context,
            messages.clone(),
            true,
            &mut PendingNetworkPayloads::default(),
        )
        .unwrap();

        assert_eq!(producer_ingested.block_payloads, 1);
        assert_eq!(producer_ingested.block_payloads_applied, 1);
        assert_eq!(producer_ingested.applied_blocks, 2);
        assert_eq!(producer_context.applied_payloads, vec![(4, block_hash)]);

        let mut non_producer_context = TestNetworkEventContext::new(b"non-producer-payload");
        let non_producer_ingested = ingest_network_messages(
            &mut non_producer_context,
            messages,
            false,
            &mut PendingNetworkPayloads::default(),
        )
        .unwrap();

        assert_eq!(non_producer_ingested.block_payloads, 1);
        assert_eq!(non_producer_ingested.block_payloads_applied, 1);
        assert_eq!(non_producer_ingested.applied_blocks, 2);
        assert_eq!(non_producer_context.applied_payloads, vec![(4, block_hash)]);
    }

    #[test]
    fn network_event_driver_counts_invalid_runtime_messages() {
        let mut context = TestNetworkEventContext::new(b"invalid-events");
        let mut pending = PendingNetworkPayloads::default();
        let ingested = ingest_network_messages(
            &mut context,
            vec![
                P2pMessage::NewBlock([0; 32]),
                P2pMessage::NewBlockHeader {
                    height: 0,
                    block_hash: hash_bytes(b"test", &[b"bad-height"]),
                },
                P2pMessage::NewJob([0; 32]),
                P2pMessage::NewReceipt([0; 32]),
                P2pMessage::NewAttestation([0; 32]),
                P2pMessage::PeerInfo { address: [0; 32] },
                P2pMessage::RequestProgram(hash_bytes(b"test", &[b"program"])),
            ],
            false,
            &mut pending,
        )
        .unwrap();

        assert_eq!(ingested.events, 7);
        assert_eq!(ingested.block_announcements, 2);
        assert_eq!(ingested.block_headers, 1);
        assert_eq!(ingested.jobs, 1);
        assert_eq!(ingested.receipts, 1);
        assert_eq!(ingested.attestations, 1);
        assert_eq!(ingested.peers, 1);
        assert_eq!(ingested.invalid_events, 7);
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

    #[test]
    fn network_event_driver_applies_payloads_and_retries_pending_payloads() {
        let testnet = local_matmul_round(b"driver-payloads");
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
        let mut context = TestNetworkEventContext {
            chain: testnet.chain.clone(),
            applied_payloads: Vec::new(),
            applied_blocks: 0,
        };
        context.chain.remove_job_for_testing(&job_id);
        context.chain.remove_receipt_for_testing(&receipt_id);
        context.chain.remove_attestations_for_testing(&receipt_id);
        let mut pending = PendingNetworkPayloads::default();

        let ingested = ingest_network_messages(
            &mut context,
            vec![
                P2pMessage::NewReceiptPayload {
                    receipt_id,
                    payload: encode_receipt_payload(&receipt),
                },
                P2pMessage::NewAttestationPayload {
                    attestation_id,
                    payload: encode_attestation_payload(&attestation),
                },
                P2pMessage::NewJobPayload {
                    job_id,
                    payload: encode_job_payload(&job),
                },
            ],
            false,
            &mut pending,
        )
        .unwrap();

        assert_eq!(ingested.events, 3);
        assert_eq!(ingested.job_payloads_applied, 1);
        assert_eq!(ingested.receipt_payloads_applied, 1);
        assert_eq!(ingested.attestation_payloads_applied, 1);
        assert_eq!(ingested.invalid_events, 0);
        assert!(pending.is_empty());
        assert_eq!(context.chain.state().jobs().get(&job_id), Some(&job));
        assert_eq!(
            context.chain.state().receipts().get(&receipt_id),
            Some(&receipt)
        );
        assert_eq!(
            context
                .chain
                .state()
                .attestations()
                .get(&receipt_id)
                .and_then(|items| items.first()),
            Some(&attestation)
        );
    }

    #[test]
    fn network_event_driver_reports_direct_applied_and_invalid_payload_edges() {
        let testnet = local_matmul_round(b"driver-direct-payloads");
        let job = testnet
            .chain
            .state()
            .jobs()
            .values()
            .next()
            .expect("local round must produce a job")
            .clone();
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
        let mut context = TestNetworkEventContext {
            chain: testnet.chain.clone(),
            applied_payloads: Vec::new(),
            applied_blocks: 0,
        };
        let mut pending = PendingNetworkPayloads::default();

        let ingested = ingest_network_messages(
            &mut context,
            vec![
                P2pMessage::NewReceiptPayload {
                    receipt_id,
                    payload: encode_receipt_payload(&receipt),
                },
                P2pMessage::NewAttestationPayload {
                    attestation_id,
                    payload: encode_attestation_payload(&attestation),
                },
                P2pMessage::NewJobPayload {
                    job_id: job.job_id(),
                    payload: vec![0xff],
                },
                P2pMessage::NewReceiptPayload {
                    receipt_id,
                    payload: vec![0xff],
                },
                P2pMessage::NewAttestationPayload {
                    attestation_id,
                    payload: vec![0xff],
                },
            ],
            false,
            &mut pending,
        )
        .unwrap();

        assert_eq!(ingested.events, 5);
        assert_eq!(ingested.job_payloads, 1);
        assert_eq!(ingested.receipt_payloads, 2);
        assert_eq!(ingested.receipt_payloads_applied, 1);
        assert_eq!(ingested.attestation_payloads, 2);
        assert_eq!(ingested.attestation_payloads_applied, 1);
        assert_eq!(ingested.invalid_events, 3);
        assert!(pending.is_empty());
    }
}
