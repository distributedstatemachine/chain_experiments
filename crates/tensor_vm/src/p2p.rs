mod behaviour;
mod node;
mod peer_book;
mod request_response;
mod service;
mod service_events;
mod wire;

pub use behaviour::TensorVmNetworkBehaviour;
pub use node::{TensorVmLibp2pNode, build_libp2p_node};
pub use peer_book::{PeerBookStore, PeerRecord};
pub use request_response::P2pRequestResponseBehaviour;
pub use service::{TensorVmLibp2pService, TensorVmLibp2pServiceInfo, spawn_libp2p_service};
pub use wire::{
    decode_attestation_payload, decode_block_payload, decode_block_vote_payload,
    decode_job_payload, decode_message, decode_receipt_payload, decode_tensor_payload,
    encode_attestation_payload, encode_block_payload, encode_block_vote_payload,
    encode_gossipsub_message, encode_job_payload, encode_message, encode_receipt_payload,
    encode_tensor_payload, gossip_topic_for_message, gossipsub_ident_topic,
    request_response_protocol_for_message, request_response_stream_protocol,
};

pub const LIBP2P_PROTOCOL_PREFIX: &str = "/tensorchain/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkStackRecommendation {
    pub libp2p_required: bool,
    pub consensus_transport: &'static str,
    pub tensor_fetch_transport: &'static str,
    pub rationale: Vec<&'static str>,
}

