use super::{AddressArg, DataDirArgs, HashArg, MinerDeviceArg, NodeRuntimeArgs, RoleRuntimeArgs};
use std::net::SocketAddr;
use std::path::PathBuf;

pub(super) fn path(value: &str) -> PathBuf {
    value.into()
}

pub(super) fn multiaddr(value: &str) -> libp2p::Multiaddr {
    value.parse().expect("test multiaddr must parse")
}

fn socket_addr(value: &str) -> SocketAddr {
    value.parse().expect("test socket address must parse")
}

pub(super) fn miner_device(value: &str) -> MinerDeviceArg {
    value.parse().expect("test miner device must parse")
}

pub(super) fn data_dir_args(data_dir: &str) -> DataDirArgs {
    DataDirArgs {
        data_dir: path(data_dir),
    }
}

pub(super) fn hash_arg(value: [u8; 32]) -> HashArg {
    HashArg::new(value)
}

pub(super) fn address_arg(value: [u8; 32]) -> AddressArg {
    AddressArg::new(value)
}

pub(super) fn node_runtime_args(
    listen: &str,
    p2p_listen: &str,
    data_dir: &str,
    identity_seed: Option<[u8; 32]>,
    auth_token: &str,
    max_requests: usize,
) -> NodeRuntimeArgs {
    NodeRuntimeArgs {
        listen: socket_addr(listen),
        p2p_listen: multiaddr(p2p_listen),
        data_dir: path(data_dir),
        identity_seed: identity_seed.map(HashArg::new),
        auth_token: auth_token.to_owned(),
        max_requests,
    }
}

pub(super) fn role_runtime_args(
    node: &str,
    listen: &str,
    p2p_listen: &str,
    data_dir: &str,
    identity_seed: Option<[u8; 32]>,
    auth_token: &str,
    max_requests: usize,
) -> RoleRuntimeArgs {
    RoleRuntimeArgs {
        node: multiaddr(node),
        node_runtime: node_runtime_args(
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        ),
    }
}
