use super::{
    runtime_config::ServiceRuntimeConfig, runtime_status::RuntimeP2pReport,
    shared::p2p_identity_report,
};
use tensor_vm::{
    Faucet, Libp2pControlPlaneConfig, NodeStore, RpcGateway, RpcHttpServer, RpcNode, RpcPolicy,
    TensorVmLibp2pService, spawn_libp2p_service,
};

pub(super) struct RuntimeServices {
    pub(super) store: NodeStore,
    pub(super) server: RpcHttpServer,
    pub(super) p2p_service: TensorVmLibp2pService,
    pub(super) p2p_metadata: RuntimeP2pMetadata,
}

pub(super) struct RuntimeP2pMetadata {
    peer_id: String,
    topics: usize,
    request_response_protocols: usize,
    bootstrap_peer_count: usize,
    identity: String,
    max_transmit_bytes: usize,
    request_timeout_seconds: u64,
    max_concurrent_streams: usize,
    idle_timeout_seconds: u64,
}

impl RuntimeP2pMetadata {
    pub(super) fn report(&self) -> RuntimeP2pReport<'_> {
        RuntimeP2pReport {
            peer_id: &self.peer_id,
            topics: self.topics,
            request_response_protocols: self.request_response_protocols,
            bootstrap_peer_count: self.bootstrap_peer_count,
            identity: &self.identity,
            max_transmit_bytes: self.max_transmit_bytes,
            request_timeout_seconds: self.request_timeout_seconds,
            max_concurrent_streams: self.max_concurrent_streams,
            idle_timeout_seconds: self.idle_timeout_seconds,
        }
    }
}

pub(super) fn start_runtime_services(
    config: &ServiceRuntimeConfig,
) -> std::result::Result<RuntimeServices, String> {
    let network = &config.node.network;
    let store = NodeStore::open(config.node.data_dir());
    let chain = store.load_chain().map_err(|error| {
        format!(
            "failed to load node store {}: {error}",
            config.node.data_dir().display()
        )
    })?;
    let bootstrap_addresses = if store.peer_book_store().path().exists() {
        store
            .peer_book_store()
            .load_bootstrap_addresses()
            .map_err(|error| {
                format!(
                    "failed to load libp2p peer book {}: {error}",
                    config.node.data_dir().display()
                )
            })?
    } else {
        Vec::new()
    };
    let bootstrap_peer_count = bootstrap_addresses.len();
    let p2p_config = Libp2pControlPlaneConfig {
        listen_addresses: vec![network.p2p_listen.clone()],
        bootstrap_addresses,
        identity_seed: network.identity_seed,
        ..Libp2pControlPlaneConfig::default()
    };
    let max_transmit_bytes = p2p_config.max_gossipsub_transmit_bytes;
    let request_timeout_seconds = p2p_config.request_timeout_seconds;
    let max_concurrent_streams = p2p_config.max_concurrent_request_streams;
    let idle_timeout_seconds = p2p_config.idle_connection_timeout_seconds;
    let p2p_service = spawn_libp2p_service(p2p_config)
        .map_err(|error| format!("failed to start mandatory libp2p service: {error}"))?;
    let p2p_info = p2p_service.info();
    let p2p_metadata = RuntimeP2pMetadata {
        peer_id: p2p_service.peer_id().to_string(),
        topics: p2p_info.subscribed_topics.len(),
        request_response_protocols: p2p_info.request_response_protocols.len(),
        bootstrap_peer_count,
        identity: p2p_identity_report(network.identity_seed),
        max_transmit_bytes,
        request_timeout_seconds,
        max_concurrent_streams,
        idle_timeout_seconds,
    };
    let node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    let gateway = RpcGateway::new(
        node,
        RpcPolicy {
            auth_token: Some(network.auth_token.clone()),
            ..RpcPolicy::default()
        },
    );
    let server = RpcHttpServer::bind(&network.rpc_listen, gateway).map_err(|error| {
        format!(
            "failed to bind service listener {}: {error}",
            network.rpc_listen
        )
    })?;
    Ok(RuntimeServices {
        store,
        server,
        p2p_service,
        p2p_metadata,
    })
}