pub fn recommended_network_stack() -> NetworkStackRecommendation {
    NetworkStackRecommendation {
        libp2p_required: true,
        consensus_transport: "rust-libp2p gossipsub/identify/kademlia",
        tensor_fetch_transport: "rust-libp2p request-response",
        rationale: vec![
            "rust-libp2p is the mandatory TensorVM P2P runtime dependency",
            "gossipsub carries block, job, receipt, attestation, and peer announcements",
            "identify advertises TensorVM protocol support to connected peers",
            "request-response streams carry tensor roots, rows, chunks, and program fetches",
            "the TensorVM MVP uses libp2p for both consensus propagation and bounded tensor/program fetches",
        ],
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GossipTopic {
    Blocks,
    Jobs,
    Receipts,
    Attestations,
    Peers,
}

impl GossipTopic {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocks => "/tensorchain/1/blocks",
            Self::Jobs => "/tensorchain/1/jobs",
            Self::Receipts => "/tensorchain/1/receipts",
            Self::Attestations => "/tensorchain/1/attestations",
            Self::Peers => "/tensorchain/1/peers",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RequestResponseProtocol {
    TensorChunk,
    TensorRow,
    TensorByRoot,
    Program,
}

impl RequestResponseProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TensorChunk => "/tensorchain/1/tensor/chunk",
            Self::TensorRow => "/tensorchain/1/tensor/row",
            Self::TensorByRoot => "/tensorchain/1/tensor/by-root",
            Self::Program => "/tensorchain/1/program",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Libp2pControlPlaneConfig {
    pub gossipsub_topics: Vec<GossipTopic>,
    pub request_response_protocols: Vec<RequestResponseProtocol>,
    pub listen_addresses: Vec<String>,
    pub bootstrap_addresses: Vec<String>,
    pub identity_seed: Option<[u8; 32]>,
    pub max_gossipsub_transmit_bytes: usize,
    pub request_timeout_seconds: u64,
    pub max_concurrent_request_streams: usize,
    pub idle_connection_timeout_seconds: u64,
}

impl Default for Libp2pControlPlaneConfig {
    fn default() -> Self {
        Self {
            gossipsub_topics: vec![
                GossipTopic::Blocks,
                GossipTopic::Jobs,
                GossipTopic::Receipts,
                GossipTopic::Attestations,
                GossipTopic::Peers,
            ],
            request_response_protocols: vec![
                RequestResponseProtocol::TensorChunk,
                RequestResponseProtocol::TensorRow,
                RequestResponseProtocol::TensorByRoot,
                RequestResponseProtocol::Program,
            ],
            listen_addresses: Vec::new(),
            bootstrap_addresses: Vec::new(),
            identity_seed: None,
            max_gossipsub_transmit_bytes: 1024 * 1024,
            request_timeout_seconds: 10,
            max_concurrent_request_streams: 128,
            idle_connection_timeout_seconds: 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::P2pMessage;
    use crate::chain::{BlockVote, TensorBlock};
    use crate::error::TvmError;
    use crate::tensor::{DType, Tensor};
    use crate::types::{Hash, address, hash_bytes};
    use std::time::{Duration, Instant};

    #[test]
    fn libp2p_service_spawns_background_runtime() {
        let service = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();

        assert!(!service.peer_id().to_string().is_empty());
        assert_eq!(service.info().identify_protocol, "/tensorchain/1/identify");
        assert_eq!(service.info().subscribed_topics.len(), 5);
        assert_eq!(service.info().request_response_protocols.len(), 4);
        std::thread::sleep(Duration::from_millis(150));
    }

    #[test]
    fn libp2p_service_reports_connected_peer_count() {
        let port = free_tcp_port();
        let service_a = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{port}")],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-connected-a"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        let bootstrap_address = format!("/ip4/127.0.0.1/tcp/{port}/p2p/{}", service_a.peer_id());
        let service_b = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            bootstrap_addresses: vec![bootstrap_address],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-connected-b"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();

        wait_for_connected_services(&service_a, &service_b);
    }

    #[test]
    fn libp2p_service_fetches_tensor_by_commitment_root() {
        let port = free_tcp_port();
        let tensor =
            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![11, 13, 17, 19]).unwrap();
        let commitment_root = tensor.commitment_root();
        let service_a = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{port}")],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-fetch-a"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        service_a.register_tensor(tensor.clone());
        let bootstrap_address = format!("/ip4/127.0.0.1/tcp/{port}/p2p/{}", service_a.peer_id());
        let service_b = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            bootstrap_addresses: vec![bootstrap_address],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-fetch-b"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();

        wait_for_connected_services(&service_a, &service_b);
        assert!(
            service_b
                .connected_peer_ids()
                .contains(&service_a.peer_id())
        );
        let response = service_b
            .request_response(
                service_a.peer_id(),
                P2pMessage::RequestTensorByCommitmentRoot { commitment_root },
                Duration::from_secs(5),
            )
            .unwrap();
        let P2pMessage::TensorByCommitmentRootResponse {
            commitment_root: response_root,
            payload: Some(payload),
        } = response
        else {
            panic!("expected tensor-by-root response");
        };
        assert_eq!(response_root, commitment_root);
        assert_eq!(decode_tensor_payload(&payload).unwrap(), tensor);

        let missing_root = hash_bytes(b"test", &[b"missing-tensor-root"]);
        let response = service_b
            .request_response(
                service_a.peer_id(),
                P2pMessage::RequestTensorByCommitmentRoot {
                    commitment_root: missing_root,
                },
                Duration::from_secs(5),
            )
            .unwrap();
        assert_eq!(
            response,
            P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: missing_root,
                payload: None,
            }
        );
    }

    #[test]
    fn libp2p_service_redials_bootstrap_peer_after_restart() {
        let port = free_tcp_port();
        let seed_a = hash_bytes(b"test", &[b"libp2p-service-redial-a"]);
        let mut service_a = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{port}")],
            identity_seed: Some(seed_a),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        let bootstrap_address = format!("/ip4/127.0.0.1/tcp/{port}/p2p/{}", service_a.peer_id());
        let service_b = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            bootstrap_addresses: vec![bootstrap_address],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-redial-b"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        wait_for_connected_services(&service_a, &service_b);

        drop(service_a);
        wait_for_peer_count(&service_b, 0);
        service_a = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{port}")],
            identity_seed: Some(seed_a),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();

        wait_for_connected_services(&service_a, &service_b);
    }

    #[test]
    fn libp2p_service_publishes_and_observes_block_gossip() {
        let port = free_tcp_port();
        let service_a = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{port}")],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-gossip-a"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        let bootstrap_address = format!("/ip4/127.0.0.1/tcp/{port}/p2p/{}", service_a.peer_id());
        let service_b = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            bootstrap_addresses: vec![bootstrap_address],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-gossip-b"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        wait_for_connected_services(&service_a, &service_b);

        let block_hash = hash_bytes(b"test", &[b"libp2p-service-observed-block"]);
        wait_for_observed_block(&service_a, &service_b, block_hash);
        let block_header_hash = hash_bytes(b"test", &[b"libp2p-service-observed-block-header"]);
        wait_for_observed_block_header(&service_a, &service_b, 7, block_header_hash);
        wait_for_stale_block_announcements_to_preserve_latest_header(
            &service_a,
            &service_b,
            7,
            block_header_hash,
        );
        let block_payload = wire_test_block(b"libp2p-service-observed-block-payload", 8);
        wait_for_observed_block_payload(&service_a, &service_b, &block_payload);
        let block_vote = BlockVote::new(
            address(b"libp2p-observed-vote-validator"),
            10_000,
            &block_payload,
        );
        wait_for_observed_block_vote(&service_a, &service_b, &block_vote);
        wait_for_observed_consensus_gossip(&service_a, &service_b);
        let observed_messages = service_b.drain_observed_messages();
        assert!(
            observed_messages
                .iter()
                .any(|message| matches!(message, P2pMessage::NewBlock(_)))
        );
        assert!(
            observed_messages
                .iter()
                .any(|message| matches!(message, P2pMessage::NewBlockHeader { height: 7, .. }))
        );
        assert!(
            observed_messages
                .iter()
                .any(|message| matches!(message, P2pMessage::NewBlockPayload { height: 8, .. }))
        );
        assert!(
            observed_messages
                .iter()
                .any(|message| matches!(message, P2pMessage::NewBlockVotePayload { .. }))
        );
        assert!(
            observed_messages
                .iter()
                .any(|message| matches!(message, P2pMessage::NewJob(_)))
        );
        assert!(
            observed_messages
                .iter()
                .any(|message| matches!(message, P2pMessage::NewReceipt(_)))
        );
        assert!(
            observed_messages
                .iter()
                .any(|message| matches!(message, P2pMessage::NewAttestation(_)))
        );
        assert!(service_b.drain_observed_messages().is_empty());
    }

    #[test]
    fn libp2p_service_rejects_request_response_gossip_publish() {
        let service = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-bad-publish"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        let hash = hash_bytes(b"test", &[b"request-response-publish"]);

        assert_eq!(
            service.publish_gossip(P2pMessage::RequestProgram(hash)),
            Err(TvmError::InvalidReceipt(
                "message is not a gossipsub announcement"
            ))
        );
    }

    fn free_tcp_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn wire_test_block(label: &[u8], height: u64) -> TensorBlock {
        TensorBlock {
            height,
            parent_hash: hash_bytes(b"test-block", &[label, b"parent"]),
            epoch: height / 4,
            proposer: hash_bytes(b"test-block", &[label, b"proposer"]),
            settled_receipt_set_root: hash_bytes(b"test-block", &[label, b"settled"]),
            checks_root: hash_bytes(b"test-block", &[label, b"checks"]),
            attestation_root: hash_bytes(b"test-block", &[label, b"attestations"]),
            state_root: hash_bytes(b"test-block", &[label, b"state"]),
            reward_root: hash_bytes(b"test-block", &[label, b"rewards"]),
            beacon: hash_bytes(b"test-block", &[label, b"beacon"]),
            difficulty_target: [0xff; 32],
            nonce: height.saturating_add(1),
            timestamp: height.saturating_mul(6),
            proposer_signature: hash_bytes(b"test-block", &[label, b"proposer-signature"]),
            validator_signature_aggregate: hash_bytes(
                b"test-block",
                &[label, b"validator-signature"],
            ),
        }
    }

    fn wait_for_connected_services(
        service_a: &TensorVmLibp2pService,
        service_b: &TensorVmLibp2pService,
    ) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline
            && (service_a.connected_peer_count() == 0 || service_b.connected_peer_count() == 0)
        {
            std::thread::sleep(Duration::from_millis(50));
        }

        assert_eq!(service_a.connected_peer_count(), 1);
        assert_eq!(service_b.connected_peer_count(), 1);
    }

    fn wait_for_peer_count(service: &TensorVmLibp2pService, expected_count: usize) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && service.connected_peer_count() != expected_count {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(service.connected_peer_count(), expected_count);
    }

    fn wait_for_observed_block(
        publisher: &TensorVmLibp2pService,
        observer: &TensorVmLibp2pService,
        block_hash: Hash,
    ) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && observer.latest_observed_block_hash() != block_hash {
            publisher
                .publish_gossip(P2pMessage::NewBlock(block_hash))
                .unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }

        assert!(observer.observed_block_gossip_count() > 0);
        assert_eq!(observer.latest_observed_block_hash(), block_hash);
        assert!(observer.observed_block_hashes().contains(&block_hash));
    }

    fn wait_for_observed_block_header(
        publisher: &TensorVmLibp2pService,
        observer: &TensorVmLibp2pService,
        height: u64,
        block_hash: Hash,
    ) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline
            && (observer.latest_observed_block_height() != height
                || observer.latest_observed_block_hash() != block_hash)
        {
            publisher
                .publish_gossip(P2pMessage::NewBlockHeader { height, block_hash })
                .unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }

        assert!(observer.observed_block_gossip_count() > 1);
        assert_eq!(observer.latest_observed_block_height(), height);
        assert_eq!(observer.latest_observed_block_hash(), block_hash);
        assert!(observer.observed_block_hashes().contains(&block_hash));
    }

    fn wait_for_observed_block_payload(
        publisher: &TensorVmLibp2pService,
        observer: &TensorVmLibp2pService,
        block: &TensorBlock,
    ) {
        let block_hash = block.hash();
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline
            && (observer.latest_observed_block_payload_height() != block.height
                || observer.latest_observed_block_payload_hash() != block_hash)
        {
            publisher
                .publish_gossip(P2pMessage::NewBlockPayload {
                    height: block.height,
                    block_hash,
                    payload: encode_block_payload(block),
                })
                .unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }

        assert!(observer.observed_block_payload_gossip_count() > 0);
        assert_eq!(
            observer.latest_observed_block_payload_height(),
            block.height
        );
        assert_eq!(observer.latest_observed_block_payload_hash(), block_hash);
        assert!(
            observer
                .observed_block_payload_hashes()
                .contains(&block_hash)
        );
    }

    fn wait_for_observed_block_vote(
        publisher: &TensorVmLibp2pService,
        observer: &TensorVmLibp2pService,
        vote: &BlockVote,
    ) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && observer.observed_block_vote_gossip_count() == 0 {
            publisher
                .publish_gossip(P2pMessage::NewBlockVotePayload {
                    block_hash: vote.block_hash,
                    validator: vote.validator,
                    payload: encode_block_vote_payload(vote),
                })
                .unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }

        assert!(observer.observed_block_vote_gossip_count() > 0);
    }

    fn wait_for_stale_block_announcements_to_preserve_latest_header(
        publisher: &TensorVmLibp2pService,
        observer: &TensorVmLibp2pService,
        latest_height: u64,
        latest_hash: Hash,
    ) {
        let stale_header_hash = hash_bytes(b"test", &[b"stale-block-header"]);
        wait_for_observed_hash(
            publisher,
            observer,
            P2pMessage::NewBlockHeader {
                height: latest_height - 1,
                block_hash: stale_header_hash,
            },
            stale_header_hash,
        );
        assert_eq!(observer.latest_observed_block_height(), latest_height);
        assert_eq!(observer.latest_observed_block_hash(), latest_hash);

        let legacy_block_hash = hash_bytes(b"test", &[b"legacy-block-without-height"]);
        wait_for_observed_hash(
            publisher,
            observer,
            P2pMessage::NewBlock(legacy_block_hash),
            legacy_block_hash,
        );
        assert_eq!(observer.latest_observed_block_height(), latest_height);
        assert_eq!(observer.latest_observed_block_hash(), latest_hash);
    }

    fn wait_for_observed_hash(
        publisher: &TensorVmLibp2pService,
        observer: &TensorVmLibp2pService,
        message: P2pMessage,
        block_hash: Hash,
    ) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && !observer.observed_block_hashes().contains(&block_hash) {
            publisher.publish_gossip(message.clone()).unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(observer.observed_block_hashes().contains(&block_hash));
    }

    fn wait_for_observed_consensus_gossip(
        publisher: &TensorVmLibp2pService,
        observer: &TensorVmLibp2pService,
    ) {
        let job_hash = hash_bytes(b"test", &[b"libp2p-service-observed-job"]);
        let receipt_hash = hash_bytes(b"test", &[b"libp2p-service-observed-receipt"]);
        let attestation_hash = hash_bytes(b"test", &[b"libp2p-service-observed-attestation"]);
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline
            && (observer.observed_job_gossip_count() == 0
                || observer.observed_receipt_gossip_count() == 0
                || observer.observed_attestation_gossip_count() == 0)
        {
            publisher
                .publish_gossip(P2pMessage::NewJob(job_hash))
                .unwrap();
            publisher
                .publish_gossip(P2pMessage::NewReceipt(receipt_hash))
                .unwrap();
            publisher
                .publish_gossip(P2pMessage::NewAttestation(attestation_hash))
                .unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }

        assert!(observer.observed_job_gossip_count() > 0);
        assert!(observer.observed_receipt_gossip_count() > 0);
        assert!(observer.observed_attestation_gossip_count() > 0);
    }

    #[test]
    fn libp2p_service_rejects_invalid_runtime_config() {
        let error = match spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["not-a-multiaddr".to_owned()],
            ..Libp2pControlPlaneConfig::default()
        }) {
            Err(error) => error,
            Ok(_) => panic!("invalid libp2p config started"),
        };
        assert_eq!(error, TvmError::InvalidReceipt("invalid libp2p multiaddr"));
    }
}
