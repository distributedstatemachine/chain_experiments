use super::arguments::{parse_hash_argument, parse_hash_list_argument};
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use crate::types::Hash;
use clap::ValueEnum;
use libp2p::Multiaddr;
use std::net::SocketAddr;

pub(super) const DEFAULT_DATA_DIR: &str = ".tensorvm";
pub(super) const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:8545";
pub(super) const DEFAULT_P2P_LISTEN_ADDR: &str = "/ip4/127.0.0.1/tcp/4001";
pub(super) const DEFAULT_MAX_REQUESTS: usize = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PublicServiceKindArg {
    Rpc,
    Explorer,
    Faucet,
    Telemetry,
}

impl From<PublicServiceKindArg> for PublicServiceKind {
    fn from(kind: PublicServiceKindArg) -> Self {
        match kind {
            PublicServiceKindArg::Rpc => Self::Rpc,
            PublicServiceKindArg::Explorer => Self::Explorer,
            PublicServiceKindArg::Faucet => Self::Faucet,
            PublicServiceKindArg::Telemetry => Self::Telemetry,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PublicNodeRoleArg {
    Miner,
    Validator,
}

impl From<PublicNodeRoleArg> for PublicNodeRole {
    fn from(role: PublicNodeRoleArg) -> Self {
        match role {
            PublicNodeRoleArg::Miner => Self::Miner,
            PublicNodeRoleArg::Validator => Self::Validator,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PublicEvidenceRecordKindArg {
    BlockHistory,
    FinalityHistory,
    NetworkRuntime,
    DataAvailability,
    InvalidWork,
    RewardSettlement,
}

impl From<PublicEvidenceRecordKindArg> for PublicEvidenceRecordKind {
    fn from(kind: PublicEvidenceRecordKindArg) -> Self {
        match kind {
            PublicEvidenceRecordKindArg::BlockHistory => Self::BlockHistory,
            PublicEvidenceRecordKindArg::FinalityHistory => Self::FinalityHistory,
            PublicEvidenceRecordKindArg::NetworkRuntime => Self::NetworkRuntimeObservations,
            PublicEvidenceRecordKindArg::DataAvailability => Self::DataAvailabilityMeasurements,
            PublicEvidenceRecordKindArg::InvalidWork => Self::InvalidWorkRejections,
            PublicEvidenceRecordKindArg::RewardSettlement => Self::RewardSettlements,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HashList(pub Vec<Hash>);

pub(super) fn parse_hash_value(value: &str) -> std::result::Result<Hash, String> {
    parse_hash_argument(value).map_err(|error| error.to_string())
}

pub(super) fn parse_hash_list_value(value: &str) -> std::result::Result<HashList, String> {
    parse_hash_list_argument(value)
        .map(HashList)
        .map_err(|error| error.to_string())
}

pub(super) fn parse_socket_addr_value(value: &str) -> std::result::Result<String, String> {
    value
        .parse::<SocketAddr>()
        .map(|_| value.to_owned())
        .map_err(|_| "invalid socket address; expected host:port".to_owned())
}

pub(super) fn parse_multiaddr_value(value: &str) -> std::result::Result<String, String> {
    value
        .parse::<Multiaddr>()
        .map(|_| value.to_owned())
        .map_err(|_| "invalid libp2p multiaddr".to_owned())
}
