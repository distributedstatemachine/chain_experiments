use super::runtime::RoleRuntimeLoop;
use super::runtime_config::{RuntimeRole, ServiceRuntimeConfig, runtime_node_config};
use super::shared::local_cpu_seed_beacon;
use super::*;
use std::{
    collections::BTreeSet,
    path::Path,
    thread,
    time::{Duration, Instant},
};
use tensor_vm::{
    Chain, ChainCommand, ChainEngine, ChainNetworkPayloadProcessor, ChainParams, ChainProfile,
    Faucet, FreivaldsParams, JobScheduler, Libp2pControlPlaneConfig, NetworkEventIngest,
    NetworkPayloadApply, NodeConfig, NodeRuntimeState, NodeStore, PendingNetworkPayloads,
    ReceiptState, RpcGateway, RpcHttpServer, RpcNode, RpcPolicy, Tensor, TensorVmLibp2pService,
    ValidatorAttestation, VerificationResult,
    api::P2pMessage,
    encode_attestation_payload, encode_job_payload, encode_receipt_payload,
    hash::hex,
    network_ingest_order,
    node::{
        apply_network_attestation_payload, apply_network_job_payload,
        apply_network_receipt_payload, attestation_announcement_hash,
    },
    roles::{CpuReferenceMinerRole, RoleReceiptBundle},
    spawn_libp2p_service,
    testnet::{LocalTestnet, TestnetConfig},
    types::hash_bytes,
};
use tensor_vm::{ChainSnapshot, types::address};

#[path = "main_tests/miner_role.rs"]
mod miner_role;

#[path = "main_tests/network_payloads.rs"]
mod network_payloads;

#[path = "main_tests/runtime_persistence.rs"]
mod runtime_persistence;

#[path = "main_tests/runtime_roles.rs"]
mod runtime_roles;

#[path = "main_tests/runtime_state.rs"]
mod runtime_state;

#[path = "main_tests/service_commands.rs"]
mod service_commands;

#[path = "main_tests/validator_role.rs"]
mod validator_role;

fn assert_tensor_count(node: &RpcNode, expected: usize) {
    let response = node.handle(&tensor_vm::RpcRequest {
        method: "GET".to_owned(),
        path: "/tensor/latest".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(response.status, 200);
    assert!(
        response
            .body
            .contains(&format!("\"tensor_count\":{expected}")),
        "unexpected tensor latest response: {}",
        response.body
    );
}

fn insert_bundle_tensors(node: &mut RpcNode, bundle: &RoleReceiptBundle) {
    for tensor in bundle.served_tensors() {
        node.insert_tensor(tensor);
    }
}

fn unique_temp_data_dir(name: &str) -> std::path::PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("tensor-vm-{name}-{}-{now}", std::process::id()))
}

fn test_service_runtime_config(data_dir: &Path, auth_token: &str) -> ServiceRuntimeConfig {
    let data_dir_text = data_dir.to_string_lossy().into_owned();
    ServiceRuntimeConfig {
        runtime_command: "service_serve",
        role: RuntimeRole::Service,
        role_wallet_address: None,
        node: runtime_node_config(
            &data_dir_text,
            RuntimeRole::Service,
            "127.0.0.1:0",
            "/ip4/127.0.0.1/tcp/0",
            Some(hash_bytes(b"test", &[data_dir_text.as_bytes()])),
            auth_token,
            0,
        )
        .unwrap(),
    }
}

fn file_modified_at(path: &Path) -> std::time::SystemTime {
    std::fs::metadata(path).unwrap().modified().unwrap()
}

fn send_http_request(addr: std::net::SocketAddr, request: &str) -> String {
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpStream};

    let mut client = TcpStream::connect(addr).unwrap();
    client.write_all(request.as_bytes()).unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    response
}

fn free_tcp_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn wait_for_connected_role_services(
    service_a: &TensorVmLibp2pService,
    service_b: &TensorVmLibp2pService,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline
        && (service_a.connected_peer_count() == 0 || service_b.connected_peer_count() == 0)
    {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(service_a.connected_peer_count(), 1);
    assert_eq!(service_b.connected_peer_count(), 1);
}
