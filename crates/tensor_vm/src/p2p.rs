use crate::api::P2pMessage;
use crate::error::{Result, TvmError};
use crate::types::{Address, Hash};
use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const LIBP2P_PROTOCOL_PREFIX: &str = "/tensorchain/1";
pub const P2P_FRAME_MAGIC: [u8; 4] = *b"TCN1";
const PEER_BOOK_MAGIC: &[u8] = b"TENSORVM_PEER_BOOK_V1\n";
const PEER_RECORD_PAYLOAD_LEN: usize = 32 + 8 + 8 + 8;
const PEER_BOOK_DIGEST_LEN: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkBackend {
    Libp2p,
    Iroh,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkStackRecommendation {
    pub primary: NetworkBackend,
    pub control_plane: NetworkBackend,
    pub tensor_data_plane: NetworkBackend,
    pub future_tensor_blob_candidate: Option<NetworkBackend>,
    pub rationale: Vec<&'static str>,
}

pub fn recommended_network_stack() -> NetworkStackRecommendation {
    NetworkStackRecommendation {
        primary: NetworkBackend::Libp2p,
        control_plane: NetworkBackend::Libp2p,
        tensor_data_plane: NetworkBackend::Libp2p,
        future_tensor_blob_candidate: Some(NetworkBackend::Iroh),
        rationale: vec![
            "gossipsub maps directly to block, job, receipt, attestation, and peer announcements",
            "kademlia/identify/mdns cover the MVP discovery and bootstrap surface",
            "request-response streams cover tensor rows, tensor chunks, and program fetches",
            "iroh is better kept as a later verified blob-transfer data plane once consensus networking is stable",
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestResponseProtocol {
    TensorChunk,
    TensorRow,
    Program,
}

impl RequestResponseProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TensorChunk => "/tensorchain/1/tensor/chunk",
            Self::TensorRow => "/tensorchain/1/tensor/row",
            Self::Program => "/tensorchain/1/program",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Libp2pControlPlaneConfig {
    pub gossipsub_topics: Vec<GossipTopic>,
    pub request_response_protocols: Vec<RequestResponseProtocol>,
    pub enable_identify: bool,
    pub enable_kademlia: bool,
    pub enable_mdns: bool,
    pub max_inbox_messages: usize,
    pub min_peer_score: i64,
    pub max_peer_count: usize,
    pub max_messages_per_window: u64,
    pub valid_message_score_reward: i64,
    pub rate_limit_score_penalty: i64,
    pub rate_limit_backoff_windows: u64,
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
                RequestResponseProtocol::Program,
            ],
            enable_identify: true,
            enable_kademlia: true,
            enable_mdns: true,
            max_inbox_messages: 256,
            min_peer_score: -100,
            max_peer_count: 10_000,
            max_messages_per_window: 1_024,
            valid_message_score_reward: 1,
            rate_limit_score_penalty: 25,
            rate_limit_backoff_windows: 1,
        }
    }
}

pub fn gossip_topic_for_message(message: &P2pMessage) -> Option<GossipTopic> {
    match message {
        P2pMessage::NewBlock(_) => Some(GossipTopic::Blocks),
        P2pMessage::NewJob(_) => Some(GossipTopic::Jobs),
        P2pMessage::NewReceipt(_) => Some(GossipTopic::Receipts),
        P2pMessage::NewAttestation(_) => Some(GossipTopic::Attestations),
        P2pMessage::PeerInfo { .. } => Some(GossipTopic::Peers),
        P2pMessage::RequestTensorChunk { .. }
        | P2pMessage::TensorChunkResponse { .. }
        | P2pMessage::RequestTensorRow { .. }
        | P2pMessage::TensorRowResponse { .. }
        | P2pMessage::RequestProgram(_)
        | P2pMessage::ProgramResponse { .. } => None,
    }
}

pub fn request_response_protocol_for_message(
    message: &P2pMessage,
) -> Option<RequestResponseProtocol> {
    match message {
        P2pMessage::RequestTensorChunk { .. } | P2pMessage::TensorChunkResponse { .. } => {
            Some(RequestResponseProtocol::TensorChunk)
        }
        P2pMessage::RequestTensorRow { .. } | P2pMessage::TensorRowResponse { .. } => {
            Some(RequestResponseProtocol::TensorRow)
        }
        P2pMessage::RequestProgram(_) | P2pMessage::ProgramResponse { .. } => {
            Some(RequestResponseProtocol::Program)
        }
        P2pMessage::NewBlock(_)
        | P2pMessage::NewJob(_)
        | P2pMessage::NewReceipt(_)
        | P2pMessage::NewAttestation(_)
        | P2pMessage::PeerInfo { .. } => None,
    }
}

pub fn encode_message(message: &P2pMessage) -> Vec<u8> {
    let mut out = Vec::new();
    match message {
        P2pMessage::NewBlock(hash) => {
            out.push(1);
            write_hash(&mut out, hash);
        }
        P2pMessage::NewJob(hash) => {
            out.push(2);
            write_hash(&mut out, hash);
        }
        P2pMessage::NewReceipt(hash) => {
            out.push(3);
            write_hash(&mut out, hash);
        }
        P2pMessage::NewAttestation(hash) => {
            out.push(4);
            write_hash(&mut out, hash);
        }
        P2pMessage::RequestTensorChunk {
            tensor_id,
            chunk_index,
        } => {
            out.push(5);
            write_hash(&mut out, tensor_id);
            write_u64(&mut out, *chunk_index);
        }
        P2pMessage::TensorChunkResponse {
            tensor_id,
            chunk_index,
            bytes,
        } => {
            out.push(6);
            write_hash(&mut out, tensor_id);
            write_u64(&mut out, *chunk_index);
            write_bytes(&mut out, bytes);
        }
        P2pMessage::RequestTensorRow {
            tensor_id,
            row_index,
        } => {
            out.push(7);
            write_hash(&mut out, tensor_id);
            write_u64(&mut out, *row_index);
        }
        P2pMessage::TensorRowResponse {
            tensor_id,
            row_index,
            values,
        } => {
            out.push(8);
            write_hash(&mut out, tensor_id);
            write_u64(&mut out, *row_index);
            write_u64(&mut out, values.len() as u64);
            for value in values {
                write_u64(&mut out, *value);
            }
        }
        P2pMessage::RequestProgram(hash) => {
            out.push(9);
            write_hash(&mut out, hash);
        }
        P2pMessage::ProgramResponse {
            program_hash,
            bytes,
        } => {
            out.push(10);
            write_hash(&mut out, program_hash);
            write_bytes(&mut out, bytes);
        }
        P2pMessage::PeerInfo { address } => {
            out.push(11);
            write_hash(&mut out, address);
        }
    }
    out
}

pub fn decode_message(input: &[u8]) -> Result<P2pMessage> {
    let mut reader = Reader::new(input);
    let tag = reader.read_u8()?;
    let message = match tag {
        1 => P2pMessage::NewBlock(reader.read_hash()?),
        2 => P2pMessage::NewJob(reader.read_hash()?),
        3 => P2pMessage::NewReceipt(reader.read_hash()?),
        4 => P2pMessage::NewAttestation(reader.read_hash()?),
        5 => P2pMessage::RequestTensorChunk {
            tensor_id: reader.read_hash()?,
            chunk_index: reader.read_u64()?,
        },
        6 => P2pMessage::TensorChunkResponse {
            tensor_id: reader.read_hash()?,
            chunk_index: reader.read_u64()?,
            bytes: reader.read_bytes()?,
        },
        7 => P2pMessage::RequestTensorRow {
            tensor_id: reader.read_hash()?,
            row_index: reader.read_u64()?,
        },
        8 => {
            let tensor_id = reader.read_hash()?;
            let row_index = reader.read_u64()?;
            let len = reader.read_u64()? as usize;
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(reader.read_u64()?);
            }
            P2pMessage::TensorRowResponse {
                tensor_id,
                row_index,
                values,
            }
        }
        9 => P2pMessage::RequestProgram(reader.read_hash()?),
        10 => P2pMessage::ProgramResponse {
            program_hash: reader.read_hash()?,
            bytes: reader.read_bytes()?,
        },
        11 => P2pMessage::PeerInfo {
            address: reader.read_hash()?,
        },
        _ => return Err(TvmError::InvalidReceipt("unknown p2p message tag")),
    };
    if !reader.is_done() {
        return Err(TvmError::InvalidReceipt("trailing p2p bytes"));
    }
    Ok(message)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct P2pTransportConfig {
    pub max_frame_bytes: usize,
    pub read_timeout_ms: u64,
}

impl Default for P2pTransportConfig {
    fn default() -> Self {
        Self {
            max_frame_bytes: 1024 * 1024,
            read_timeout_ms: 5_000,
        }
    }
}

#[derive(Debug)]
pub struct P2pTcpServer {
    listener: TcpListener,
    config: P2pTransportConfig,
}

impl P2pTcpServer {
    pub fn bind(addr: &str, config: P2pTransportConfig) -> std::io::Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(addr)?,
            config,
        })
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub fn serve_next(&self) -> std::io::Result<P2pMessage> {
        let (mut stream, _) = self.listener.accept()?;
        stream.set_read_timeout(Some(Duration::from_millis(self.config.read_timeout_ms)))?;
        read_framed_message(&mut stream, &self.config)
    }

    pub fn serve_n(&self, max_messages: usize) -> std::io::Result<Vec<P2pMessage>> {
        let mut messages = Vec::with_capacity(max_messages);
        for _ in 0..max_messages {
            messages.push(self.serve_next()?);
        }
        Ok(messages)
    }
}

