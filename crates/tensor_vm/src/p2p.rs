mod behaviour;
mod node;
mod peer_book;
mod request_response;
mod service;
mod service_events;
mod wire;

pub use behaviour::TensorVmNetworkBehaviour;
#[cfg(test)]
use behaviour::TensorVmNetworkBehaviourEvent;
pub use node::{TensorVmLibp2pNode, build_libp2p_node};
#[cfg(test)]
use peer_book::bootstrap_peer_address;
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
    use super::request_response::{
        P2pRequestResponseEvent, send_request_for_protocol, send_response_for_protocol,
    };
    use super::*;
    use crate::api::P2pMessage;
    use crate::chain::{BlockVote, TensorBlock};
    use crate::error::TvmError;
    use crate::tensor::{DType, Tensor};
    use crate::types::{Hash, address, hash_bytes};
    use futures::{FutureExt, StreamExt};
    use libp2p::multiaddr::Protocol;
    use libp2p::swarm::SwarmEvent;
    use libp2p::{Multiaddr, PeerId};
    use std::time::{Duration, Instant};

    #[test]
    fn libp2p_node_builds_real_swarm_and_protocol_behaviour() {
        let config = Libp2pControlPlaneConfig::default();
        let node = build_libp2p_node(&config).unwrap();
        assert!(!node.peer_id.to_string().is_empty());
        assert_eq!(node.subscribed_topics.len(), 5);
        assert!(
            node.subscribed_topics
                .contains(&"/tensorchain/1/blocks".to_owned())
        );
        assert_eq!(node.request_response_protocols.len(), 4);
        assert!(
            node.request_response_protocols
                .contains(&"/tensorchain/1/tensor/chunk".to_owned())
        );
        assert!(
            node.request_response_protocols
                .contains(&"/tensorchain/1/tensor/by-root".to_owned())
        );
        assert_eq!(node.identify_protocol, "/tensorchain/1/identify");
    }

    #[test]
    fn libp2p_node_uses_configured_identity_seed() {
        let seed = hash_bytes(b"test", &[b"libp2p-identity-seed"]);
        let peer_a = build_libp2p_node(&Libp2pControlPlaneConfig {
            identity_seed: Some(seed),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap()
        .peer_id;
        let peer_b = build_libp2p_node(&Libp2pControlPlaneConfig {
            identity_seed: Some(seed),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap()
        .peer_id;
        let peer_c = build_libp2p_node(&Libp2pControlPlaneConfig {
            identity_seed: Some(hash_bytes(b"test", &[b"other-libp2p-identity-seed"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap()
        .peer_id;

        assert_eq!(peer_a, peer_b);
        assert_ne!(peer_a, peer_c);
    }

    #[test]
    fn libp2p_node_accepts_listen_and_bootstrap_multiaddrs() {
        let bootstrap_peer = PeerId::random();
        let bootstrap_address = format!("/ip4/127.0.0.1/tcp/4001/p2p/{bootstrap_peer}");
        let (discovered_peer, discovered_address) =
            bootstrap_peer_address(&bootstrap_address.parse().unwrap()).unwrap();
        assert_eq!(discovered_peer, bootstrap_peer);
        assert_eq!(discovered_address.to_string(), "/ip4/127.0.0.1/tcp/4001");
        let plain_address: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        assert_eq!(bootstrap_peer_address(&plain_address), None);
        let config = Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            bootstrap_addresses: vec![bootstrap_address],
            ..Libp2pControlPlaneConfig::default()
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap();
        runtime.block_on(async {
            let node = build_libp2p_node(&config).unwrap();
            assert!(!node.peer_id.to_string().is_empty());
        });
    }

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

    #[test]
    fn local_testnet_libp2p_swarms_exchange_gossip_and_request_response() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut producer = build_libp2p_node(&Libp2pControlPlaneConfig {
                listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
                ..Libp2pControlPlaneConfig::default()
            })
            .unwrap();
            let mut consumer = build_libp2p_node(&Libp2pControlPlaneConfig::default()).unwrap();
            let listen_addr = wait_for_listen_addr(&mut producer).await;
            let mut dial_addr = listen_addr;
            dial_addr.push(Protocol::P2p(producer.peer_id));
            consumer.swarm.dial(dial_addr).unwrap();

            wait_for_connection(&mut producer, &mut consumer).await;
            producer
                .swarm
                .behaviour_mut()
                .gossipsub
                .add_explicit_peer(&consumer.peer_id);
            consumer
                .swarm
                .behaviour_mut()
                .gossipsub
                .add_explicit_peer(&producer.peer_id);
            wait_for_gossip_subscriptions(
                &mut producer,
                consumer.peer_id,
                &[
                    GossipTopic::Blocks,
                    GossipTopic::Jobs,
                    GossipTopic::Receipts,
                    GossipTopic::Attestations,
                    GossipTopic::Peers,
                ],
            )
            .await;

            let gossip_messages = [
                P2pMessage::NewBlock(hash_bytes(b"test", &[b"gate-0-libp2p-block"])),
                P2pMessage::NewBlockHeader {
                    height: 3,
                    block_hash: hash_bytes(b"test", &[b"gate-0-libp2p-block-header"]),
                },
                {
                    let block = wire_test_block(b"gate-0-libp2p-block-payload", 4);
                    P2pMessage::NewBlockPayload {
                        height: block.height,
                        block_hash: block.hash(),
                        payload: encode_block_payload(&block),
                    }
                },
                P2pMessage::NewJob(hash_bytes(b"test", &[b"gate-0-libp2p-job"])),
                P2pMessage::NewReceipt(hash_bytes(b"test", &[b"gate-0-libp2p-receipt"])),
                P2pMessage::NewAttestation(hash_bytes(b"test", &[b"gate-0-libp2p-attestation"])),
                P2pMessage::PeerInfo {
                    address: address(b"gate-0-libp2p-peer"),
                },
            ];
            for message in gossip_messages {
                let (topic, payload) = encode_gossipsub_message(&message).unwrap();
                producer
                    .swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(topic, payload)
                    .unwrap();
                wait_for_gossip_message(&mut producer, &mut consumer, message).await;
            }

            let tensor_id = hash_bytes(b"test", &[b"gate-0-libp2p-tensor"]);
            let tensor = Tensor::from_vec(vec![1, 3], DType::FieldElement, vec![3, 5, 8]).unwrap();
            let commitment_root = tensor.commitment_root();
            let program_hash = hash_bytes(b"test", &[b"gate-0-libp2p-program"]);
            let request_response_messages = [
                (
                    P2pMessage::RequestTensorChunk {
                        tensor_id,
                        chunk_index: 1,
                    },
                    P2pMessage::TensorChunkResponse {
                        tensor_id,
                        chunk_index: 1,
                        bytes: vec![1, 1, 2, 3, 5, 8],
                    },
                ),
                (
                    P2pMessage::RequestTensorRow {
                        tensor_id,
                        row_index: 2,
                    },
                    P2pMessage::TensorRowResponse {
                        tensor_id,
                        row_index: 2,
                        values: vec![3, 5, 8],
                    },
                ),
                (
                    P2pMessage::RequestTensorByCommitmentRoot { commitment_root },
                    P2pMessage::TensorByCommitmentRootResponse {
                        commitment_root,
                        payload: Some(encode_tensor_payload(&tensor)),
                    },
                ),
                (
                    P2pMessage::RequestProgram(program_hash),
                    P2pMessage::ProgramResponse {
                        program_hash,
                        bytes: b"tensor-vm-gate-0-program".to_vec(),
                    },
                ),
            ];
            for (request, response) in request_response_messages {
                let protocol = request_response_protocol_for_message(&request).unwrap();
                let request_id = send_request_for_protocol(
                    &mut consumer.swarm,
                    protocol,
                    &producer.peer_id,
                    request.clone(),
                );
                wait_for_request_response(
                    &mut producer,
                    &mut consumer,
                    protocol,
                    &request,
                    &response,
                    request_id,
                )
                .await;
            }
        });
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

    async fn wait_for_listen_addr(node: &mut TensorVmLibp2pNode) -> Multiaddr {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let SwarmEvent::NewListenAddr { address, .. } =
                    node.swarm.select_next_some().await
                {
                    break address;
                }
            }
        })
        .await
        .expect("libp2p node must begin listening")
    }

    async fn wait_for_connection(
        producer: &mut TensorVmLibp2pNode,
        consumer: &mut TensorVmLibp2pNode,
    ) {
        tokio::time::timeout(Duration::from_secs(10), async {
            let mut producer_connected = false;
            let mut consumer_connected = false;
            while !(producer_connected && consumer_connected) {
                let producer_event = producer.swarm.select_next_some().fuse();
                let consumer_event = consumer.swarm.select_next_some().fuse();
                futures::pin_mut!(producer_event, consumer_event);
                futures::select! {
                    event = producer_event => {
                        if let SwarmEvent::ConnectionEstablished { peer_id, .. } = event {
                            producer_connected |= peer_id == consumer.peer_id;
                        }
                    }
                    event = consumer_event => {
                        if let SwarmEvent::ConnectionEstablished { peer_id, .. } = event {
                            consumer_connected |= peer_id == producer.peer_id;
                        }
                    }
                }
            }
        })
        .await
        .expect("libp2p swarms must connect");
    }

    async fn wait_for_gossip_subscriptions(
        node: &mut TensorVmLibp2pNode,
        expected_peer: PeerId,
        expected_topics: &[GossipTopic],
    ) {
        tokio::time::timeout(Duration::from_secs(10), async {
            let mut seen_topics = Vec::new();
            loop {
                if let SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::Gossipsub(
                    libp2p::gossipsub::Event::Subscribed { peer_id, topic },
                )) = node.swarm.select_next_some().await
                    && peer_id == expected_peer
                    && expected_topics
                        .iter()
                        .any(|expected| topic.to_string() == expected.as_str())
                    && !seen_topics.contains(&topic.to_string())
                {
                    seen_topics.push(topic.to_string());
                    if seen_topics.len() == expected_topics.len() {
                        break;
                    }
                }
            }
        })
        .await
        .expect("libp2p peer must advertise all TensorVM gossip subscriptions");
    }

    async fn wait_for_gossip_message(
        producer: &mut TensorVmLibp2pNode,
        consumer: &mut TensorVmLibp2pNode,
        expected: P2pMessage,
    ) {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let producer_event = producer.swarm.select_next_some().fuse();
                let consumer_event = consumer.swarm.select_next_some().fuse();
                futures::pin_mut!(producer_event, consumer_event);
                futures::select! {
                    _ = producer_event => {}
                    event = consumer_event => {
                        if let SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::Gossipsub(
                            libp2p::gossipsub::Event::Message {
                                propagation_source,
                                message,
                                ..
                            },
                        )) = event
                        {
                            assert_eq!(propagation_source, producer.peer_id);
                            assert_eq!(decode_message(&message.data).unwrap(), expected);
                            break;
                        }
                    }
                }
            }
        })
        .await
        .expect("libp2p gossipsub message must be delivered");
    }

    async fn wait_for_request_response(
        producer: &mut TensorVmLibp2pNode,
        consumer: &mut TensorVmLibp2pNode,
        protocol: RequestResponseProtocol,
        expected_request: &P2pMessage,
        response: &P2pMessage,
        expected_request_id: libp2p::request_response::OutboundRequestId,
    ) {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let producer_event = producer.swarm.select_next_some().fuse();
                let consumer_event = consumer.swarm.select_next_some().fuse();
                futures::pin_mut!(producer_event, consumer_event);
                futures::select! {
                    event = producer_event => {
                        if let Some(libp2p::request_response::Event::Message {
                                peer,
                                message:
                                    libp2p::request_response::Message::Request {
                                        request,
                                        channel,
                                        ..
                                    },
                            }) = request_response_event_for_protocol(event, protocol)
                        {
                            assert_eq!(peer, consumer.peer_id);
                            assert_eq!(&request, expected_request);
                            send_response_for_protocol(
                                &mut producer.swarm,
                                protocol,
                                channel,
                                response.clone(),
                            )
                                .unwrap();
                        }
                    }
                    event = consumer_event => {
                        if let Some(libp2p::request_response::Event::Message {
                                peer,
                                message:
                                    libp2p::request_response::Message::Response {
                                        request_id,
                                        response: actual_response,
                                    },
                            }) = request_response_event_for_protocol(event, protocol)
                        {
                            assert_eq!(peer, producer.peer_id);
                            assert_eq!(request_id, expected_request_id);
                            assert_eq!(&actual_response, response);
                            break;
                        }
                    }
                }
            }
        })
        .await
        .expect("libp2p request-response exchange must complete");
    }

    fn request_response_event_for_protocol(
        event: SwarmEvent<TensorVmNetworkBehaviourEvent>,
        protocol: RequestResponseProtocol,
    ) -> Option<P2pRequestResponseEvent> {
        match (protocol, event) {
            (
                RequestResponseProtocol::TensorChunk,
                SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::TensorChunkRequestResponse(
                    event,
                )),
            )
            | (
                RequestResponseProtocol::TensorRow,
                SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::TensorRowRequestResponse(
                    event,
                )),
            )
            | (
                RequestResponseProtocol::TensorByRoot,
                SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::TensorByRootRequestResponse(
                    event,
                )),
            )
            | (
                RequestResponseProtocol::Program,
                SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::ProgramRequestResponse(event)),
            ) => Some(event),
            _ => None,
        }
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
