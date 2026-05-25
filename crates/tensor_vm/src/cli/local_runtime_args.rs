use super::value_types::HashArg;
use crate::types::Hash;
use clap::{Args, ValueHint};
use libp2p::Multiaddr;
use std::net::SocketAddr;
use std::path::PathBuf;

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
pub(crate) struct NodeRuntimeArgs {
    #[arg(
        long,
        env = "TVMD_LISTEN",
        default_value_t = default_listen_addr(),
        value_name = "ADDR",
        help = "RPC and service listen address."
    )]
    pub(crate) listen: SocketAddr,
    #[command(flatten)]
    pub(crate) p2p_listen: P2pListenArgs,
    #[command(flatten)]
    pub(crate) data_dir: DataDirArgs,
    #[command(flatten)]
    pub(crate) identity_seed: IdentitySeedArgs,
    #[arg(
        long,
        env = "TVMD_AUTH_TOKEN",
        value_name = "TOKEN",
        hide_env_values = true,
        help = "Bearer token required by local RPC, explorer, faucet, and telemetry endpoints."
    )]
    pub(crate) auth_token: String,
    #[arg(
        long,
        env = "TVMD_MAX_REQUESTS",
        default_value_t = DEFAULT_MAX_REQUESTS,
        value_name = "N",
        help = "Maximum RPC requests before the service exits; 0 keeps serving."
    )]
    pub(crate) max_requests: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct DataDirArgs {
    #[arg(
        long,
        env = "TVMD_DATA_DIR",
        default_value = DEFAULT_DATA_DIR,
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        help = "Node store directory."
    )]
    pub(crate) data_dir: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct P2pListenArgs {
    #[arg(
        long,
        env = "TVMD_P2P_LISTEN",
        default_value_t = default_p2p_listen_addr(),
        value_name = "MULTIADDR",
        help = "libp2p listen multiaddress."
    )]
    pub(crate) p2p_listen: Multiaddr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Args)]
pub(crate) struct IdentitySeedArgs {
    #[arg(
        long,
        value_name = "HEX",
        help = "Deterministic 32-byte seed for the libp2p identity."
    )]
    pub(crate) identity_seed: Option<HashArg>,
}

impl IdentitySeedArgs {
    pub(crate) fn hash(&self) -> Option<Hash> {
        self.identity_seed.map(HashArg::into_hash)
    }
}
