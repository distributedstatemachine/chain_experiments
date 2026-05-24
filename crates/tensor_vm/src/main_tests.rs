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
    app::{
        RoleRuntimeLoop, RuntimeRole, ServiceRuntimeConfig, local_cpu_seed_beacon,
        runtime_node_config,
    },
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

#[path = "main_tests/support.rs"]
mod support;
use support::{
    assert_tensor_count, file_modified_at, free_tcp_port, insert_bundle_tensors, send_http_request,
    test_service_runtime_config, unique_temp_data_dir, wait_for_connected_role_services,
};

#[path = "main_tests/validator_role.rs"]
mod validator_role;