pub fn send_framed_message(
    addr: SocketAddr,
    message: &P2pMessage,
    config: &P2pTransportConfig,
) -> std::io::Result<()> {
    let mut stream = TcpStream::connect(addr)?;
    write_framed_message(&mut stream, message, config)?;
    stream.flush()
}

pub fn write_framed_message(
    stream: &mut TcpStream,
    message: &P2pMessage,
    config: &P2pTransportConfig,
) -> std::io::Result<()> {
    write_framed_message_to(stream, message, config)
}

pub fn read_framed_message(
    stream: &mut TcpStream,
    config: &P2pTransportConfig,
) -> std::io::Result<P2pMessage> {
    read_framed_message_from(stream, config)
}

pub fn write_framed_message_to<W: Write>(
    writer: &mut W,
    message: &P2pMessage,
    config: &P2pTransportConfig,
) -> std::io::Result<()> {
    writer.write_all(&encode_frame(message, config)?)
}

pub fn read_framed_message_from<R: Read>(
    reader: &mut R,
    config: &P2pTransportConfig,
) -> std::io::Result<P2pMessage> {
    let mut header = [0_u8; 8];
    reader.read_exact(&mut header)?;
    if header[..4] != P2P_FRAME_MAGIC {
        return Err(invalid_data("invalid p2p frame magic"));
    }
    let payload_len = frame_payload_len(&header[4..8]);
    if payload_len as usize > config.max_frame_bytes {
        return Err(invalid_data("p2p frame exceeds configured maximum"));
    }
    let mut payload = vec![0_u8; payload_len as usize];
    reader.read_exact(&mut payload)?;
    decode_message(&payload).map_err(|_| invalid_data("invalid p2p message payload"))
}

