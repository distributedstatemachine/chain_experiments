use crate::error::{Result as TvmResult, TvmError};
use crate::types::hash_bytes;
use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) const PEER_BOOK_MAGIC: &[u8] = b"TENSORVM_LIBP2P_PEER_BOOK_V1\n";
const PEER_BOOK_DIGEST_LEN: usize = 32;

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

    pub fn from_strings(peer_id: &str, address: &str) -> TvmResult<Self> {
        let record = Self {
            peer_id: peer_id.to_owned(),
            address: address.to_owned(),
        };
        record.bootstrap_multiaddr()?;
        Ok(record)
    }

    pub fn peer_id(&self) -> TvmResult<PeerId> {
        self.peer_id
            .parse()
            .map_err(|_| TvmError::Storage("invalid peer id"))
    }

    pub fn multiaddr(&self) -> TvmResult<Multiaddr> {
        parse_multiaddr(&self.address)
    }

    pub fn bootstrap_multiaddr(&self) -> TvmResult<Multiaddr> {
        let peer_id = self.peer_id()?;
        let mut address = self.multiaddr()?;
        if !multiaddr_has_nonzero_tcp(&address) {
            return Err(TvmError::Storage("peer book address missing tcp port"));
        }
        if let Some((embedded_peer_id, _)) = bootstrap_peer_address(&address) {
            if embedded_peer_id != peer_id {
                return Err(TvmError::Storage("peer book address peer id mismatch"));
            }
            return Ok(address);
        }
        address.push(Protocol::P2p(peer_id));
        Ok(address)
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

    pub fn upsert_record(&self, record: PeerRecord) -> TvmResult<Vec<PeerRecord>> {
        let mut records = if self.path.exists() {
            self.load_records()?
        } else {
            Vec::new()
        };
        if let Some(existing) = records
            .iter_mut()
            .find(|existing| existing.peer_id == record.peer_id)
        {
            *existing = record;
        } else {
            records.push(record);
        }
        self.save_records(&records)?;
        Ok(records)
    }

    pub fn load_records(&self) -> TvmResult<Vec<PeerRecord>> {
        let bytes =
            fs::read(&self.path).map_err(|_| TvmError::Storage("failed to read peer book"))?;
        decode_peer_records(&bytes)
    }

    pub fn load_bootstrap_addresses(&self) -> TvmResult<Vec<String>> {
        self.load_records()?
            .into_iter()
            .map(|record| Ok(record.bootstrap_multiaddr()?.to_string()))
            .collect()
    }
}

pub(super) fn parse_multiaddr(address: &str) -> TvmResult<Multiaddr> {
    address
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid libp2p multiaddr"))
}

pub(super) fn bootstrap_peer_address(address: &Multiaddr) -> Option<(PeerId, Multiaddr)> {
    let mut peer_address = address.clone();
    match peer_address.pop() {
        Some(Protocol::P2p(peer_id)) => Some((peer_id, peer_address)),
        _ => None,
    }
}

fn multiaddr_has_nonzero_tcp(address: &Multiaddr) -> bool {
    address
        .iter()
        .any(|protocol| matches!(protocol, Protocol::Tcp(port) if port != 0))
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

pub(super) fn decode_peer_records(bytes: &[u8]) -> TvmResult<Vec<PeerRecord>> {
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

pub(super) fn read_peer_u64(bytes: &[u8], offset: &mut usize) -> TvmResult<u64> {
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

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(super) fn write_string(out: &mut Vec<u8>, value: &str) {
    write_u64(out, value.len() as u64);
    out.extend_from_slice(value.as_bytes());
}
