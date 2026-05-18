use crate::api::P2pMessage;
use crate::error::{Result as TvmResult, TvmError};
use crate::types::{Hash, hash_bytes};
use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId, StreamProtocol, Swarm};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const LIBP2P_PROTOCOL_PREFIX: &str = "/tensorchain/1";
const PEER_BOOK_MAGIC: &[u8] = b"TENSORVM_LIBP2P_PEER_BOOK_V1\n";
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
            "rust-libp2p is the default TensorVM P2P runtime dependency",
            "gossipsub carries block, job, receipt, attestation, and peer announcements",
            "identify advertises TensorVM protocol support to connected peers",
            "request-response streams carry tensor rows, tensor chunks, and program fetches",
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
    pub listen_addresses: Vec<String>,
    pub bootstrap_addresses: Vec<String>,
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
                RequestResponseProtocol::Program,
            ],
            listen_addresses: Vec::new(),
            bootstrap_addresses: Vec::new(),
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
    pub request_response: libp2p::request_response::json::Behaviour<P2pMessage, P2pMessage>,
}

pub struct TensorVmLibp2pNode {
    pub peer_id: PeerId,
    pub swarm: Swarm<TensorVmNetworkBehaviour>,
    pub identify_protocol: String,
    pub subscribed_topics: Vec<String>,
    pub request_response_protocols: Vec<String>,
}