pub fn encode_frame(message: &P2pMessage, config: &P2pTransportConfig) -> std::io::Result<Vec<u8>> {
    let payload = encode_message(message);
    if payload.len() > config.max_frame_bytes || payload.len() > u32::MAX as usize {
        return Err(invalid_data("p2p frame exceeds configured maximum"));
    }
    let mut frame = Vec::with_capacity(8 + payload.len());
    frame.extend_from_slice(&P2P_FRAME_MAGIC);
    frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_frame(input: &[u8], config: &P2pTransportConfig) -> std::io::Result<P2pMessage> {
    if input.len() < 8 {
        return Err(invalid_data("short p2p frame"));
    }
    if input[..4] != P2P_FRAME_MAGIC {
        return Err(invalid_data("invalid p2p frame magic"));
    }
    let payload_len = frame_payload_len(&input[4..8]);
    if payload_len as usize > config.max_frame_bytes {
        return Err(invalid_data("p2p frame exceeds configured maximum"));
    }
    let expected_len = 8_usize
        .checked_add(payload_len as usize)
        .ok_or_else(|| invalid_data("p2p frame length overflow"))?;
    if input.len() != expected_len {
        return Err(invalid_data("p2p frame length mismatch"));
    }
    decode_message(&input[8..expected_len]).map_err(|_| invalid_data("invalid p2p message payload"))
}

fn invalid_data(message: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

fn frame_payload_len(bytes: &[u8]) -> u32 {
    let mut len = [0_u8; 4];
    len.copy_from_slice(bytes);
    u32::from_le_bytes(len)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerState {
    pub address: Address,
    pub score: i64,
    pub dropped_messages: u64,
    pub inbox_len: usize,
    pub blocked_until_window: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerRecord {
    pub address: Address,
    pub score: i64,
    pub dropped_messages: u64,
    pub blocked_until_window: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerAdvertisement {
    pub address: Address,
    pub listen_addr: String,
    pub protocols: Vec<String>,
    pub observed_at_window: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerDirectoryConfig {
    pub max_advertisements: usize,
    pub max_listen_addr_len: usize,
    pub max_protocols_per_peer: usize,
    pub max_protocol_len: usize,
}

impl Default for PeerDirectoryConfig {
    fn default() -> Self {
        Self {
            max_advertisements: 10_000,
            max_listen_addr_len: 256,
            max_protocols_per_peer: 16,
            max_protocol_len: 128,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PeerDirectory {
    config: PeerDirectoryConfig,
    advertisements: BTreeMap<Address, PeerAdvertisement>,
}

impl Default for PeerDirectory {
    fn default() -> Self {
        Self::with_config(PeerDirectoryConfig::default())
    }
}

impl PeerDirectory {
    pub fn with_config(config: PeerDirectoryConfig) -> Self {
        Self {
            config,
            advertisements: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.advertisements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.advertisements.is_empty()
    }

    pub fn get(&self, address: &Address) -> Option<&PeerAdvertisement> {
        self.advertisements.get(address)
    }

    pub fn advertise(&mut self, advertisement: PeerAdvertisement) -> Result<bool> {
        self.validate(&advertisement)?;
        match self.advertisements.get(&advertisement.address) {
            Some(existing) if existing.observed_at_window > advertisement.observed_at_window => {
                Ok(false)
            }
            Some(existing) if existing == &advertisement => Ok(false),
            None if self.advertisements.len() >= self.config.max_advertisements => {
                Err(TvmError::InvalidReceipt("peer directory full"))
            }
            _ => {
                self.advertisements
                    .insert(advertisement.address, advertisement);
                Ok(true)
            }
        }
    }

    pub fn bootstrap(
        &mut self,
        advertisements: impl IntoIterator<Item = PeerAdvertisement>,
    ) -> Result<usize> {
        let mut accepted = 0;
        for advertisement in advertisements {
            if self.advertise(advertisement)? {
                accepted += 1;
            }
        }
        Ok(accepted)
    }

    pub fn closest_peers(&self, target: &Hash, limit: usize) -> Vec<PeerAdvertisement> {
        let mut advertisements: Vec<_> = self.advertisements.values().cloned().collect();
        advertisements
            .sort_by(|left, right| compare_peer_distance(target, &left.address, &right.address));
        advertisements.truncate(limit);
        advertisements
    }

    fn validate(&self, advertisement: &PeerAdvertisement) -> Result<()> {
        if advertisement.listen_addr.is_empty() {
            return Err(TvmError::InvalidReceipt("empty peer listen address"));
        }
        if advertisement.listen_addr.len() > self.config.max_listen_addr_len {
            return Err(TvmError::InvalidReceipt("peer listen address too long"));
        }
        if advertisement.protocols.len() > self.config.max_protocols_per_peer {
            return Err(TvmError::InvalidReceipt("too many peer protocols"));
        }
        for protocol in &advertisement.protocols {
            if protocol.is_empty() {
                return Err(TvmError::InvalidReceipt("empty peer protocol"));
            }
            if protocol.len() > self.config.max_protocol_len {
                return Err(TvmError::InvalidReceipt("peer protocol too long"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct PeerInbox {
    score: i64,
    dropped_messages: u64,
    accepted_messages_in_window: u64,
    window: u64,
    blocked_until_window: u64,
    inbox: VecDeque<Vec<u8>>,
}

impl PeerInbox {
    fn new() -> Self {
        Self {
            score: 0,
            dropped_messages: 0,
            accepted_messages_in_window: 0,
            window: 0,
            blocked_until_window: 0,
            inbox: VecDeque::new(),
        }
    }

    fn from_record(record: PeerRecord) -> Self {
        Self {
            score: record.score,
            dropped_messages: record.dropped_messages,
            accepted_messages_in_window: 0,
            window: 0,
            blocked_until_window: record.blocked_until_window,
            inbox: VecDeque::new(),
        }
    }

    fn to_record(&self, address: Address) -> PeerRecord {
        PeerRecord {
            address,
            score: self.score,
            dropped_messages: self.dropped_messages,
            blocked_until_window: self.blocked_until_window,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BroadcastReport {
    pub delivered: usize,
    pub dropped: usize,
    pub discovered: usize,
}

#[derive(Clone, Debug)]
pub struct LocalNetwork {
    config: Libp2pControlPlaneConfig,
    directory: PeerDirectory,
    peers: BTreeMap<Address, PeerInbox>,
    current_window: u64,
}

impl Default for LocalNetwork {
    fn default() -> Self {
        Self::with_config(Libp2pControlPlaneConfig::default())
    }
}

impl LocalNetwork {
    pub fn with_config(config: Libp2pControlPlaneConfig) -> Self {
        let directory = PeerDirectory::with_config(PeerDirectoryConfig {
            max_advertisements: config.max_peer_count,
            ..PeerDirectoryConfig::default()
        });
        Self {
            config,
            directory,
            peers: BTreeMap::new(),
            current_window: 0,
        }
    }

    pub fn add_peer(&mut self, address: Address) {
        let _ = self.try_add_peer(address);
    }

    pub fn try_add_peer(&mut self, address: Address) -> Result<bool> {
        if self.peers.contains_key(&address) {
            return Ok(false);
        }
        if self.peers.len() >= self.config.max_peer_count {
            return Err(TvmError::InvalidReceipt("peer limit reached"));
        }
        self.peers.insert(address, PeerInbox::new());
        Ok(true)
    }

    pub fn from_peer_records(
        config: Libp2pControlPlaneConfig,
        records: impl IntoIterator<Item = PeerRecord>,
    ) -> Result<Self> {
        let mut network = Self::with_config(config);
        for record in records {
            if network.peers.contains_key(&record.address) {
                return Err(TvmError::Storage("duplicate peer record"));
            }
            if network.peers.len() >= network.config.max_peer_count {
                return Err(TvmError::Storage("peer book exceeds peer limit"));
            }
            network
                .peers
                .insert(record.address, PeerInbox::from_record(record));
        }
        Ok(network)
    }

    pub fn advertise_peer(&mut self, advertisement: PeerAdvertisement) -> Result<bool> {
        let address = advertisement.address;
        if !self.peers.contains_key(&address) && self.peers.len() >= self.config.max_peer_count {
            return Err(TvmError::InvalidReceipt("peer limit reached"));
        }
        let directory_updated = self.directory.advertise(advertisement)?;
        let peer_added = self.try_add_peer(address)?;
        Ok(directory_updated || peer_added)
    }

    pub fn bootstrap_peers(
        &mut self,
        advertisements: impl IntoIterator<Item = PeerAdvertisement>,
    ) -> Result<usize> {
        let mut accepted = 0;
        for advertisement in advertisements {
            if self.advertise_peer(advertisement)? {
                accepted += 1;
            }
        }
        Ok(accepted)
    }

    pub fn peer_advertisement(&self, address: &Address) -> Option<&PeerAdvertisement> {
        self.directory.get(address)
    }

    pub fn closest_peers(&self, target: &Hash, limit: usize) -> Vec<PeerAdvertisement> {
        self.directory.closest_peers(target, limit)
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    pub fn current_window(&self) -> u64 {
        self.current_window
    }

    pub fn inbox_len(&self, address: &Address) -> Result<usize> {
        self.peers
            .get(address)
            .map(|peer| peer.inbox.len())
            .ok_or(TvmError::InvalidReceipt("unknown peer"))
    }

    pub fn peer_state(&self, address: &Address) -> Result<PeerState> {
        let peer = self
            .peers
            .get(address)
            .ok_or(TvmError::InvalidReceipt("unknown peer"))?;
        Ok(PeerState {
            address: *address,
            score: peer.score,
            dropped_messages: peer.dropped_messages,
            inbox_len: peer.inbox.len(),
            blocked_until_window: peer.blocked_until_window,
        })
    }

    pub fn set_peer_score(&mut self, address: &Address, score: i64) -> Result<()> {
        let peer = self
            .peers
            .get_mut(address)
            .ok_or(TvmError::InvalidReceipt("unknown peer"))?;
        peer.score = score;
        Ok(())
    }

    pub fn peer_records(&self) -> Vec<PeerRecord> {
        self.peers
            .iter()
            .map(|(address, peer)| peer.to_record(*address))
            .collect()
    }

    pub fn advance_admission_window(&mut self) {
        self.current_window = self.current_window.saturating_add(1);
    }

    pub fn broadcast(&mut self, from: Address, message: &P2pMessage) {
        let _ = self.try_broadcast(from, message);
    }

    pub fn try_broadcast(
        &mut self,
        from: Address,
        message: &P2pMessage,
    ) -> Result<BroadcastReport> {
        if !self.admit_inbound(&from)? {
            return Ok(BroadcastReport {
                delivered: 0,
                dropped: self.peers.len().saturating_sub(1),
                discovered: 0,
            });
        }
        let discovered = self.discover_from_message(message);
        let encoded = encode_message(message);
        let mut delivered = 0;
        let mut dropped = 0;
        for (address, peer) in &mut self.peers {
            if *address == from {
                continue;
            }
            if peer.score < self.config.min_peer_score
                || peer.inbox.len() >= self.config.max_inbox_messages
            {
                peer.dropped_messages = peer.dropped_messages.saturating_add(1);
                dropped += 1;
                continue;
            }
            peer.inbox.push_back(encoded.clone());
            delivered += 1;
        }
        Ok(BroadcastReport {
            delivered,
            dropped,
            discovered,
        })
    }

    pub fn admit_inbound(&mut self, from: &Address) -> Result<bool> {
        let peer = self
            .peers
            .get_mut(from)
            .ok_or(TvmError::InvalidReceipt("unknown peer"))?;
        if peer.window != self.current_window {
            peer.window = self.current_window;
            peer.accepted_messages_in_window = 0;
        }
        if self.current_window < peer.blocked_until_window
            || peer.score < self.config.min_peer_score
        {
            peer.dropped_messages = peer.dropped_messages.saturating_add(1);
            return Ok(false);
        }
        if peer.accepted_messages_in_window >= self.config.max_messages_per_window {
            peer.score = peer
                .score
                .saturating_sub(self.config.rate_limit_score_penalty);
            peer.blocked_until_window = self
                .current_window
                .saturating_add(self.config.rate_limit_backoff_windows);
            peer.dropped_messages = peer.dropped_messages.saturating_add(1);
            return Ok(false);
        }
        peer.accepted_messages_in_window = peer.accepted_messages_in_window.saturating_add(1);
        peer.score = peer
            .score
            .saturating_add(self.config.valid_message_score_reward);
        Ok(true)
    }

    pub fn recv(&mut self, address: &Address) -> Result<Option<P2pMessage>> {
        let Some(inbox) = self.peers.get_mut(address) else {
            return Err(TvmError::InvalidReceipt("unknown peer"));
        };
        inbox
            .inbox
            .pop_front()
            .map(|bytes| decode_message(&bytes))
            .transpose()
    }

    fn discover_from_message(&mut self, message: &P2pMessage) -> usize {
        let P2pMessage::PeerInfo { address } = message else {
            return 0;
        };
        usize::from(self.try_add_peer(*address).unwrap_or(false))
    }
}

fn compare_peer_distance(target: &Hash, left: &Address, right: &Address) -> Ordering {
    for (index, (left_byte, right_byte)) in left.iter().zip(right.iter()).enumerate() {
        let left_distance = *left_byte ^ target[index];
        let right_distance = *right_byte ^ target[index];
        match left_distance.cmp(&right_distance) {
            Ordering::Equal => continue,
            ordering => return ordering,
        }
    }
    left.cmp(right)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerBookStore {
    path: PathBuf,
}

impl PeerBookStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save_network(&self, network: &LocalNetwork) -> Result<()> {
        self.save_records(&network.peer_records())
    }

    pub fn save_records(&self, records: &[PeerRecord]) -> Result<()> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .map_err(|_| TvmError::Storage("failed to create peer book directory"))?;
        }
        fs::write(&self.path, encode_peer_records(records))
            .map_err(|_| TvmError::Storage("failed to write peer book"))?;
        Ok(())
    }

    pub fn load_records(&self) -> Result<Vec<PeerRecord>> {
        let bytes =
            fs::read(&self.path).map_err(|_| TvmError::Storage("failed to read peer book"))?;
        decode_peer_records(&bytes)
    }

    pub fn load_network(&self, config: Libp2pControlPlaneConfig) -> Result<LocalNetwork> {
        LocalNetwork::from_peer_records(config, self.load_records()?)
    }
}

fn encode_peer_records(records: &[PeerRecord]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(8 + records.len() * PEER_RECORD_PAYLOAD_LEN);
    write_u64(&mut payload, records.len() as u64);
    for record in records {
        write_hash(&mut payload, &record.address);
        payload.extend_from_slice(&record.score.to_le_bytes());
        write_u64(&mut payload, record.dropped_messages);
        write_u64(&mut payload, record.blocked_until_window);
    }
    let digest = crate::types::hash_bytes(b"tensor-vm-peer-book-v1", &[&payload]);
    let mut encoded =
        Vec::with_capacity(PEER_BOOK_MAGIC.len() + payload.len() + PEER_BOOK_DIGEST_LEN);
    encoded.extend_from_slice(PEER_BOOK_MAGIC);
    encoded.extend_from_slice(&payload);
    encoded.extend_from_slice(&digest);
    encoded
}

fn decode_peer_records(bytes: &[u8]) -> Result<Vec<PeerRecord>> {
    if !bytes.starts_with(PEER_BOOK_MAGIC) {
        return Err(TvmError::Storage("invalid peer book magic"));
    }
    let minimum_len = PEER_BOOK_MAGIC.len() + 8 + PEER_BOOK_DIGEST_LEN;
    if bytes.len() < minimum_len {
        return Err(TvmError::Storage("invalid peer book length"));
    }
    let payload_start = PEER_BOOK_MAGIC.len();
    let payload_end = bytes.len() - PEER_BOOK_DIGEST_LEN;
    let payload = &bytes[payload_start..payload_end];
    let expected_digest = crate::types::hash_bytes(b"tensor-vm-peer-book-v1", &[payload]);
    if bytes[payload_end..] != expected_digest {
        return Err(TvmError::Storage("peer book checksum mismatch"));
    }

    let mut offset = 0;
    let record_count = read_peer_u64(payload, &mut offset)? as usize;
    let expected_payload_len = 8_usize
        .checked_add(
            record_count
                .checked_mul(PEER_RECORD_PAYLOAD_LEN)
                .ok_or(TvmError::Storage("peer book length overflow"))?,
        )
        .ok_or(TvmError::Storage("peer book length overflow"))?;
    if payload.len() != expected_payload_len {
        return Err(TvmError::Storage("invalid peer book length"));
    }

    let mut records = Vec::with_capacity(record_count);
    for _ in 0..record_count {
        let address = read_peer_hash(payload, &mut offset)?;
        let score = read_peer_i64(payload, &mut offset)?;
        let dropped_messages = read_peer_u64(payload, &mut offset)?;
        let blocked_until_window = read_peer_u64(payload, &mut offset)?;
        records.push(PeerRecord {
            address,
            score,
            dropped_messages,
            blocked_until_window,
        });
    }
    Ok(records)
}

fn read_peer_u64(bytes: &[u8], offset: &mut usize) -> Result<u64> {
    if bytes.len().saturating_sub(*offset) < 8 {
        return Err(TvmError::Storage("truncated peer book u64"));
    }
    let mut out = [0_u8; 8];
    out.copy_from_slice(&bytes[*offset..*offset + 8]);
    *offset += 8;
    Ok(u64::from_le_bytes(out))
}

fn read_peer_i64(bytes: &[u8], offset: &mut usize) -> Result<i64> {
    if bytes.len().saturating_sub(*offset) < 8 {
        return Err(TvmError::Storage("truncated peer book i64"));
    }
    let mut out = [0_u8; 8];
    out.copy_from_slice(&bytes[*offset..*offset + 8]);
    *offset += 8;
    Ok(i64::from_le_bytes(out))
}

fn read_peer_hash(bytes: &[u8], offset: &mut usize) -> Result<Hash> {
    if bytes.len().saturating_sub(*offset) < 32 {
        return Err(TvmError::Storage("truncated peer book address"));
    }
    let mut out = [0_u8; 32];
    out.copy_from_slice(&bytes[*offset..*offset + 32]);
    *offset += 32;
    Ok(out)
}

fn write_hash(out: &mut Vec<u8>, hash: &Hash) {
    out.extend_from_slice(hash);
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    write_u64(out, bytes.len() as u64);
    out.extend_from_slice(bytes);
}

struct Reader<'a> {
    input: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    fn read_u8(&mut self) -> Result<u8> {
        let Some(byte) = self.input.get(self.offset).copied() else {
            return Err(TvmError::InvalidReceipt("short p2p message"));
        };
        self.offset += 1;
        Ok(byte)
    }

    fn read_u64(&mut self) -> Result<u64> {
        let bytes = self.read_exact(8)?;
        let mut out = [0_u8; 8];
        out.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(out))
    }

    fn read_hash(&mut self) -> Result<Hash> {
        let bytes = self.read_exact(32)?;
        let mut out = [0_u8; 32];
        out.copy_from_slice(bytes);
        Ok(out)
    }

    fn read_bytes(&mut self) -> Result<Vec<u8>> {
        let len = self.read_u64()? as usize;
        Ok(self.read_exact(len)?.to_vec())
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(TvmError::InvalidReceipt("p2p length overflow"))?;
        let Some(bytes) = self.input.get(self.offset..end) else {
            return Err(TvmError::InvalidReceipt("short p2p message"));
        };
        self.offset = end;
        Ok(bytes)
    }

    fn is_done(&self) -> bool {
        self.offset == self.input.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{address, hash_bytes};

    #[test]
    fn p2p_messages_roundtrip() {
        let h = hash_bytes(b"test", &[b"h"]);
        let peer = address(b"peer");
        let messages = vec![
            P2pMessage::NewBlock(h),
            P2pMessage::NewJob(h),
            P2pMessage::NewReceipt(h),
            P2pMessage::NewAttestation(h),
            P2pMessage::RequestTensorChunk {
                tensor_id: h,
                chunk_index: 7,
            },
            P2pMessage::TensorChunkResponse {
                tensor_id: h,
                chunk_index: 7,
                bytes: vec![1, 2, 3],
            },
            P2pMessage::RequestTensorRow {
                tensor_id: h,
                row_index: 9,
            },
            P2pMessage::TensorRowResponse {
                tensor_id: h,
                row_index: 9,
                values: vec![4, 5, 6],
            },
            P2pMessage::RequestProgram(h),
            P2pMessage::ProgramResponse {
                program_hash: h,
                bytes: vec![7, 8],
            },
            P2pMessage::PeerInfo { address: peer },
        ];

        for message in messages {
            assert_eq!(decode_message(&encode_message(&message)).unwrap(), message);
        }
    }

    #[test]
    fn local_network_broadcasts_to_other_peers() {
        let a = address(b"a");
        let b = address(b"b");
        let h = hash_bytes(b"test", &[b"block"]);
        let mut network = LocalNetwork::default();
        network.add_peer(a);
        network.add_peer(b);
        network.broadcast(a, &P2pMessage::NewBlock(h));
        assert_eq!(network.recv(&a).unwrap(), None);
        assert_eq!(network.recv(&b).unwrap(), Some(P2pMessage::NewBlock(h)));
    }

    #[test]
    fn libp2p_mapping_separates_gossip_and_request_response() {
        let h = hash_bytes(b"test", &[b"h"]);
        let recommendation = recommended_network_stack();
        assert_eq!(recommendation.primary, NetworkBackend::Libp2p);
        assert_eq!(recommendation.control_plane, NetworkBackend::Libp2p);
        assert_eq!(recommendation.tensor_data_plane, NetworkBackend::Libp2p);
        assert_eq!(
            recommendation.future_tensor_blob_candidate,
            Some(NetworkBackend::Iroh)
        );
        assert!(
            recommendation
                .rationale
                .iter()
                .any(|reason| reason.contains("gossipsub"))
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewBlock(h)),
            Some(GossipTopic::Blocks)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewJob(h)),
            Some(GossipTopic::Jobs)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewReceipt(h)),
            Some(GossipTopic::Receipts)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewAttestation(h)),
            Some(GossipTopic::Attestations)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::PeerInfo { address: h }),
            Some(GossipTopic::Peers)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::RequestProgram(h)),
            None
        );
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::RequestTensorChunk {
                tensor_id: h,
                chunk_index: 0,
            }),
            Some(RequestResponseProtocol::TensorChunk)
        );
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::RequestTensorRow {
                tensor_id: h,
                row_index: 0,
            }),
            Some(RequestResponseProtocol::TensorRow)
        );
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::RequestProgram(h)),
            Some(RequestResponseProtocol::Program)
        );
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::NewBlock(h)),
            None
        );
        assert_eq!(GossipTopic::Blocks.as_str(), "/tensorchain/1/blocks");
        assert_eq!(GossipTopic::Jobs.as_str(), "/tensorchain/1/jobs");
        assert_eq!(GossipTopic::Receipts.as_str(), "/tensorchain/1/receipts");
        assert_eq!(
            GossipTopic::Attestations.as_str(),
            "/tensorchain/1/attestations"
        );
        assert_eq!(GossipTopic::Peers.as_str(), "/tensorchain/1/peers");
        assert_eq!(
            RequestResponseProtocol::TensorRow.as_str(),
            "/tensorchain/1/tensor/row"
        );
        assert_eq!(
            RequestResponseProtocol::TensorChunk.as_str(),
            "/tensorchain/1/tensor/chunk"
        );
        assert_eq!(
            RequestResponseProtocol::Program.as_str(),
            "/tensorchain/1/program"
        );
        let config = Libp2pControlPlaneConfig::default();
        assert!(config.enable_identify);
        assert!(config.enable_kademlia);
        assert!(config.enable_mdns);
    }

    #[test]
    fn local_network_applies_backpressure_and_peer_scores() {
        let a = address(b"a");
        let b = address(b"b");
        let c = address(b"c");
        let h = hash_bytes(b"test", &[b"block"]);
        let mut network = LocalNetwork::with_config(Libp2pControlPlaneConfig {
            max_inbox_messages: 1,
            min_peer_score: -10,
            ..Libp2pControlPlaneConfig::default()
        });
        network.add_peer(a);
        network.add_peer(b);
        network.add_peer(c);
        network.set_peer_score(&c, -20).unwrap();

        let first = network.try_broadcast(a, &P2pMessage::NewBlock(h)).unwrap();
        assert_eq!(
            first,
            BroadcastReport {
                delivered: 1,
                dropped: 1,
                discovered: 0,
            }
        );
        let second = network.try_broadcast(a, &P2pMessage::NewJob(h)).unwrap();
        assert_eq!(second.delivered, 0);
        assert_eq!(second.dropped, 2);
        assert_eq!(network.inbox_len(&b).unwrap(), 1);
        assert_eq!(network.peer_state(&b).unwrap().dropped_messages, 1);
        assert_eq!(network.peer_state(&c).unwrap().dropped_messages, 2);
    }

    #[test]
    fn local_network_rate_limits_and_backs_off_flooding_peer() {
        let a = address(b"rate-limited-a");
        let b = address(b"rate-limited-b");
        let h = hash_bytes(b"test", &[b"rate-limited-block"]);
        let mut network = LocalNetwork::with_config(Libp2pControlPlaneConfig {
            max_messages_per_window: 1,
            valid_message_score_reward: 0,
            rate_limit_score_penalty: 7,
            rate_limit_backoff_windows: 2,
            ..Libp2pControlPlaneConfig::default()
        });
        network.add_peer(a);
        network.add_peer(b);

        let first = network.try_broadcast(a, &P2pMessage::NewBlock(h)).unwrap();
        assert_eq!(first.delivered, 1);
        assert_eq!(network.recv(&b).unwrap(), Some(P2pMessage::NewBlock(h)));

        let second = network.try_broadcast(a, &P2pMessage::NewJob(h)).unwrap();
        assert_eq!(
            second,
            BroadcastReport {
                delivered: 0,
                dropped: 1,
                discovered: 0,
            }
        );
        let penalized = network.peer_state(&a).unwrap();
        assert_eq!(penalized.score, -7);
        assert_eq!(penalized.dropped_messages, 1);
        assert_eq!(penalized.blocked_until_window, 2);
        assert_eq!(network.recv(&b).unwrap(), None);

        network.advance_admission_window();
        assert_eq!(network.current_window(), 1);
        let blocked = network
            .try_broadcast(a, &P2pMessage::NewReceipt(h))
            .unwrap();
        assert_eq!(blocked.delivered, 0);
        assert_eq!(network.peer_state(&a).unwrap().dropped_messages, 2);

        network.advance_admission_window();
        let admitted = network
            .try_broadcast(a, &P2pMessage::NewAttestation(h))
            .unwrap();
        assert_eq!(admitted.delivered, 1);
        assert_eq!(
            network.recv(&b).unwrap(),
            Some(P2pMessage::NewAttestation(h))
        );
    }

    #[test]
    fn local_network_enforces_peer_limit_on_discovery() {
        let a = address(b"peer-limit-a");
        let b = address(b"peer-limit-b");
        let c = address(b"peer-limit-c");
        let mut network = LocalNetwork::with_config(Libp2pControlPlaneConfig {
            max_peer_count: 2,
            ..Libp2pControlPlaneConfig::default()
        });
        assert!(network.try_add_peer(a).unwrap());
        assert!(!network.try_add_peer(a).unwrap());
        assert!(network.try_add_peer(b).unwrap());
        assert_eq!(
            network.try_add_peer(c),
            Err(TvmError::InvalidReceipt("peer limit reached"))
        );

        let report = network
            .try_broadcast(a, &P2pMessage::PeerInfo { address: c })
            .unwrap();
        assert_eq!(report.discovered, 0);
        assert_eq!(network.peer_count(), 2);
        assert!(network.peer_state(&c).is_err());
    }

    #[test]
    fn local_network_discovers_peers_from_peer_info() {
        let a = address(b"a");
        let discovered = address(b"discovered");
        let mut network = LocalNetwork::default();
        network.add_peer(a);
        let report = network
            .try_broadcast(
                a,
                &P2pMessage::PeerInfo {
                    address: discovered,
                },
            )
            .unwrap();
        assert_eq!(report.discovered, 1);
        assert_eq!(network.peer_count(), 2);
        assert!(network.peer_state(&discovered).is_ok());
    }

    fn test_advertisement(address: Address, observed_at_window: u64) -> PeerAdvertisement {
        PeerAdvertisement {
            address,
            listen_addr: format!("/ip4/127.0.0.1/tcp/{}", 10_000 + u16::from(address[0])),
            protocols: vec![
                LIBP2P_PROTOCOL_PREFIX.to_owned(),
                GossipTopic::Blocks.as_str().to_owned(),
            ],
            observed_at_window,
        }
    }

    #[test]
    fn peer_directory_bootstraps_and_finds_kademlia_style_closest_peers() {
        let mut target = [0_u8; 32];
        target[0] = 0;
        let mut near = [0_u8; 32];
        near[0] = 1;
        let mut mid = [0_u8; 32];
        mid[0] = 2;
        let mut far = [0_u8; 32];
        far[0] = 200;

        let mut directory = PeerDirectory::default();
        assert!(directory.is_empty());
        assert_eq!(
            directory
                .bootstrap([
                    test_advertisement(far, 1),
                    test_advertisement(mid, 1),
                    test_advertisement(near, 1),
                ])
                .unwrap(),
            3
        );
        assert_eq!(directory.len(), 3);
        let closest = directory.closest_peers(&target, 2);
        assert_eq!(
            closest
                .iter()
                .map(|advertisement| advertisement.address)
                .collect::<Vec<_>>(),
            vec![near, mid]
        );

        let mut tie_left = [7_u8; 32];
        tie_left[31] = 1;
        let mut tie_right = [7_u8; 32];
        tie_right[31] = 2;
        let mut tie_directory = PeerDirectory::default();
        tie_directory
            .bootstrap([
                test_advertisement(tie_right, 1),
                test_advertisement(tie_left, 1),
            ])
            .unwrap();
        assert_eq!(
            tie_directory.closest_peers(&[0; 32], 2)[0].address,
            tie_left
        );
        assert_eq!(
            compare_peer_distance(&[0; 32], &tie_left, &tie_left),
            std::cmp::Ordering::Equal
        );

        let mut stale = test_advertisement(near, 0);
        stale.listen_addr = "/ip4/127.0.0.1/tcp/1".to_owned();
        assert!(!directory.advertise(stale).unwrap());
        assert_ne!(
            directory.get(&near).unwrap().listen_addr,
            "/ip4/127.0.0.1/tcp/1"
        );

        let mut newer = test_advertisement(near, 2);
        newer.listen_addr = "/ip4/127.0.0.1/tcp/2".to_owned();
        assert!(directory.advertise(newer).unwrap());
        assert_eq!(
            directory.get(&near).unwrap().listen_addr,
            "/ip4/127.0.0.1/tcp/2"
        );
    }

    #[test]
    fn peer_directory_rejects_invalid_or_excessive_advertisements() {
        let mut peer = [0_u8; 32];
        peer[0] = 1;
        let mut directory = PeerDirectory::with_config(PeerDirectoryConfig {
            max_advertisements: 1,
            max_listen_addr_len: 8,
            max_protocols_per_peer: 1,
            max_protocol_len: 8,
        });

        let mut empty_addr = test_advertisement(peer, 1);
        empty_addr.listen_addr.clear();
        assert_eq!(
            directory.advertise(empty_addr),
            Err(TvmError::InvalidReceipt("empty peer listen address"))
        );

        let mut long_addr = test_advertisement(peer, 1);
        long_addr.listen_addr = "x".repeat(9);
        assert_eq!(
            directory.advertise(long_addr),
            Err(TvmError::InvalidReceipt("peer listen address too long"))
        );

        let mut too_many_protocols = test_advertisement(peer, 1);
        too_many_protocols.listen_addr = "addr".to_owned();
        too_many_protocols.protocols = vec!["/a".to_owned(), "/b".to_owned()];
        assert_eq!(
            directory.advertise(too_many_protocols),
            Err(TvmError::InvalidReceipt("too many peer protocols"))
        );

        let mut empty_protocol = test_advertisement(peer, 1);
        empty_protocol.listen_addr = "addr".to_owned();
        empty_protocol.protocols = vec![String::new()];
        assert_eq!(
            directory.advertise(empty_protocol),
            Err(TvmError::InvalidReceipt("empty peer protocol"))
        );

        let mut long_protocol = test_advertisement(peer, 1);
        long_protocol.listen_addr = "addr".to_owned();
        long_protocol.protocols = vec!["x".repeat(9)];
        assert_eq!(
            directory.advertise(long_protocol),
            Err(TvmError::InvalidReceipt("peer protocol too long"))
        );

        let mut accepted = test_advertisement(peer, 1);
        accepted.listen_addr = "addr".to_owned();
        accepted.protocols = vec!["/tc".to_owned()];
        assert!(directory.advertise(accepted).unwrap());

        let mut second = [0_u8; 32];
        second[0] = 2;
        let mut second_advertisement = test_advertisement(second, 1);
        second_advertisement.listen_addr = "addr2".to_owned();
        second_advertisement.protocols = vec!["/tc".to_owned()];
        assert_eq!(
            directory.advertise(second_advertisement),
            Err(TvmError::InvalidReceipt("peer directory full"))
        );
    }

    #[test]
    fn local_network_bootstraps_advertisements_and_exposes_closest_peers() {
        let mut a = [0_u8; 32];
        a[0] = 1;
        let mut b = [0_u8; 32];
        b[0] = 2;
        let mut c = [0_u8; 32];
        c[0] = 3;
        let mut target = [0_u8; 32];
        target[0] = 0;
        let mut network = LocalNetwork::with_config(Libp2pControlPlaneConfig {
            max_peer_count: 2,
            ..Libp2pControlPlaneConfig::default()
        });

        assert_eq!(
            network
                .bootstrap_peers([test_advertisement(b, 1), test_advertisement(a, 1)])
                .unwrap(),
            2
        );
        assert_eq!(network.peer_count(), 2);
        assert_eq!(network.closest_peers(&target, 1)[0].address, a);
        assert_eq!(
            network.peer_advertisement(&b).unwrap().listen_addr,
            test_advertisement(b, 1).listen_addr
        );
        assert_eq!(
            network.bootstrap_peers([test_advertisement(a, 1)]).unwrap(),
            0
        );
        assert_eq!(
            network.advertise_peer(test_advertisement(c, 1)),
            Err(TvmError::InvalidReceipt("peer limit reached"))
        );
    }

    #[test]
    fn peer_book_store_persists_scores_and_detects_tampering() {
        let a = address(b"peer-book-a");
        let b = address(b"peer-book-b");
        let h = hash_bytes(b"test", &[b"peer-book"]);
        let mut network = LocalNetwork::with_config(Libp2pControlPlaneConfig {
            max_messages_per_window: 1,
            rate_limit_score_penalty: 5,
            rate_limit_backoff_windows: 3,
            ..Libp2pControlPlaneConfig::default()
        });
        network.add_peer(a);
        network.add_peer(b);
        network.try_broadcast(a, &P2pMessage::NewBlock(h)).unwrap();
        network.try_broadcast(a, &P2pMessage::NewJob(h)).unwrap();
        network.set_peer_score(&b, -12).unwrap();

        let path = std::env::temp_dir().join(format!(
            "tensor-vm-peer-book-{}-{}.bin",
            std::process::id(),
            network.peer_count()
        ));
        let store = PeerBookStore::new(path.clone());
        store.save_network(&network).unwrap();
        assert_eq!(store.path(), path.as_path());

        let loaded = store
            .load_network(Libp2pControlPlaneConfig::default())
            .unwrap();
        assert_eq!(loaded.peer_count(), 2);
        assert_eq!(loaded.peer_state(&a).unwrap().score, -4);
        assert_eq!(loaded.peer_state(&a).unwrap().dropped_messages, 1);
        assert_eq!(loaded.peer_state(&a).unwrap().blocked_until_window, 3);
        assert_eq!(loaded.peer_state(&b).unwrap().score, -12);

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
    fn peer_book_decode_rejects_malformed_records_and_limits() {
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

        let mut payload = Vec::new();
        write_u64(&mut payload, 1);
        let digest = crate::types::hash_bytes(b"tensor-vm-peer-book-v1", &[&payload]);
        let mut bad_len = Vec::from(PEER_BOOK_MAGIC);
        bad_len.extend_from_slice(&payload);
        bad_len.extend_from_slice(&digest);
        assert_eq!(
            decode_peer_records(&bad_len),
            Err(TvmError::Storage("invalid peer book length"))
        );

        let a = address(b"duplicate-peer-book-a");
        let record = PeerRecord {
            address: a,
            score: 0,
            dropped_messages: 0,
            blocked_until_window: 0,
        };
        assert!(matches!(
            LocalNetwork::from_peer_records(
                Libp2pControlPlaneConfig::default(),
                vec![record.clone(), record]
            ),
            Err(TvmError::Storage("duplicate peer record"))
        ));
        assert!(matches!(
            LocalNetwork::from_peer_records(
                Libp2pControlPlaneConfig {
                    max_peer_count: 0,
                    ..Libp2pControlPlaneConfig::default()
                },
                vec![PeerRecord {
                    address: a,
                    score: 0,
                    dropped_messages: 0,
                    blocked_until_window: 0,
                }]
            ),
            Err(TvmError::Storage("peer book exceeds peer limit"))
        ));

        let mut truncated_u64_payload = Vec::new();
        truncated_u64_payload.extend_from_slice(&1_u64.to_le_bytes());
        truncated_u64_payload.extend_from_slice(&a);
        let truncated_u64_digest =
            crate::types::hash_bytes(b"tensor-vm-peer-book-v1", &[&truncated_u64_payload]);
        let mut truncated_u64 = Vec::from(PEER_BOOK_MAGIC);
        truncated_u64.extend_from_slice(&truncated_u64_payload);
        truncated_u64.extend_from_slice(&truncated_u64_digest);
        assert_eq!(
            decode_peer_records(&truncated_u64),
            Err(TvmError::Storage("invalid peer book length"))
        );

        assert_eq!(
            read_peer_u64(&[1, 2], &mut 0),
            Err(TvmError::Storage("truncated peer book u64"))
        );
        assert_eq!(
            read_peer_i64(&[1, 2], &mut 0),
            Err(TvmError::Storage("truncated peer book i64"))
        );
        assert_eq!(
            read_peer_hash(&[1, 2], &mut 0),
            Err(TvmError::Storage("truncated peer book address"))
        );
    }

    #[test]
    fn rejects_trailing_or_short_messages() {
        let mut encoded = encode_message(&P2pMessage::NewJob(hash_bytes(b"test", &[b"job"])));
        encoded.push(0);
        assert!(decode_message(&encoded).is_err());
        assert!(decode_message(&[1, 2, 3]).is_err());
    }

    #[test]
    fn rejects_malformed_payloads_and_unknown_peers() {
        let h = hash_bytes(b"test", &[b"malformed-p2p"]);
        assert_eq!(
            decode_message(&[]),
            Err(TvmError::InvalidReceipt("short p2p message"))
        );
        assert_eq!(
            decode_message(&[99]),
            Err(TvmError::InvalidReceipt("unknown p2p message tag"))
        );

        let mut short_hash = vec![5];
        short_hash.extend_from_slice(&h[..8]);
        assert_eq!(
            decode_message(&short_hash),
            Err(TvmError::InvalidReceipt("short p2p message"))
        );

        let mut truncated_bytes = vec![6];
        write_hash(&mut truncated_bytes, &h);
        write_u64(&mut truncated_bytes, 1);
        write_u64(&mut truncated_bytes, 4);
        truncated_bytes.extend_from_slice(&[1, 2]);
        assert_eq!(
            decode_message(&truncated_bytes),
            Err(TvmError::InvalidReceipt("short p2p message"))
        );

        let unknown = address(b"unknown-peer");
        let mut network = LocalNetwork::default();
        assert_eq!(
            network.recv(&unknown),
            Err(TvmError::InvalidReceipt("unknown peer"))
        );
        assert_eq!(
            network.inbox_len(&unknown),
            Err(TvmError::InvalidReceipt("unknown peer"))
        );
        assert_eq!(
            network.set_peer_score(&unknown, 7),
            Err(TvmError::InvalidReceipt("unknown peer"))
        );
        assert_eq!(
            network.try_broadcast(unknown, &P2pMessage::NewBlock(h)),
            Err(TvmError::InvalidReceipt("unknown peer"))
        );
    }

    #[test]
    fn p2p_frames_roundtrip_and_enforce_limits() {
        let h = hash_bytes(b"test", &[b"job"]);
        let config = P2pTransportConfig::default();
        let message = P2pMessage::NewJob(h);
        let frame = encode_frame(&message, &config).unwrap();
        assert_eq!(decode_frame(&frame, &config).unwrap(), message);

        let mut bad_magic = frame.clone();
        bad_magic[0] = 0;
        assert_eq!(
            decode_frame(&bad_magic, &config).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );

        let mut trailing = frame;
        trailing.push(0);
        assert_eq!(
            decode_frame(&trailing, &config).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );

        let limited = P2pTransportConfig {
            max_frame_bytes: 8,
            ..P2pTransportConfig::default()
        };
        let large = P2pMessage::ProgramResponse {
            program_hash: h,
            bytes: vec![0; 32],
        };
        assert_eq!(
            encode_frame(&large, &limited).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );

        let mut short = Vec::from(P2P_FRAME_MAGIC);
        short.extend_from_slice(&1_u32.to_le_bytes());
        short.push(255);
        assert_eq!(
            decode_frame(&short, &config).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
        assert_eq!(
            decode_frame(&[1, 2, 3], &config).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
        let mut oversized_header = Vec::from(P2P_FRAME_MAGIC);
        oversized_header.extend_from_slice(&9_u32.to_le_bytes());
        assert_eq!(
            decode_frame(&oversized_header, &limited)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn p2p_in_memory_frame_io_covers_transport_codec_edges() {
        let h = hash_bytes(b"test", &[b"in-memory-frame"]);
        let config = P2pTransportConfig::default();
        let message = P2pMessage::NewAttestation(h);
        let mut bytes = Vec::new();
        write_framed_message_to(&mut bytes, &message, &config).unwrap();
        assert_eq!(
            read_framed_message_from(&mut std::io::Cursor::new(&bytes), &config).unwrap(),
            message
        );

        let mut bad_magic = b"BAD!".to_vec();
        bad_magic.extend_from_slice(&0_u32.to_le_bytes());
        assert_eq!(
            read_framed_message_from(&mut std::io::Cursor::new(&bad_magic), &config)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidData
        );

        let mut oversized = Vec::from(P2P_FRAME_MAGIC);
        oversized.extend_from_slice(&9_u32.to_le_bytes());
        assert_eq!(
            read_framed_message_from(
                &mut std::io::Cursor::new(&oversized),
                &P2pTransportConfig {
                    max_frame_bytes: 8,
                    ..P2pTransportConfig::default()
                },
            )
            .unwrap_err()
            .kind(),
            std::io::ErrorKind::InvalidData
        );

        let mut invalid_payload = Vec::from(P2P_FRAME_MAGIC);
        invalid_payload.extend_from_slice(&1_u32.to_le_bytes());
        invalid_payload.push(255);
        assert_eq!(
            read_framed_message_from(&mut std::io::Cursor::new(&invalid_payload), &config)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidData
        );

        assert_eq!(
            read_framed_message_from(&mut std::io::Cursor::new([1, 2, 3]), &config)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn p2p_tcp_server_receives_framed_message() {
        use std::io::ErrorKind;

        let config = P2pTransportConfig::default();
        let server = match P2pTcpServer::bind("127.0.0.1:0", config.clone()) {
            Ok(server) => server,
            Err(error) if error.kind() == ErrorKind::PermissionDenied => return,
            Err(error) => panic!("failed to bind P2P TCP server: {error}"),
        };
        let addr = server.local_addr().unwrap();
        let h = hash_bytes(b"test", &[b"receipt"]);
        let message = P2pMessage::NewReceipt(h);
        let server_thread = std::thread::spawn(move || server.serve_n(1));

        send_framed_message(addr, &message, &config).unwrap();
        let received = server_thread.join().unwrap().unwrap();
        assert_eq!(received, vec![message]);
    }

    #[test]
    fn p2p_tcp_reader_rejects_malformed_socket_frames() {
        use std::io::ErrorKind;

        fn read_one_frame_from_client(
            raw: Vec<u8>,
            config: P2pTransportConfig,
        ) -> Option<ErrorKind> {
            let listener = match TcpListener::bind("127.0.0.1:0") {
                Ok(listener) => listener,
                Err(error) if error.kind() == ErrorKind::PermissionDenied => {
                    return None;
                }
                Err(error) => panic!("failed to bind malformed frame listener: {error}"),
            };
            let addr = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                read_framed_message(&mut stream, &config)
                    .unwrap_err()
                    .kind()
            });
            let mut client = TcpStream::connect(addr).unwrap();
            client.write_all(&raw).unwrap();
            Some(server.join().unwrap())
        }

        let config = P2pTransportConfig::default();
        let mut bad_magic = b"BAD!".to_vec();
        bad_magic.extend_from_slice(&0_u32.to_le_bytes());
        let Some(bad_magic_error) = read_one_frame_from_client(bad_magic, config.clone()) else {
            return;
        };
        assert_eq!(bad_magic_error, ErrorKind::InvalidData);

        let mut oversized = Vec::from(P2P_FRAME_MAGIC);
        oversized.extend_from_slice(&9_u32.to_le_bytes());
        let Some(oversized_error) = read_one_frame_from_client(
            oversized,
            P2pTransportConfig {
                max_frame_bytes: 8,
                ..P2pTransportConfig::default()
            },
        ) else {
            return;
        };
        assert_eq!(oversized_error, ErrorKind::InvalidData);

        let mut invalid_payload = Vec::from(P2P_FRAME_MAGIC);
        invalid_payload.extend_from_slice(&1_u32.to_le_bytes());
        invalid_payload.push(255);
        let Some(invalid_payload_error) = read_one_frame_from_client(invalid_payload, config)
        else {
            return;
        };
        assert_eq!(invalid_payload_error, ErrorKind::InvalidData);
    }
}
