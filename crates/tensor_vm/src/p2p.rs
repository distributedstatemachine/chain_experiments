use crate::api::P2pMessage;
use crate::error::{Result as TvmResult, TvmError};
use crate::tensor::Tensor;
use crate::types::Hash;
use futures::StreamExt;
use libp2p::swarm::SwarmEvent;
use libp2p::{PeerId, Swarm};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

mod peer_book;
mod request_response;
mod wire;

pub use peer_book::{PeerBookStore, PeerRecord};
use peer_book::{bootstrap_peer_address, parse_multiaddr};
pub use request_response::P2pRequestResponseBehaviour;
use request_response::{
    PendingRequestKey, RequestResponseCommand, build_request_response_behaviour,
    handle_request_response_event, send_request_for_protocol,
};
use wire::is_request_response_request;
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

#[derive(libp2p::swarm::NetworkBehaviour)]
pub struct TensorVmNetworkBehaviour {
    pub gossipsub: libp2p::gossipsub::Behaviour,
    pub identify: libp2p::identify::Behaviour,
    pub kademlia: libp2p::kad::Behaviour<libp2p::kad::store::MemoryStore>,
    pub tensor_chunk_request_response: P2pRequestResponseBehaviour,
    pub tensor_row_request_response: P2pRequestResponseBehaviour,
    pub tensor_by_root_request_response: P2pRequestResponseBehaviour,
    pub program_request_response: P2pRequestResponseBehaviour,
}

pub struct TensorVmLibp2pNode {
    pub peer_id: PeerId,
    pub swarm: Swarm<TensorVmNetworkBehaviour>,
    pub identify_protocol: String,
    pub subscribed_topics: Vec<String>,
    pub request_response_protocols: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TensorVmLibp2pServiceInfo {
    pub peer_id: PeerId,
    pub identify_protocol: String,
    pub subscribed_topics: Vec<String>,
    pub request_response_protocols: Vec<String>,
}

pub struct TensorVmLibp2pService {
    info: TensorVmLibp2pServiceInfo,
    connected_peer_count: Arc<AtomicUsize>,
    observed_block_gossip_count: Arc<AtomicUsize>,
    observed_block_payload_gossip_count: Arc<AtomicUsize>,
    observed_block_vote_gossip_count: Arc<AtomicUsize>,
    observed_job_gossip_count: Arc<AtomicUsize>,
    observed_receipt_gossip_count: Arc<AtomicUsize>,
    observed_attestation_gossip_count: Arc<AtomicUsize>,
    latest_observed_block_height: Arc<AtomicU64>,
    latest_observed_block_hash: Arc<Mutex<Hash>>,
    observed_block_hashes: Arc<Mutex<VecDeque<Hash>>>,
    latest_observed_block_payload_height: Arc<AtomicU64>,
    latest_observed_block_payload_hash: Arc<Mutex<Hash>>,
    observed_block_payload_hashes: Arc<Mutex<VecDeque<Hash>>>,
    connected_peer_ids: Arc<Mutex<Vec<PeerId>>>,
    tensor_store: Arc<Mutex<BTreeMap<Hash, Tensor>>>,
    observed_message_rx: Mutex<mpsc::Receiver<P2pMessage>>,
    publish_tx: mpsc::Sender<P2pMessage>,
    request_tx: mpsc::Sender<RequestResponseCommand>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

const OBSERVED_BLOCK_HASH_LIMIT: usize = 256;

impl TensorVmLibp2pService {
    pub fn info(&self) -> &TensorVmLibp2pServiceInfo {
        &self.info
    }

    pub fn peer_id(&self) -> PeerId {
        self.info.peer_id
    }

    pub fn connected_peer_count(&self) -> usize {
        self.connected_peer_count.load(Ordering::Relaxed)
    }

    pub fn observed_block_gossip_count(&self) -> usize {
        self.observed_block_gossip_count.load(Ordering::Relaxed)
    }

    pub fn observed_block_payload_gossip_count(&self) -> usize {
        self.observed_block_payload_gossip_count
            .load(Ordering::Relaxed)
    }

    pub fn observed_block_vote_gossip_count(&self) -> usize {
        self.observed_block_vote_gossip_count
            .load(Ordering::Relaxed)
    }

    pub fn observed_job_gossip_count(&self) -> usize {
        self.observed_job_gossip_count.load(Ordering::Relaxed)
    }

    pub fn observed_receipt_gossip_count(&self) -> usize {
        self.observed_receipt_gossip_count.load(Ordering::Relaxed)
    }

