use super::*;

#[test]
fn execute_command_fixture_reports_local_runtime_readiness() {
    let miner_register =
        execute_command_fixture(&CommandFixture::MinerRegister { stake: 100 }).unwrap();
    assert!(miner_register.contains("command=miner_register"));
    assert!(miner_register.contains("min_stake=100"));
    assert!(miner_register.contains("stake_sufficient=true"));

    let miner_start = execute_command_fixture(&CommandFixture::MinerStart {
        wallet: "miner.key".to_owned(),
        device: "cpu".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
    })
    .unwrap();
    assert!(miner_start.contains("command=miner_start"));
    assert!(miner_start.contains("wallet=miner.key"));
    assert!(miner_start.contains("device=cpu"));
    assert!(miner_start.contains("device_backend=cpu-reference"));
    assert!(miner_start.contains(&format!(
        "cuda_kernels_compiled={}",
        cuda_kernels_compiled()
    )));
    assert!(miner_start.contains("node=/ip4/127.0.0.1/tcp/4001"));
    assert!(miner_start.contains(&format!("address={}", hex(&address(b"miner.key")))));
    assert!(miner_start.contains("reference_backend_ready=true"));

    let miner_run = execute_command_fixture(&CommandFixture::MinerRun {
        wallet: "miner.key".to_owned(),
        device: "cpu".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        listen: "127.0.0.1:8545".to_owned(),
        p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: Some([0x11; 32]),
        auth_token: "secret".to_owned(),
        max_requests: 7,
    })
    .unwrap();
    assert!(miner_run.contains("command=miner_run"));
    assert!(miner_run.contains("role=miner"));
    assert!(miner_run.contains("device_backend=cpu-reference"));
    assert!(miner_run.contains("p2p_runtime=libp2p"));
    assert!(miner_run.contains("p2p_identity_seeded=true"));
    assert!(miner_run.contains("role_runtime_ready=true"));

    let validator_register =
        execute_command_fixture(&CommandFixture::ValidatorRegister { stake: 10_000 }).unwrap();
    assert!(validator_register.contains("command=validator_register"));
    assert!(validator_register.contains("min_stake=10000"));

    let validator_start = execute_command_fixture(&CommandFixture::ValidatorStart {
        wallet: "validator.key".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
    })
    .unwrap();
    assert!(validator_start.contains("command=validator_start"));
    assert!(validator_start.contains("reference_verifier_ready=true"));

    let validator_run = execute_command_fixture(&CommandFixture::ValidatorRun {
        wallet: "validator.key".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        listen: "127.0.0.1:8545".to_owned(),
        p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: None,
        auth_token: "secret".to_owned(),
        max_requests: 7,
    })
    .unwrap();
    assert!(validator_run.contains("command=validator_run"));
    assert!(validator_run.contains("role=validator"));
    assert!(validator_run.contains("reference_verifier_ready=true"));
    assert!(validator_run.contains("p2p_runtime=libp2p"));
    assert!(validator_run.contains("p2p_identity_seeded=false"));
    assert!(validator_run.contains("role_runtime_ready=true"));

    let proposer_run = execute_command_fixture(&CommandFixture::ProposerRun {
        wallet: "proposer.key".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        listen: "127.0.0.1:8545".to_owned(),
        p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: Some([0x33; 32]),
        auth_token: "secret".to_owned(),
        max_requests: 7,
    })
    .unwrap();
    assert!(proposer_run.contains("command=proposer_run"));
    assert!(proposer_run.contains("role=proposer"));
    assert!(proposer_run.contains("proposer_ready=true"));
    assert!(proposer_run.contains("p2p_runtime=libp2p"));
    assert!(proposer_run.contains("p2p_identity_seeded=true"));
    assert!(proposer_run.contains("role_runtime_ready=true"));

    let miner_status = execute_command_fixture(&CommandFixture::MinerStatus).unwrap();
    assert!(miner_status.contains("command=miner_status"));
    assert!(miner_status.contains("status_source=rpc_or_node_store_required"));

    let validator_status = execute_command_fixture(&CommandFixture::ValidatorStatus).unwrap();
    assert!(validator_status.contains("command=validator_status"));
    assert!(validator_status.contains("status_source=rpc_or_node_store_required"));

    let service_init = execute_command_fixture(&CommandFixture::ServiceInit {
        data_dir: "/var/lib/tensorvm".to_owned(),
    })
    .unwrap();
    assert!(service_init.contains("command=service_init"));
    assert!(service_init.contains("node_store_ready=true"));

    let bootstrap_peer = PeerId::random().to_string();
    let service_peer_add = execute_command_fixture(&CommandFixture::ServicePeerAdd {
        data_dir: "/var/lib/tensorvm".to_owned(),
        peer_id: bootstrap_peer.clone(),
        address: "/dns/bootstrap.tensorvm.net/tcp/4001".to_owned(),
    })
    .unwrap();
    assert!(service_peer_add.contains("command=service_peer_add"));
    assert!(service_peer_add.contains(&format!("peer_id={bootstrap_peer}")));
    assert!(service_peer_add.contains("peer_book_ready=true"));

    let service_readiness = execute_command_fixture(&CommandFixture::ServiceReadiness {
        p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: Some([0x11; 32]),
    })
    .unwrap();
    assert!(service_readiness.contains("command=service_readiness"));
    assert!(service_readiness.contains("p2p_runtime=libp2p"));
    assert!(service_readiness.contains("p2p_gossipsub=enabled"));
    assert!(service_readiness.contains("p2p_identify=enabled"));
    assert!(service_readiness.contains("p2p_kademlia=enabled"));
    assert!(service_readiness.contains("p2p_request_response=enabled"));
    assert!(service_readiness.contains("p2p_identity_seeded=true"));
    assert!(service_readiness.contains(&format!("p2p_identity_seed={}", "11".repeat(32))));
    assert!(service_readiness.contains("p2p_max_transmit_bytes=1048576"));
    assert!(service_readiness.contains("p2p_request_timeout_seconds=10"));
    assert!(service_readiness.contains("p2p_max_concurrent_streams=128"));
    assert!(service_readiness.contains("p2p_idle_timeout_seconds=60"));
    assert!(service_readiness.contains("node_store_required=true"));
    assert!(service_readiness.contains("libp2p_ready=true"));

    let unseeded_service_readiness = execute_command_fixture(&CommandFixture::ServiceReadiness {
        p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: None,
    })
    .unwrap();
    assert!(unseeded_service_readiness.contains("p2p_identity_seeded=false"));

    let service_serve = execute_command_fixture(&CommandFixture::ServiceServe {
        listen: "0.0.0.0:8545".to_owned(),
        p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: Some([0x22; 32]),
        auth_token: "secret".to_owned(),
        max_requests: 0,
    })
    .unwrap();
    assert!(service_serve.contains("command=service_serve"));
    assert!(service_serve.contains("p2p_runtime=libp2p"));
    assert!(service_serve.contains("p2p_gossipsub=enabled"));
    assert!(service_serve.contains("p2p_identify=enabled"));
    assert!(service_serve.contains("p2p_kademlia=enabled"));
    assert!(service_serve.contains("p2p_request_response=enabled"));
    assert!(service_serve.contains("p2p_identity_seeded=true"));
    assert!(service_serve.contains(&format!("p2p_identity_seed={}", "22".repeat(32))));
    assert!(service_serve.contains("p2p_max_transmit_bytes=1048576"));
    assert!(service_serve.contains("p2p_request_timeout_seconds=10"));
    assert!(service_serve.contains("p2p_max_concurrent_streams=128"));
    assert!(service_serve.contains("p2p_idle_timeout_seconds=60"));
    assert!(service_serve.contains("auth_enabled=true"));
    assert!(service_serve.contains("rpc_routes=enabled"));
    assert!(service_serve.contains("explorer_routes=enabled"));
    assert!(service_serve.contains("faucet_routes=enabled"));
    assert!(service_serve.contains("telemetry_routes=enabled"));
    assert!(service_serve.contains("node_store_required=true"));

    let service_status = execute_command_fixture(&CommandFixture::ServiceStatus {
        data_dir: "/var/lib/tensorvm".to_owned(),
    })
    .unwrap();
    assert!(service_status.contains("command=service_status"));
    assert!(service_status.contains("data_dir=/var/lib/tensorvm"));
    assert!(service_status.contains("status_source=node_store"));

    let service_block = execute_command_fixture(&CommandFixture::ServiceBlock {
        data_dir: "/var/lib/tensorvm".to_owned(),
        height: 3,
    })
    .unwrap();
    assert!(service_block.contains("command=service_block"));
    assert!(service_block.contains("data_dir=/var/lib/tensorvm"));
    assert!(service_block.contains("height=3"));
    assert!(service_block.contains("status_source=node_store"));

    let local_seed = execute_command_fixture(&CommandFixture::LocalTestnetSeed {
        data_dir: "/var/lib/tensorvm".to_owned(),
    })
    .unwrap();
    assert!(local_seed.contains("command=local_testnet_seed"));
    assert!(local_seed.contains("data_dir=/var/lib/tensorvm"));
    assert!(local_seed.contains("local_cpu_seed_ready=true"));
}