pub fn build_libp2p_node(config: &Libp2pControlPlaneConfig) -> TvmResult<TensorVmLibp2pNode> {
    let keypair = libp2p::identity::Keypair::generate_ed25519();
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
    let request_protocols = config
        .request_response_protocols
        .iter()
        .map(|protocol| {
            Ok((
                request_response_stream_protocol(*protocol)?,
                libp2p::request_response::ProtocolSupport::Full,
            ))
        })
        .collect::<TvmResult<Vec<_>>>()?;
    let request_response = libp2p::request_response::json::Behaviour::new(
        request_protocols,
        libp2p::request_response::Config::default()
            .with_request_timeout(Duration::from_secs(config.request_timeout_seconds))
            .with_max_concurrent_streams(config.max_concurrent_request_streams),
    );

    Ok(TensorVmNetworkBehaviour {
        gossipsub,
        identify,
        kademlia,
        request_response,
    })
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

pub fn gossipsub_ident_topic(topic: GossipTopic) -> libp2p::gossipsub::IdentTopic {
    libp2p::gossipsub::IdentTopic::new(topic.as_str())
}

pub fn request_response_stream_protocol(
    protocol: RequestResponseProtocol,
) -> TvmResult<StreamProtocol> {
    StreamProtocol::try_from_owned(protocol.as_str().to_owned())
        .map_err(|_| TvmError::InvalidReceipt("invalid libp2p stream protocol"))
}

pub fn encode_gossipsub_message(
    message: &P2pMessage,
) -> TvmResult<(libp2p::gossipsub::IdentTopic, Vec<u8>)> {
    let topic = gossip_topic_for_message(message).ok_or(TvmError::InvalidReceipt(
        "message is not a gossipsub announcement",
    ))?;
    Ok((gossipsub_ident_topic(topic), encode_message(message)))
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

pub fn decode_message(input: &[u8]) -> TvmResult<P2pMessage> {
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
pub struct PeerRecord {
    pub peer_id: String,
    pub address: String,
}

impl PeerRecord {
    pub fn new(peer_id: PeerId, address: Multiaddr) -> Self {
        Self {
            peer_id: peer_id.to_string(),
            address: address.to_string(),
        }
    }

    pub fn peer_id(&self) -> TvmResult<PeerId> {
        self.peer_id
            .parse()
            .map_err(|_| TvmError::Storage("invalid peer id"))
    }

    pub fn multiaddr(&self) -> TvmResult<Multiaddr> {
        parse_multiaddr(&self.address)
    }
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

    pub fn save_records(&self, records: &[PeerRecord]) -> TvmResult<()> {
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

    pub fn load_records(&self) -> TvmResult<Vec<PeerRecord>> {
        let bytes =
            fs::read(&self.path).map_err(|_| TvmError::Storage("failed to read peer book"))?;
        decode_peer_records(&bytes)
    }

    pub fn load_bootstrap_addresses(&self) -> TvmResult<Vec<String>> {
        self.load_records()
            .map(|records| records.into_iter().map(|record| record.address).collect())
    }
}

fn parse_multiaddr(address: &str) -> TvmResult<Multiaddr> {
    address
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid libp2p multiaddr"))
}

fn bootstrap_peer_address(address: &Multiaddr) -> Option<(PeerId, Multiaddr)> {
    let mut peer_address = address.clone();
    match peer_address.pop() {
        Some(Protocol::P2p(peer_id)) => Some((peer_id, peer_address)),
        _ => None,
    }
}

fn encode_peer_records(records: &[PeerRecord]) -> Vec<u8> {
    let mut payload = Vec::new();
    write_u64(&mut payload, records.len() as u64);
    for record in records {
        write_string(&mut payload, &record.peer_id);
        write_string(&mut payload, &record.address);
    }
    let digest = hash_bytes(b"tensor-vm-libp2p-peer-book-v1", &[&payload]);
    let mut encoded =
        Vec::with_capacity(PEER_BOOK_MAGIC.len() + payload.len() + PEER_BOOK_DIGEST_LEN);
    encoded.extend_from_slice(PEER_BOOK_MAGIC);
    encoded.extend_from_slice(&payload);
    encoded.extend_from_slice(&digest);
    encoded
}

fn decode_peer_records(bytes: &[u8]) -> TvmResult<Vec<PeerRecord>> {
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
    let expected_digest = hash_bytes(b"tensor-vm-libp2p-peer-book-v1", &[payload]);
    if bytes[payload_end..] != expected_digest {
        return Err(TvmError::Storage("peer book checksum mismatch"));
    }

    let mut offset = 0;
    let record_count = read_peer_u64(payload, &mut offset)? as usize;
    let mut records = Vec::with_capacity(record_count);
    for _ in 0..record_count {
        let peer_id = read_peer_string(payload, &mut offset)?;
        let address = read_peer_string(payload, &mut offset)?;
        let record = PeerRecord { peer_id, address };
        record.peer_id()?;
        record.multiaddr()?;
        records.push(record);
    }
    if offset != payload.len() {
        return Err(TvmError::Storage("trailing peer book bytes"));
    }
    Ok(records)
}

fn read_peer_u64(bytes: &[u8], offset: &mut usize) -> TvmResult<u64> {
    if bytes.len().saturating_sub(*offset) < 8 {
        return Err(TvmError::Storage("truncated peer book u64"));
    }
    let mut out = [0_u8; 8];
    out.copy_from_slice(&bytes[*offset..*offset + 8]);
    *offset += 8;
    Ok(u64::from_le_bytes(out))
}

fn read_peer_string(bytes: &[u8], offset: &mut usize) -> TvmResult<String> {
    let len = read_peer_u64(bytes, offset)? as usize;
    let end = offset
        .checked_add(len)
        .ok_or(TvmError::Storage("peer book string length overflow"))?;
    let Some(raw) = bytes.get(*offset..end) else {
        return Err(TvmError::Storage("truncated peer book string"));
    };
    *offset = end;
    String::from_utf8(raw.to_vec()).map_err(|_| TvmError::Storage("invalid peer book utf8"))
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

fn write_string(out: &mut Vec<u8>, value: &str) {
    write_u64(out, value.len() as u64);
    out.extend_from_slice(value.as_bytes());
}

struct Reader<'a> {
    input: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    fn read_u8(&mut self) -> TvmResult<u8> {
        let Some(byte) = self.input.get(self.offset).copied() else {
            return Err(TvmError::InvalidReceipt("short p2p message"));
        };
        self.offset += 1;
        Ok(byte)
    }

    fn read_u64(&mut self) -> TvmResult<u64> {
        let bytes = self.read_exact(8)?;
        let mut out = [0_u8; 8];
        out.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(out))
    }

    fn read_hash(&mut self) -> TvmResult<Hash> {
        let bytes = self.read_exact(32)?;
        let mut out = [0_u8; 32];
        out.copy_from_slice(bytes);
        Ok(out)
    }

    fn read_bytes(&mut self) -> TvmResult<Vec<u8>> {
        let len = self.read_u64()? as usize;
        Ok(self.read_exact(len)?.to_vec())
    }

    fn read_exact(&mut self, len: usize) -> TvmResult<&'a [u8]> {
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
                .any(|reason| reason.contains("rust-libp2p"))
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
        assert_eq!(
            gossipsub_ident_topic(GossipTopic::Blocks).to_string(),
            "/tensorchain/1/blocks"
        );
        assert_eq!(
            request_response_stream_protocol(RequestResponseProtocol::TensorRow)
                .unwrap()
                .to_string(),
            "/tensorchain/1/tensor/row"
        );
    }

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
        assert_eq!(node.request_response_protocols.len(), 3);
        assert!(
            node.request_response_protocols
                .contains(&"/tensorchain/1/tensor/chunk".to_owned())
        );
        assert_eq!(node.identify_protocol, "/tensorchain/1/identify");
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
    fn gossipsub_encoding_rejects_request_response_messages() {
        let h = hash_bytes(b"test", &[b"gossipsub-encode"]);
        let (topic, payload) = encode_gossipsub_message(&P2pMessage::NewBlock(h)).unwrap();
        assert_eq!(topic.to_string(), "/tensorchain/1/blocks");
        assert_eq!(decode_message(&payload).unwrap(), P2pMessage::NewBlock(h));
        match encode_gossipsub_message(&P2pMessage::RequestProgram(h)) {
            Err(error) => assert_eq!(
                error,
                TvmError::InvalidReceipt("message is not a gossipsub announcement")
            ),
            Ok(_) => panic!("request-response message encoded as gossipsub"),
        }
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
            vec![address_a.to_string(), address_b.to_string()]
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
        write_u64(&mut trailing_payload, 0);
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
        write_u64(&mut bad_record_payload, 1);
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
        write_u64(&mut truncated_string_payload, 1);
        write_u64(&mut truncated_string_payload, 10);
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
        write_u64(&mut bad_addr_payload, 1);
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

    #[test]
    fn rejects_trailing_or_short_messages() {
        let mut encoded = encode_message(&P2pMessage::NewJob(hash_bytes(b"test", &[b"job"])));
        encoded.push(0);
        assert!(decode_message(&encoded).is_err());
        assert!(decode_message(&[1, 2, 3]).is_err());
    }

    #[test]
    fn rejects_malformed_payloads() {
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
    }
}