    pub fn observed_attestation_gossip_count(&self) -> usize {
        self.observed_attestation_gossip_count
            .load(Ordering::Relaxed)
    }

    pub fn latest_observed_block_height(&self) -> u64 {
        self.latest_observed_block_height.load(Ordering::Relaxed)
    }

    pub fn latest_observed_block_hash(&self) -> Hash {
        self.latest_observed_block_hash
            .lock()
            .map(|hash| *hash)
            .unwrap_or([0; 32])
    }

    pub fn observed_block_hashes(&self) -> Vec<Hash> {
        self.observed_block_hashes
            .lock()
            .map(|hashes| hashes.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn latest_observed_block_payload_height(&self) -> u64 {
        self.latest_observed_block_payload_height
            .load(Ordering::Relaxed)
    }

    pub fn latest_observed_block_payload_hash(&self) -> Hash {
        self.latest_observed_block_payload_hash
            .lock()
            .map(|hash| *hash)
            .unwrap_or([0; 32])
    }

    pub fn observed_block_payload_hashes(&self) -> Vec<Hash> {
        self.observed_block_payload_hashes
            .lock()
            .map(|hashes| hashes.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn connected_peer_ids(&self) -> Vec<PeerId> {
        self.connected_peer_ids
            .lock()
            .map(|peer_ids| peer_ids.clone())
            .unwrap_or_default()
    }

    pub fn drain_observed_messages(&self) -> Vec<P2pMessage> {
        let receiver = self
            .observed_message_rx
            .lock()
            .expect("observed message receiver mutex poisoned");
        let mut messages = Vec::new();
        while let Ok(message) = receiver.try_recv() {
            messages.push(message);
        }
        messages
    }

    pub fn publish_gossip(&self, message: P2pMessage) -> TvmResult<()> {
        encode_gossipsub_message(&message)?;
        self.publish_tx
            .send(message)
            .map_err(|_| TvmError::InvalidReceipt("libp2p publish worker stopped"))
    }

    pub fn register_tensor(&self, tensor: Tensor) {
        if let Ok(mut tensors) = self.tensor_store.lock() {
            tensors.insert(tensor.tensor_id(), tensor);
        }
    }

    pub fn request_response(
        &self,
        peer_id: PeerId,
        request: P2pMessage,
        timeout: Duration,
    ) -> TvmResult<P2pMessage> {
        if !is_request_response_request(&request) {
            return Err(TvmError::InvalidReceipt(
                "message is not a request-response request",
            ));
        }
        let protocol = request_response_protocol_for_message(&request).ok_or(
            TvmError::InvalidReceipt("request-response protocol missing"),
        )?;
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        self.request_tx
            .send(RequestResponseCommand {
                peer_id,
                protocol,
                request,
                response_tx,
            })
            .map_err(|_| TvmError::InvalidReceipt("libp2p request worker stopped"))?;
        response_rx
            .recv_timeout(timeout)
            .map_err(|_| TvmError::InvalidReceipt("libp2p request-response timeout"))?
            .map_err(TvmError::InvalidReceipt)
    }
}

impl Drop for TensorVmLibp2pService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn build_libp2p_node(config: &Libp2pControlPlaneConfig) -> TvmResult<TensorVmLibp2pNode> {
    let keypair = match config.identity_seed {
        Some(seed) => libp2p::identity::Keypair::ed25519_from_bytes(seed)
            .map_err(|_| TvmError::InvalidReceipt("libp2p identity seed rejected"))?,
        None => libp2p::identity::Keypair::generate_ed25519(),
    };
    let peer_id = PeerId::from(keypair.public());
    let behaviour = build_libp2p_behaviour(config, &keypair)?;
    let identify_protocol = format!("{LIBP2P_PROTOCOL_PREFIX}/identify");
    let subscribed_topics = config
        .gossipsub_topics
        .iter()
        .map(|topic| topic.as_str().to_owned())
        .collect();
    let request_response_protocols = config
        .request_response_protocols
        .iter()
        .map(|protocol| protocol.as_str().to_owned())
        .collect();
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::tls::Config::new,
            libp2p::yamux::Config::default,
        )
        .map_err(|_| TvmError::InvalidReceipt("libp2p transport build failed"))?
        .with_dns()
        .map_err(|_| TvmError::InvalidReceipt("libp2p dns transport build failed"))?
        .with_behaviour(|_| behaviour)
        .map_err(|_| TvmError::InvalidReceipt("libp2p behaviour build failed"))?
        .with_swarm_config(|swarm_config| {
            swarm_config.with_idle_connection_timeout(Duration::from_secs(
                config.idle_connection_timeout_seconds,
            ))
        })
        .build();

    for address in &config.listen_addresses {
        swarm
            .listen_on(parse_multiaddr(address)?)
            .map_err(|_| TvmError::InvalidReceipt("libp2p listen address rejected"))?;
    }
    for address in &config.bootstrap_addresses {
        let multiaddr = parse_multiaddr(address)?;
        if let Some((peer_id, peer_address)) = bootstrap_peer_address(&multiaddr) {
            swarm
                .behaviour_mut()
                .kademlia
                .add_address(&peer_id, peer_address);
        }
        swarm
            .dial(multiaddr)
            .map_err(|_| TvmError::InvalidReceipt("libp2p bootstrap address rejected"))?;
    }

    Ok(TensorVmLibp2pNode {
        peer_id,
        swarm,
        identify_protocol,
        subscribed_topics,
        request_response_protocols,
    })
}

pub fn spawn_libp2p_service(config: Libp2pControlPlaneConfig) -> TvmResult<TensorVmLibp2pService> {
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let connected_peer_count = Arc::new(AtomicUsize::new(0));
    let worker_connected_peer_count = Arc::clone(&connected_peer_count);
    let observed_block_gossip_count = Arc::new(AtomicUsize::new(0));
    let worker_observed_block_gossip_count = Arc::clone(&observed_block_gossip_count);
    let observed_block_payload_gossip_count = Arc::new(AtomicUsize::new(0));
    let worker_observed_block_payload_gossip_count =
        Arc::clone(&observed_block_payload_gossip_count);
    let observed_block_vote_gossip_count = Arc::new(AtomicUsize::new(0));
    let worker_observed_block_vote_gossip_count = Arc::clone(&observed_block_vote_gossip_count);
    let observed_job_gossip_count = Arc::new(AtomicUsize::new(0));
    let worker_observed_job_gossip_count = Arc::clone(&observed_job_gossip_count);
    let observed_receipt_gossip_count = Arc::new(AtomicUsize::new(0));
    let worker_observed_receipt_gossip_count = Arc::clone(&observed_receipt_gossip_count);
    let observed_attestation_gossip_count = Arc::new(AtomicUsize::new(0));
    let worker_observed_attestation_gossip_count = Arc::clone(&observed_attestation_gossip_count);
    let latest_observed_block_height = Arc::new(AtomicU64::new(0));
    let worker_latest_observed_block_height = Arc::clone(&latest_observed_block_height);
    let latest_observed_block_hash = Arc::new(Mutex::new([0; 32]));
    let worker_latest_observed_block_hash = Arc::clone(&latest_observed_block_hash);
    let observed_block_hashes = Arc::new(Mutex::new(VecDeque::new()));
    let worker_observed_block_hashes = Arc::clone(&observed_block_hashes);
    let latest_observed_block_payload_height = Arc::new(AtomicU64::new(0));
    let worker_latest_observed_block_payload_height =
        Arc::clone(&latest_observed_block_payload_height);
    let latest_observed_block_payload_hash = Arc::new(Mutex::new([0; 32]));
    let worker_latest_observed_block_payload_hash = Arc::clone(&latest_observed_block_payload_hash);
    let observed_block_payload_hashes = Arc::new(Mutex::new(VecDeque::new()));
    let worker_observed_block_payload_hashes = Arc::clone(&observed_block_payload_hashes);
    let connected_peer_ids = Arc::new(Mutex::new(Vec::new()));
    let worker_connected_peer_ids = Arc::clone(&connected_peer_ids);
    let tensor_store = Arc::new(Mutex::new(BTreeMap::new()));
    let worker_tensor_store = Arc::clone(&tensor_store);
    let (publish_tx, publish_rx) = mpsc::channel();
    let (request_tx, request_rx) = mpsc::channel::<RequestResponseCommand>();
    let (observed_message_tx, observed_message_rx) = mpsc::channel();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .map_err(|_| TvmError::InvalidReceipt("libp2p runtime build failed"))?;
    let worker = thread::spawn(move || {
        runtime.block_on(async move {
            let mut node = match build_libp2p_node(&config) {
                Ok(node) => node,
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                    return;
                }
            };
            let info = TensorVmLibp2pServiceInfo {
                peer_id: node.peer_id,
                identify_protocol: node.identify_protocol.clone(),
                subscribed_topics: node.subscribed_topics.clone(),
                request_response_protocols: node.request_response_protocols.clone(),
            };
            let _ = ready_tx.send(Ok(info));
            let bootstrap_multiaddrs = config
                .bootstrap_addresses
                .iter()
                .filter_map(|address| parse_multiaddr(address).ok())
                .collect::<Vec<_>>();
            let mut next_bootstrap_dial = Instant::now() + Duration::from_millis(250);
            let mut peer_connections = HashMap::new();
            let event_metrics = ServiceEventMetrics {
                connected_peer_count: worker_connected_peer_count.as_ref(),
                observed_block_gossip_count: worker_observed_block_gossip_count.as_ref(),
                observed_block_payload_gossip_count: worker_observed_block_payload_gossip_count
                    .as_ref(),
                observed_block_vote_gossip_count: worker_observed_block_vote_gossip_count.as_ref(),
                observed_job_gossip_count: worker_observed_job_gossip_count.as_ref(),
                observed_receipt_gossip_count: worker_observed_receipt_gossip_count.as_ref(),
                observed_attestation_gossip_count: worker_observed_attestation_gossip_count
                    .as_ref(),
                latest_observed_block_height: worker_latest_observed_block_height.as_ref(),
                latest_observed_block_hash: worker_latest_observed_block_hash.as_ref(),
                observed_block_hashes: worker_observed_block_hashes.as_ref(),
                latest_observed_block_payload_height: worker_latest_observed_block_payload_height
                    .as_ref(),
                latest_observed_block_payload_hash: worker_latest_observed_block_payload_hash
                    .as_ref(),
                observed_block_payload_hashes: worker_observed_block_payload_hashes.as_ref(),
                connected_peer_ids: worker_connected_peer_ids.as_ref(),
                tensor_store: worker_tensor_store.as_ref(),
                observed_message_tx: &observed_message_tx,
            };
            let mut pending_requests = HashMap::new();

            while !worker_stop.load(Ordering::Relaxed) {
                while let Ok(message) = publish_rx.try_recv() {
                    if let Ok((topic, payload)) = encode_gossipsub_message(&message) {
                        let _ = node.swarm.behaviour_mut().gossipsub.publish(topic, payload);
                    }
                }
                while let Ok(command) = request_rx.try_recv() {
                    if request_response_protocol_for_message(&command.request)
                        != Some(command.protocol)
                        || !node
                            .request_response_protocols
                            .iter()
                            .any(|protocol| protocol == command.protocol.as_str())
                    {
                        let _ = command
                            .response_tx
                            .send(Err("message is not a request-response request"));
                        continue;
                    }
                    let request_id = send_request_for_protocol(
                        &mut node.swarm,
                        command.protocol,
                        &command.peer_id,
                        command.request,
                    );
                    pending_requests.insert(
                        PendingRequestKey {
                            protocol: command.protocol,
                            request_id,
                        },
                        command.response_tx,
                    );
                }
                if let Ok(event) =
                    tokio::time::timeout(Duration::from_millis(100), node.swarm.select_next_some())
                        .await
                {
                    handle_swarm_event(
                        event,
                        &mut peer_connections,
                        &event_metrics,
                        &mut pending_requests,
                        &mut node.swarm,
                    );
                }
                if !bootstrap_multiaddrs.is_empty()
                    && peer_connections.is_empty()
                    && Instant::now() >= next_bootstrap_dial
                {
                    for address in &bootstrap_multiaddrs {
                        let _ = node.swarm.dial(address.clone());
                    }
                    next_bootstrap_dial = Instant::now() + Duration::from_secs(1);
                }
            }
        });
    });

    match ready_rx
        .recv()
        .map_err(|_| TvmError::InvalidReceipt("libp2p service failed to start"))?
    {
        Ok(info) => Ok(TensorVmLibp2pService {
            info,
            connected_peer_count,
            observed_block_gossip_count,
            observed_block_payload_gossip_count,
            observed_block_vote_gossip_count,
            observed_job_gossip_count,
            observed_receipt_gossip_count,
            observed_attestation_gossip_count,
            latest_observed_block_height,
            latest_observed_block_hash,
            observed_block_hashes,
            latest_observed_block_payload_height,
            latest_observed_block_payload_hash,
            observed_block_payload_hashes,
            connected_peer_ids,
            tensor_store,
            observed_message_rx: Mutex::new(observed_message_rx),
            publish_tx,
            request_tx,
            stop,
            worker: Some(worker),
        }),
        Err(error) => {
            let _ = worker.join();
            Err(error)
        }
    }
}

struct ServiceEventMetrics<'a> {
    connected_peer_count: &'a AtomicUsize,
    observed_block_gossip_count: &'a AtomicUsize,
    observed_block_payload_gossip_count: &'a AtomicUsize,
    observed_block_vote_gossip_count: &'a AtomicUsize,
    observed_job_gossip_count: &'a AtomicUsize,
    observed_receipt_gossip_count: &'a AtomicUsize,
    observed_attestation_gossip_count: &'a AtomicUsize,
    latest_observed_block_height: &'a AtomicU64,
    latest_observed_block_hash: &'a Mutex<Hash>,
    observed_block_hashes: &'a Mutex<VecDeque<Hash>>,
    latest_observed_block_payload_height: &'a AtomicU64,
    latest_observed_block_payload_hash: &'a Mutex<Hash>,
    observed_block_payload_hashes: &'a Mutex<VecDeque<Hash>>,
    connected_peer_ids: &'a Mutex<Vec<PeerId>>,
    tensor_store: &'a Mutex<BTreeMap<Hash, Tensor>>,
    observed_message_tx: &'a mpsc::Sender<P2pMessage>,
}

fn handle_swarm_event(
    event: SwarmEvent<TensorVmNetworkBehaviourEvent>,
    peer_connections: &mut HashMap<PeerId, usize>,
    metrics: &ServiceEventMetrics<'_>,
    pending_requests: &mut HashMap<
        PendingRequestKey,
        mpsc::SyncSender<std::result::Result<P2pMessage, &'static str>>,
    >,
    swarm: &mut Swarm<TensorVmNetworkBehaviour>,
) {
    match event {
        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
            *peer_connections.entry(peer_id).or_default() += 1;
            metrics
                .connected_peer_count
                .store(peer_connections.len(), Ordering::Relaxed);
            update_connected_peer_ids(metrics.connected_peer_ids, peer_connections);
        }
        SwarmEvent::ConnectionClosed { peer_id, .. } => {
            if let Some(connection_count) = peer_connections.get_mut(&peer_id) {
                *connection_count = connection_count.saturating_sub(1);
                if *connection_count == 0 {
                    peer_connections.remove(&peer_id);
                }
            }
            metrics
                .connected_peer_count
                .store(peer_connections.len(), Ordering::Relaxed);
            update_connected_peer_ids(metrics.connected_peer_ids, peer_connections);
        }
        SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::Gossipsub(
            libp2p::gossipsub::Event::Message { message, .. },
        )) => {
            if let Ok(message) = decode_message(&message.data) {
                let _ = metrics.observed_message_tx.send(message.clone());
                if let Some((height, block_hash)) = block_announcement(&message) {
                    metrics
                        .observed_block_gossip_count
                        .fetch_add(1, Ordering::Relaxed);
                    let update_latest_block_hash = if height > 0 {
                        let current_height =
                            metrics.latest_observed_block_height.load(Ordering::Relaxed);
                        if height >= current_height {
                            metrics
                                .latest_observed_block_height
                                .store(height, Ordering::Relaxed);
                            true
                        } else {
                            false
                        }
                    } else {
                        metrics.latest_observed_block_height.load(Ordering::Relaxed) == 0
                    };
                    if update_latest_block_hash
                        && let Ok(mut latest_block_hash) = metrics.latest_observed_block_hash.lock()
                    {
                        *latest_block_hash = block_hash;
                    }
                    if let Ok(mut block_hashes) = metrics.observed_block_hashes.lock() {
                        remember_observed_block_hash(&mut block_hashes, block_hash);
                    }
                }
                if let P2pMessage::NewBlockPayload {
                    height, block_hash, ..
                } = &message
                {
                    metrics
                        .observed_block_payload_gossip_count
                        .fetch_add(1, Ordering::Relaxed);
                    let current_height = metrics
                        .latest_observed_block_payload_height
                        .load(Ordering::Relaxed);
                    if *height >= current_height {
                        metrics
                            .latest_observed_block_payload_height
                            .store(*height, Ordering::Relaxed);
                        if let Ok(mut latest_block_hash) =
                            metrics.latest_observed_block_payload_hash.lock()
                        {
                            *latest_block_hash = *block_hash;
                        }
                    }
                    if let Ok(mut block_hashes) = metrics.observed_block_payload_hashes.lock() {
                        remember_observed_block_hash(&mut block_hashes, *block_hash);
                    }
                }
                if matches!(&message, P2pMessage::NewBlockVotePayload { .. }) {
                    metrics
                        .observed_block_vote_gossip_count
                        .fetch_add(1, Ordering::Relaxed);
                }
                if matches!(
                    &message,
                    P2pMessage::NewJob(_) | P2pMessage::NewJobPayload { .. }
                ) {
                    metrics
                        .observed_job_gossip_count
                        .fetch_add(1, Ordering::Relaxed);
                }
                if matches!(
                    &message,
                    P2pMessage::NewReceipt(_) | P2pMessage::NewReceiptPayload { .. }
                ) {
                    metrics
                        .observed_receipt_gossip_count
                        .fetch_add(1, Ordering::Relaxed);
                }
                if matches!(
                    &message,
                    P2pMessage::NewAttestation(_) | P2pMessage::NewAttestationPayload { .. }
                ) {
                    metrics
                        .observed_attestation_gossip_count
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::TensorChunkRequestResponse(event)) => {
            handle_request_response_event(
                RequestResponseProtocol::TensorChunk,
                event,
                metrics,
                pending_requests,
                swarm,
            );
        }
        SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::TensorRowRequestResponse(event)) => {
            handle_request_response_event(
                RequestResponseProtocol::TensorRow,
                event,
                metrics,
                pending_requests,
                swarm,
            );
        }
        SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::TensorByRootRequestResponse(
            event,
        )) => {
            handle_request_response_event(
                RequestResponseProtocol::TensorByRoot,
                event,
                metrics,
                pending_requests,
                swarm,
            );
        }
        SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::ProgramRequestResponse(event)) => {
            handle_request_response_event(
                RequestResponseProtocol::Program,
                event,
                metrics,
                pending_requests,
                swarm,
            );
        }
        _ => {}
    }
}

