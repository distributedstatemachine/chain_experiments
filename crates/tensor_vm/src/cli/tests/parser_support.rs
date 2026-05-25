use super::{
    AddressArg, DataDirArgs, EvidenceBundleIdArgs, HashArg, IdentitySeedArgs, ManifestSignerArgs,
    MinerDeviceArg, NodeRuntimeArgs, NodeServeArgs, OperatorIdArgs, P2pListenArgs,
    PublicationBundleArgs, RecordArtifactLocatorArgs, RecordFileArgs, RecordRootArgs,
    RecordRootsArgs, RoleNodeArgs, RoleRuntimeArgs, RoleWalletArgs, RunWindowContextArgs,
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
    DataDirArgs::new(path(data_dir))
}

pub(super) fn role_wallet_args(wallet: &str) -> RoleWalletArgs {
    RoleWalletArgs::new(path(wallet))
}

pub(super) fn role_node_args(node: &str) -> RoleNodeArgs {
    RoleNodeArgs::new(multiaddr(node))
}

pub(super) fn p2p_listen_args(p2p_listen: &str) -> P2pListenArgs {
    P2pListenArgs::new(multiaddr(p2p_listen))
}

pub(super) fn hash_arg(value: [u8; 32]) -> HashArg {
    HashArg::new(value)
}

pub(super) fn identity_seed_args(identity_seed: Option<[u8; 32]>) -> IdentitySeedArgs {
    IdentitySeedArgs::new(identity_seed.map(HashArg::new))
}

pub(super) fn evidence_bundle_id_args(bundle_id: [u8; 32]) -> EvidenceBundleIdArgs {
    EvidenceBundleIdArgs::new(bundle_id)
}

pub(super) fn operator_id_args(operator_id: [u8; 32]) -> OperatorIdArgs {
    OperatorIdArgs::new(operator_id)
}

pub(super) fn publication_bundle_args(
    bundle_id: [u8; 32],
    public_uri: &str,
) -> PublicationBundleArgs {
    PublicationBundleArgs::new(evidence_bundle_id_args(bundle_id), public_uri)
}

pub(super) fn run_window_context_args(
    bundle_id: [u8; 32],
    manifest_signer: [u8; 32],
) -> RunWindowContextArgs {
    RunWindowContextArgs::new(
        evidence_bundle_id_args(bundle_id),
        manifest_signer_args(manifest_signer),
    )
}

pub(super) fn record_artifact_locator_args(artifact_uri: &str) -> RecordArtifactLocatorArgs {
    RecordArtifactLocatorArgs::new(artifact_uri)
}

pub(super) fn record_file_args(record_file: &str) -> RecordFileArgs {
    RecordFileArgs::new(path(record_file))
}

pub(super) fn record_root_args(record_root: [u8; 32], record_count: u64) -> RecordRootArgs {
    RecordRootArgs::new(record_root, record_count)
}

pub(super) fn record_roots_args(record_roots: Vec<[u8; 32]>) -> RecordRootsArgs {
    RecordRootsArgs::new(record_roots)
}

pub(super) fn address_arg(value: [u8; 32]) -> AddressArg {
    AddressArg::new(value)
}

pub(super) fn manifest_signer_args(manifest_signer: [u8; 32]) -> ManifestSignerArgs {
    ManifestSignerArgs::new(manifest_signer)
}

pub(super) fn node_runtime_args(
    listen: &str,
    p2p_listen: &str,
    data_dir: &str,
    identity_seed: Option<[u8; 32]>,
    auth_token: &str,
    max_requests: usize,
) -> NodeRuntimeArgs {
    NodeRuntimeArgs::new(
        socket_addr(listen),
        p2p_listen_args(p2p_listen),
        data_dir_args(data_dir),
        identity_seed_args(identity_seed),
        auth_token.to_owned(),
        max_requests,
    )
}

pub(super) fn node_serve_args(
    listen: &str,
    p2p_listen: &str,
    data_dir: &str,
    identity_seed: Option<[u8; 32]>,
    auth_token: &str,
    max_requests: usize,
) -> NodeServeArgs {
    NodeServeArgs::new(node_runtime_args(
        listen,
        p2p_listen,
        data_dir,
        identity_seed,
        auth_token,
        max_requests,
    ))
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
    RoleRuntimeArgs::new(
        role_node_args(node),
        node_runtime_args(
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        ),
    )
}
