use crate::api::P2pMessage;
use crate::error::{Result as TvmResult, TvmError};
use crate::tensor::Tensor;
use crate::types::Hash;
use futures::StreamExt;
use libp2p::PeerId;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::peer_book::parse_multiaddr;
use super::request_response::{
    PendingRequestKey, RequestResponseCommand, send_request_for_protocol,
};
use super::service_events::{ServiceEventMetrics, handle_swarm_event};
use super::wire::{
    encode_gossipsub_message, is_request_response_request, request_response_protocol_for_message,
};
use super::{Libp2pControlPlaneConfig, build_libp2p_node};

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
