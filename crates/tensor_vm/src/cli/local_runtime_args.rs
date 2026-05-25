use super::value_types::HashArg;
use crate::types::Hash;
use clap::{Args, ValueHint};
use libp2p::Multiaddr;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

pub(super) const DEFAULT_DATA_DIR: &str = ".tensorvm";
const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:8545";
const DEFAULT_P2P_LISTEN_ADDR: &str = "/ip4/127.0.0.1/tcp/4001";
const DEFAULT_MAX_REQUESTS: usize = 0;

fn default_listen_addr() -> SocketAddr {
    DEFAULT_LISTEN_ADDR
        .parse()
        .expect("default service listen address must be a socket address")
}

pub(super) fn default_p2p_listen_addr() -> Multiaddr {
    DEFAULT_P2P_LISTEN_ADDR
        .parse()
        .expect("default p2p listen address must be a multiaddr")
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NodeRuntimeArgs {
    #[arg(
        long,
        env = "TVMD_LISTEN",
        default_value_t = default_listen_addr(),
        value_name = "ADDR",
        help = "RPC and service listen address."
    )]
    listen: SocketAddr,
    #[command(flatten)]
    p2p_listen: P2pListenArgs,
    #[command(flatten)]
    data_dir: DataDirArgs,
    #[command(flatten)]
    identity_seed: IdentitySeedArgs,
    #[arg(
        long,
        env = "TVMD_AUTH_TOKEN",
        value_name = "TOKEN",
        hide_env_values = true,
        help = "Bearer token required by local RPC, explorer, faucet, and telemetry endpoints."
    )]
    auth_token: String,
    #[arg(
        long,
        env = "TVMD_MAX_REQUESTS",
        default_value_t = DEFAULT_MAX_REQUESTS,
        value_name = "N",
        help = "Maximum RPC requests before the service exits; 0 keeps serving."
    )]
    max_requests: usize,
}

impl NodeRuntimeArgs {
    #[cfg(test)]
    pub(crate) fn new(
        listen: SocketAddr,
        p2p_listen: P2pListenArgs,
        data_dir: DataDirArgs,
        identity_seed: IdentitySeedArgs,
        auth_token: String,
        max_requests: usize,
    ) -> Self {
        Self {
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        }
    }

    pub fn listen(&self) -> &SocketAddr {
        &self.listen
    }

    pub fn p2p_listen(&self) -> &P2pListenArgs {
        &self.p2p_listen
    }

    pub fn data_dir(&self) -> &DataDirArgs {
        &self.data_dir
    }

    pub fn identity_seed(&self) -> Option<Hash> {
        self.identity_seed.hash()
    }

    pub fn auth_token(&self) -> &str {
        &self.auth_token
    }

    pub fn max_requests(&self) -> usize {
        self.max_requests
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct DataDirArgs {
    #[arg(
        long,
        env = "TVMD_DATA_DIR",
        default_value = DEFAULT_DATA_DIR,
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        help = "Node store directory."
    )]
    data_dir: PathBuf,
}

impl DataDirArgs {
    #[cfg(test)]
    pub(crate) fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub fn path(&self) -> &Path {
        &self.data_dir
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct P2pListenArgs {
    #[arg(
        long,
        env = "TVMD_P2P_LISTEN",
        default_value_t = default_p2p_listen_addr(),
        value_name = "MULTIADDR",
        help = "libp2p listen multiaddress."
    )]
    p2p_listen: Multiaddr,
}

impl P2pListenArgs {
    #[cfg(test)]
    pub(crate) fn new(p2p_listen: Multiaddr) -> Self {
        Self { p2p_listen }
    }

    pub fn multiaddr(&self) -> &Multiaddr {
        &self.p2p_listen
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Args)]
pub struct IdentitySeedArgs {
    #[arg(
        long,
        value_name = "HEX",
        help = "Deterministic 32-byte seed for the libp2p identity."
    )]
    identity_seed: Option<HashArg>,
}

impl IdentitySeedArgs {
    #[cfg(test)]
    pub(crate) fn new(identity_seed: Option<HashArg>) -> Self {
        Self { identity_seed }
    }

    pub fn hash(&self) -> Option<Hash> {
        self.identity_seed.map(HashArg::into_hash)
    }
}