fn update_connected_peer_ids(
    connected_peer_ids: &Mutex<Vec<PeerId>>,
    peer_connections: &HashMap<PeerId, usize>,
) {
    if let Ok(mut peer_ids) = connected_peer_ids.lock() {
        *peer_ids = peer_connections.keys().copied().collect();
        peer_ids.sort_by_key(|peer_id| peer_id.to_string());
    }
}

fn remember_observed_block_hash(block_hashes: &mut VecDeque<Hash>, block_hash: Hash) {
    if block_hashes.contains(&block_hash) {
        return;
    }
    block_hashes.push_back(block_hash);
    while block_hashes.len() > OBSERVED_BLOCK_HASH_LIMIT {
        block_hashes.pop_front();
    }
}

fn block_announcement(message: &P2pMessage) -> Option<(u64, Hash)> {
    match message {
        P2pMessage::NewBlock(block_hash) => Some((0, *block_hash)),
        P2pMessage::NewBlockHeader { height, block_hash } => Some((*height, *block_hash)),
        P2pMessage::NewBlockPayload {
            height, block_hash, ..
        } => Some((*height, *block_hash)),
        _ => None,
    }
}

fn build_libp2p_behaviour(
    config: &Libp2pControlPlaneConfig,
    keypair: &libp2p::identity::Keypair,
) -> TvmResult<TensorVmNetworkBehaviour> {
    let mut gossipsub_config = libp2p::gossipsub::ConfigBuilder::default();
    gossipsub_config
        .max_transmit_size(config.max_gossipsub_transmit_bytes)
        .validation_mode(libp2p::gossipsub::ValidationMode::Strict);
    let mut gossipsub = libp2p::gossipsub::Behaviour::new(
        libp2p::gossipsub::MessageAuthenticity::Signed(keypair.clone()),
        gossipsub_config
            .build()
            .map_err(|_| TvmError::InvalidReceipt("invalid gossipsub configuration"))?,
    )
    .map_err(|_| TvmError::InvalidReceipt("gossipsub build failed"))?;
    for topic in &config.gossipsub_topics {
        let ident_topic = gossipsub_ident_topic(*topic);
        gossipsub
            .subscribe(&ident_topic)
            .map_err(|_| TvmError::InvalidReceipt("gossipsub subscription failed"))?;
    }

    let identify = libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
        format!("{LIBP2P_PROTOCOL_PREFIX}/identify"),
        keypair.public(),
    ));
    let local_peer_id = PeerId::from(keypair.public());
    let kademlia_store = libp2p::kad::store::MemoryStore::new(local_peer_id);
    let kademlia = libp2p::kad::Behaviour::new(local_peer_id, kademlia_store);
    Ok(TensorVmNetworkBehaviour {
        gossipsub,
        identify,
        kademlia,
        tensor_chunk_request_response: build_request_response_behaviour(
            config,
            RequestResponseProtocol::TensorChunk,
        )?,
        tensor_row_request_response: build_request_response_behaviour(
            config,
            RequestResponseProtocol::TensorRow,
        )?,
        tensor_by_root_request_response: build_request_response_behaviour(
            config,
            RequestResponseProtocol::TensorByRoot,
        )?,
        program_request_response: build_request_response_behaviour(
            config,
            RequestResponseProtocol::Program,
        )?,
    })
}

