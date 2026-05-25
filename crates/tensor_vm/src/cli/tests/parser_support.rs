use super::{
    AddressArg, DataDirArgs, HashArg, IdentitySeedArgs, ManifestSignerArgs, MinerDeviceArg,
    NodeRuntimeArgs, P2pListenArgs, PublicationBundleArgs, RecordArtifactLocatorArgs,
    RecordFileArgs, RecordRootArgs, RecordRootsArgs, RoleNodeArgs, RoleRuntimeArgs, RoleWalletArgs,
    RunWindowContextArgs,
};
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

pub(super) fn role_wallet_args(wallet: &str) -> RoleWalletArgs {
    RoleWalletArgs {
        wallet: path(wallet),
    }
}

pub(super) fn role_node_args(node: &str) -> RoleNodeArgs {
    RoleNodeArgs {
        node: multiaddr(node),
    }
}

pub(super) fn p2p_listen_args(p2p_listen: &str) -> P2pListenArgs {
    P2pListenArgs {
        p2p_listen: multiaddr(p2p_listen),
    }
}

pub(super) fn hash_arg(value: [u8; 32]) -> HashArg {
    HashArg::new(value)
}

pub(super) fn identity_seed_args(identity_seed: Option<[u8; 32]>) -> IdentitySeedArgs {
    IdentitySeedArgs {
        identity_seed: identity_seed.map(HashArg::new),
    }
}

pub(super) fn publication_bundle_args(
    bundle_id: [u8; 32],
    public_uri: &str,
) -> PublicationBundleArgs {
    PublicationBundleArgs {
        bundle_id: hash_arg(bundle_id),
        public_uri: public_uri.to_owned(),
    }
}

pub(super) fn run_window_context_args(
    bundle_id: [u8; 32],
    manifest_signer: [u8; 32],
) -> RunWindowContextArgs {
    RunWindowContextArgs {
        bundle_id: hash_arg(bundle_id),
        signer: manifest_signer_args(manifest_signer),
    }
}

pub(super) fn record_artifact_locator_args(artifact_uri: &str) -> RecordArtifactLocatorArgs {
    RecordArtifactLocatorArgs {
        artifact_uri: artifact_uri.to_owned(),
    }
}

pub(super) fn record_file_args(record_file: &str) -> RecordFileArgs {
    RecordFileArgs {
        record_file: path(record_file),
    }
}

pub(super) fn record_root_args(record_root: [u8; 32], record_count: u64) -> RecordRootArgs {
    RecordRootArgs {
        record_root: hash_arg(record_root),
        record_count,
    }
}

pub(super) fn record_roots_args(record_roots: Vec<[u8; 32]>) -> RecordRootsArgs {
    RecordRootsArgs {
        record_roots: record_roots.into_iter().map(HashArg::new).collect(),
    }
}

pub(super) fn address_arg(value: [u8; 32]) -> AddressArg {
    AddressArg::new(value)
}

pub(super) fn manifest_signer_args(manifest_signer: [u8; 32]) -> ManifestSignerArgs {
    ManifestSignerArgs {
        manifest_signer: address_arg(manifest_signer),
    }
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
        p2p_listen: p2p_listen_args(p2p_listen),
        data_dir: data_dir_args(data_dir),
        identity_seed: identity_seed_args(identity_seed),
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
        node: role_node_args(node),
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
