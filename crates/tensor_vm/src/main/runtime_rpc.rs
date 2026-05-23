use std::io::ErrorKind;

use tensor_vm::{ChainSnapshot, NodeRuntimeState, NodeStore, RpcHttpServer};

pub(super) fn serve_rpc_once(
    store: &NodeStore,
    server: &mut RpcHttpServer,
    runtime_state: &mut NodeRuntimeState,
) -> std::result::Result<bool, String> {
    let chain_snapshot_before = ChainSnapshot::from_chain(&server.gateway().node.chain);
    match server.serve_next() {
        Ok(()) => record_served_request(
            store,
            server,
            runtime_state,
            ChainSnapshot::from_chain(&server.gateway().node.chain) != chain_snapshot_before,
        ),
        Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(false),
        Err(error) => Err(format!("service request failed: {error}")),
    }
}

fn record_served_request(
    store: &NodeStore,
    server: &RpcHttpServer,
    runtime_state: &mut NodeRuntimeState,
    chain_changed: bool,
) -> std::result::Result<bool, String> {
    if chain_changed {
        store
            .persist_chain(&server.gateway().node.chain)
            .map_err(|error| format!("failed to persist service state: {error}"))?;
    }
    runtime_state.record_served_request();
    Ok(true)
}