#[cfg(test)]
mod tests {
    use super::peer_book::{PEER_BOOK_MAGIC, decode_peer_records, read_peer_u64, write_string};
    use super::request_response::{P2pRequestResponseEvent, send_response_for_protocol};
    use super::*;
    use crate::chain::{BlockVote, TensorBlock};
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes};
    use futures::FutureExt;
    use libp2p::Multiaddr;
    use libp2p::multiaddr::Protocol;
    use libp2p::swarm::SwarmEvent;

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
    fn observed_block_hashes_are_bounded_and_deduplicated() {
        let mut block_hashes = VecDeque::new();
        let first_hash = hash_bytes(b"test", &[b"first-observed-block"]);
        remember_observed_block_hash(&mut block_hashes, first_hash);
        remember_observed_block_hash(&mut block_hashes, first_hash);

        for height in 0..(OBSERVED_BLOCK_HASH_LIMIT + 3) {
            let block_hash = hash_bytes(b"test", &[&height.to_le_bytes()]);
            remember_observed_block_hash(&mut block_hashes, block_hash);
        }

        assert_eq!(block_hashes.len(), OBSERVED_BLOCK_HASH_LIMIT);
        assert!(!block_hashes.contains(&first_hash));
        let last_hash = hash_bytes(b"test", &[&(OBSERVED_BLOCK_HASH_LIMIT + 2).to_le_bytes()]);
        assert_eq!(block_hashes.back(), Some(&last_hash));
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

    fn write_test_u64(out: &mut Vec<u8>, value: u64) {
        out.extend_from_slice(&value.to_le_bytes());
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

    #[test]
    fn peer_book_store_persists_libp2p_bootstrap_records_and_detects_tampering() {
        let peer_a = PeerId::random();
        let peer_b = PeerId::random();
        let address_a: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        let address_b: Multiaddr = "/dns/bootstrap.tensorvm.example/tcp/4001".parse().unwrap();
        let records = vec![
            PeerRecord::new(peer_a, address_a.clone()),
            PeerRecord::new(peer_b, address_b.clone()),
        ];
        let path = std::env::temp_dir().join(format!(
            "tensor-vm-libp2p-peer-book-{}-{}.bin",
            std::process::id(),
            records.len()
        ));
        let store = PeerBookStore::new(path.clone());
        store.save_records(&records).unwrap();
        assert_eq!(store.path(), path.as_path());

        let loaded = store.load_records().unwrap();
        assert_eq!(loaded, records);
        assert_eq!(
            store.load_bootstrap_addresses().unwrap(),
            vec![
                format!("{address_a}/p2p/{peer_a}"),
                format!("{address_b}/p2p/{peer_b}")
            ]
        );
        assert_eq!(loaded[0].peer_id().unwrap(), peer_a);
        assert_eq!(loaded[1].multiaddr().unwrap(), address_b);

        let mut bytes = std::fs::read(&path).unwrap();
        bytes[PEER_BOOK_MAGIC.len() + 4] ^= 1;
        std::fs::write(&path, bytes).unwrap();
        assert_eq!(
            store.load_records(),
            Err(TvmError::Storage("peer book checksum mismatch"))
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn peer_book_store_upserts_bootstrap_records_with_peer_ids() {
        let peer_a = PeerId::random();
        let peer_b = PeerId::random();
        let address_a: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        let address_a_updated: Multiaddr = "/ip4/127.0.0.1/tcp/4002".parse().unwrap();
        let address_b = format!("/ip4/127.0.0.1/tcp/4003/p2p/{peer_b}");
        let path = std::env::temp_dir().join(format!(
            "tensor-vm-libp2p-peer-book-upsert-{}-{}.bin",
            std::process::id(),
            peer_a
        ));
        let store = PeerBookStore::new(path.clone());

        let records = store
            .upsert_record(PeerRecord::new(peer_a, address_a))
            .unwrap();
        assert_eq!(records.len(), 1);
        let records = store
            .upsert_record(PeerRecord::from_strings(&peer_b.to_string(), &address_b).unwrap())
            .unwrap();
        assert_eq!(records.len(), 2);
        let records = store
            .upsert_record(PeerRecord::new(peer_a, address_a_updated.clone()))
            .unwrap();
        assert_eq!(records.len(), 2);

        assert_eq!(
            store.load_bootstrap_addresses().unwrap(),
            vec![
                format!("{address_a_updated}/p2p/{peer_a}"),
                address_b.clone()
            ]
        );

        let mismatched_peer = PeerId::random();
        let mismatch = PeerRecord::from_strings(
            &mismatched_peer.to_string(),
            &format!("/ip4/127.0.0.1/tcp/4004/p2p/{peer_a}"),
        );
        assert_eq!(
            mismatch,
            Err(TvmError::Storage("peer book address peer id mismatch"))
        );
        let missing_tcp = PeerRecord::from_strings(&peer_a.to_string(), &format!("/p2p/{peer_a}"));
        assert_eq!(
            missing_tcp,
            Err(TvmError::Storage("peer book address missing tcp port"))
        );
        let zero_tcp = PeerRecord::from_strings(&peer_a.to_string(), "/ip4/127.0.0.1/tcp/0");
        assert_eq!(
            zero_tcp,
            Err(TvmError::Storage("peer book address missing tcp port"))
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn peer_book_decode_rejects_malformed_records() {
        assert_eq!(
            decode_peer_records(b"bad-peer-book"),
            Err(TvmError::Storage("invalid peer book magic"))
        );

        let mut short = Vec::from(PEER_BOOK_MAGIC);
        short.extend_from_slice(&0_u64.to_le_bytes());
        assert_eq!(
            decode_peer_records(&short),
            Err(TvmError::Storage("invalid peer book length"))
        );

        let mut trailing_payload = Vec::new();
        write_test_u64(&mut trailing_payload, 0);
        trailing_payload.push(1);
        let trailing_digest = hash_bytes(b"tensor-vm-libp2p-peer-book-v1", &[&trailing_payload]);
        let mut trailing = Vec::from(PEER_BOOK_MAGIC);
        trailing.extend_from_slice(&trailing_payload);
        trailing.extend_from_slice(&trailing_digest);
        assert_eq!(
            decode_peer_records(&trailing),
            Err(TvmError::Storage("trailing peer book bytes"))
        );

        let mut bad_record_payload = Vec::new();
        write_test_u64(&mut bad_record_payload, 1);
        write_string(&mut bad_record_payload, "not-a-peer-id");
        write_string(&mut bad_record_payload, "/ip4/127.0.0.1/tcp/4001");
        let bad_record_digest =
            hash_bytes(b"tensor-vm-libp2p-peer-book-v1", &[&bad_record_payload]);
        let mut bad_record = Vec::from(PEER_BOOK_MAGIC);
        bad_record.extend_from_slice(&bad_record_payload);
        bad_record.extend_from_slice(&bad_record_digest);
        assert_eq!(
            decode_peer_records(&bad_record),
            Err(TvmError::Storage("invalid peer id"))
        );

        let mut truncated_string_payload = Vec::new();
        write_test_u64(&mut truncated_string_payload, 1);
        write_test_u64(&mut truncated_string_payload, 10);
        truncated_string_payload.extend_from_slice(b"short");
        let truncated_string_digest = hash_bytes(
            b"tensor-vm-libp2p-peer-book-v1",
            &[&truncated_string_payload],
        );
        let mut truncated_string = Vec::from(PEER_BOOK_MAGIC);
        truncated_string.extend_from_slice(&truncated_string_payload);
        truncated_string.extend_from_slice(&truncated_string_digest);
        assert_eq!(
            decode_peer_records(&truncated_string),
            Err(TvmError::Storage("truncated peer book string"))
        );

        let peer = PeerId::random();
        let mut bad_addr_payload = Vec::new();
        write_test_u64(&mut bad_addr_payload, 1);
        write_string(&mut bad_addr_payload, &peer.to_string());
        write_string(&mut bad_addr_payload, "not-a-multiaddr");
        let bad_addr_digest = hash_bytes(b"tensor-vm-libp2p-peer-book-v1", &[&bad_addr_payload]);
        let mut bad_addr = Vec::from(PEER_BOOK_MAGIC);
        bad_addr.extend_from_slice(&bad_addr_payload);
        bad_addr.extend_from_slice(&bad_addr_digest);
        assert_eq!(
            decode_peer_records(&bad_addr),
            Err(TvmError::InvalidReceipt("invalid libp2p multiaddr"))
        );

        assert_eq!(
            read_peer_u64(&[1, 2], &mut 0),
            Err(TvmError::Storage("truncated peer book u64"))
        );
    }
}
