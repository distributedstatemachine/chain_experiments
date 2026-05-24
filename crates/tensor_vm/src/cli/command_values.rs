use super::arguments::{parse_hash_argument, parse_hash_list_argument};
use crate::types::Hash;
use libp2p::Multiaddr;
use std::net::SocketAddr;

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
