use super::*;

fn execute_cli(args: &[&str]) -> String {
    let command = parse_test_cli(args).expect("test CLI args must parse");
    execute_test_cli_command(&command).expect("test CLI command must execute")
}

#[test]
fn execute_cli_reports_local_runtime_readiness() {
    let miner_register = execute_cli(&["miner", "register", "--stake", "100"]);
    assert_report_fields(
        &miner_register,
        &[
            ("command", "miner_register"),
            ("stake", "100"),
            ("min_stake", "100"),
            ("stake_sufficient", "true"),
        ],
    );

    let miner_start = execute_cli(&[
        "miner",
        "check",
        "--wallet",
        "miner.key",
        "--device",
        "cpu",
        "--node",
        "/ip4/127.0.0.1/tcp/4001",
    ]);
    let miner_address = hex(&address(b"miner.key"));
    let cuda_kernels_compiled = cuda_kernels_compiled().to_string();
    assert_report_fields(
        &miner_start,
        &[
            ("command", "miner_start"),
            ("wallet", "miner.key"),
            ("address", miner_address.as_str()),
            ("device", "cpu"),
            ("node", "/ip4/127.0.0.1/tcp/4001"),
            ("device_backend", "cpu-reference"),
            ("cuda_kernels_compiled", cuda_kernels_compiled.as_str()),
            ("reference_backend_ready", "true"),
        ],
    );

    let identity_seed_11 = "11".repeat(32);
    let miner_run = execute_cli(&[
        "miner",
        "run",
        "--wallet",
        "miner.key",
        "--device",
        "cpu",
        "--node",
        "/ip4/127.0.0.1/tcp/4001",
        "--listen",
        "127.0.0.1:8545",
        "--p2p-listen",
        "/ip4/127.0.0.1/tcp/0",
        "--data-dir",
        "/var/lib/tensorvm",
        "--identity-seed",
        &identity_seed_11,
        "--auth-token",
        "secret",
        "--max-requests",
        "7",
    ]);
    assert_report_fields(
        &miner_run,
        &[
            ("command", "miner_run"),
            ("role", "miner"),
            ("wallet", "miner.key"),
            ("address", miner_address.as_str()),
            ("device", "cpu"),
            ("node", "/ip4/127.0.0.1/tcp/4001"),
            ("listen", "127.0.0.1:8545"),
            ("p2p_listen", "/ip4/127.0.0.1/tcp/0"),
            ("p2p_runtime", "libp2p"),
            ("p2p_gossipsub", "enabled"),
            ("p2p_identify", "enabled"),
            ("p2p_kademlia", "enabled"),
            ("p2p_request_response", "enabled"),
            ("device_backend", "cpu-reference"),
            ("cuda_kernels_compiled", cuda_kernels_compiled.as_str()),
            ("p2p_identity_seeded", "true"),
            ("p2p_identity_seed", identity_seed_11.as_str()),
            ("p2p_max_transmit_bytes", "1048576"),
            ("p2p_request_timeout_seconds", "10"),
            ("p2p_max_concurrent_streams", "128"),
            ("p2p_idle_timeout_seconds", "60"),
            ("data_dir", "/var/lib/tensorvm"),
            ("auth_enabled", "true"),
            ("max_requests", "7"),
            ("role_runtime_ready", "true"),
        ],
    );

    let validator_register = execute_cli(&["validator", "register", "--stake", "10000"]);
    assert_report_fields(
        &validator_register,
        &[
            ("command", "validator_register"),
            ("stake", "10000"),
            ("min_stake", "10000"),
            ("stake_sufficient", "true"),
        ],
    );

    let validator_start = execute_cli(&[
        "validator",
        "check",
        "--wallet",
        "validator.key",
        "--node",
        "/ip4/127.0.0.1/tcp/4001",
    ]);
    let validator_address = hex(&address(b"validator.key"));
    assert_report_fields(
        &validator_start,
        &[
            ("command", "validator_start"),
            ("wallet", "validator.key"),
            ("address", validator_address.as_str()),
            ("node", "/ip4/127.0.0.1/tcp/4001"),
            ("reference_verifier_ready", "true"),
        ],
    );

    let validator_run = execute_cli(&[
        "validator",
        "run",
        "--wallet",
        "validator.key",
        "--node",
        "/ip4/127.0.0.1/tcp/4001",
        "--listen",
        "127.0.0.1:8545",
        "--p2p-listen",
        "/ip4/127.0.0.1/tcp/0",
        "--data-dir",
        "/var/lib/tensorvm",
        "--auth-token",
        "secret",
        "--max-requests",
        "7",
    ]);
    assert_report_fields(
        &validator_run,
        &[
            ("command", "validator_run"),
            ("role", "validator"),
            ("wallet", "validator.key"),
            ("address", validator_address.as_str()),
            ("node", "/ip4/127.0.0.1/tcp/4001"),
            ("listen", "127.0.0.1:8545"),
            ("p2p_listen", "/ip4/127.0.0.1/tcp/0"),
            ("p2p_runtime", "libp2p"),
            ("p2p_gossipsub", "enabled"),
            ("p2p_identify", "enabled"),
            ("p2p_kademlia", "enabled"),
            ("p2p_request_response", "enabled"),
            ("p2p_identity_seeded", "false"),
            ("p2p_max_transmit_bytes", "1048576"),
            ("p2p_request_timeout_seconds", "10"),
            ("p2p_max_concurrent_streams", "128"),
            ("p2p_idle_timeout_seconds", "60"),
            ("data_dir", "/var/lib/tensorvm"),
            ("auth_enabled", "true"),
            ("max_requests", "7"),
            ("reference_verifier_ready", "true"),
            ("role_runtime_ready", "true"),
        ],
    );

    let identity_seed_33 = "33".repeat(32);
    let proposer_run = execute_cli(&[
        "proposer",
        "run",
        "--wallet",
        "proposer.key",
        "--node",
        "/ip4/127.0.0.1/tcp/4001",
        "--listen",
        "127.0.0.1:8545",
        "--p2p-listen",
        "/ip4/127.0.0.1/tcp/0",
        "--data-dir",
        "/var/lib/tensorvm",
        "--identity-seed",
        &identity_seed_33,
        "--auth-token",
        "secret",
        "--max-requests",
        "7",
    ]);
    let proposer_address = hex(&address(b"proposer.key"));
    assert_report_fields(
        &proposer_run,
        &[
            ("command", "proposer_run"),
            ("role", "proposer"),
            ("wallet", "proposer.key"),
            ("address", proposer_address.as_str()),
            ("node", "/ip4/127.0.0.1/tcp/4001"),
            ("listen", "127.0.0.1:8545"),
            ("p2p_listen", "/ip4/127.0.0.1/tcp/0"),
            ("p2p_runtime", "libp2p"),
            ("p2p_gossipsub", "enabled"),
            ("p2p_identify", "enabled"),
            ("p2p_kademlia", "enabled"),
            ("p2p_request_response", "enabled"),
            ("p2p_identity_seeded", "true"),
            ("p2p_identity_seed", identity_seed_33.as_str()),
            ("p2p_max_transmit_bytes", "1048576"),
            ("p2p_request_timeout_seconds", "10"),
            ("p2p_max_concurrent_streams", "128"),
            ("p2p_idle_timeout_seconds", "60"),
            ("data_dir", "/var/lib/tensorvm"),
            ("auth_enabled", "true"),
            ("max_requests", "7"),
            ("proposer_ready", "true"),
            ("role_runtime_ready", "true"),
        ],
    );

    let miner_status = execute_cli(&["miner", "status"]);
    assert_report_fields(
        &miner_status,
        &[
            ("command", "miner_status"),
            ("min_stake", "100"),
            ("reference_backend_ready", "true"),
            ("status_source", "rpc_or_node_store_required"),
        ],
    );

    let validator_status = execute_cli(&["validator", "status"]);
    assert_report_fields(
        &validator_status,
        &[
            ("command", "validator_status"),
            ("min_stake", "10000"),
            ("reference_verifier_ready", "true"),
            ("status_source", "rpc_or_node_store_required"),
        ],
    );

    let service_init = execute_cli(&["node", "init", "--data-dir", "/var/lib/tensorvm"]);
    assert_report_fields(
        &service_init,
        &[
            ("command", "service_init"),
            ("data_dir", "/var/lib/tensorvm"),
            ("node_store_ready", "true"),
        ],
    );

    let bootstrap_peer = PeerId::random().to_string();
    let service_peer_add = execute_cli(&[
        "node",
        "peer",
        "add",
        "--data-dir",
        "/var/lib/tensorvm",
        "--peer-id",
        &bootstrap_peer,
        "--address",
        "/dns/bootstrap.tensorvm.net/tcp/4001",
    ]);
    assert_report_fields(
        &service_peer_add,
        &[
            ("command", "service_peer_add"),
            ("data_dir", "/var/lib/tensorvm"),
            ("peer_id", bootstrap_peer.as_str()),
            ("address", "/dns/bootstrap.tensorvm.net/tcp/4001"),
            ("peer_book_ready", "true"),
        ],
    );

    let service_readiness = execute_cli(&[
        "node",
        "check",
        "--p2p-listen",
        "/ip4/0.0.0.0/tcp/4001",
        "--data-dir",
        "/var/lib/tensorvm",
        "--identity-seed",
        &identity_seed_11,
    ]);
    assert_report_fields(
        &service_readiness,
        &[
            ("command", "service_readiness"),
            ("p2p_listen", "/ip4/0.0.0.0/tcp/4001"),
            ("p2p_runtime", "libp2p"),
            ("p2p_gossipsub", "enabled"),
            ("p2p_identify", "enabled"),
            ("p2p_kademlia", "enabled"),
            ("p2p_request_response", "enabled"),
            ("p2p_identity_seeded", "true"),
            ("p2p_identity_seed", identity_seed_11.as_str()),
            ("p2p_max_transmit_bytes", "1048576"),
            ("p2p_request_timeout_seconds", "10"),
            ("p2p_max_concurrent_streams", "128"),
            ("p2p_idle_timeout_seconds", "60"),
            ("data_dir", "/var/lib/tensorvm"),
            ("node_store_required", "true"),
            ("libp2p_ready", "true"),
        ],
    );

    let unseeded_service_readiness = execute_cli(&[
        "node",
        "check",
        "--p2p-listen",
        "/ip4/0.0.0.0/tcp/4001",
        "--data-dir",
        "/var/lib/tensorvm",
    ]);
    assert_report_fields(
        &unseeded_service_readiness,
        &[
            ("command", "service_readiness"),
            ("p2p_identity_seeded", "false"),
        ],
    );

    let identity_seed_22 = "22".repeat(32);
    let service_serve = execute_cli(&[
        "node",
        "serve",
        "--listen",
        "0.0.0.0:8545",
        "--p2p-listen",
        "/ip4/0.0.0.0/tcp/4001",
        "--data-dir",
        "/var/lib/tensorvm",
        "--identity-seed",
        &identity_seed_22,
        "--auth-token",
        "secret",
        "--max-requests",
        "0",
    ]);
    assert_report_fields(
        &service_serve,
        &[
            ("command", "service_serve"),
            ("listen", "0.0.0.0:8545"),
            ("p2p_listen", "/ip4/0.0.0.0/tcp/4001"),
            ("p2p_runtime", "libp2p"),
            ("p2p_gossipsub", "enabled"),
            ("p2p_identify", "enabled"),
            ("p2p_kademlia", "enabled"),
            ("p2p_request_response", "enabled"),
            ("p2p_identity_seeded", "true"),
            ("p2p_identity_seed", identity_seed_22.as_str()),
            ("p2p_max_transmit_bytes", "1048576"),
            ("p2p_request_timeout_seconds", "10"),
            ("p2p_max_concurrent_streams", "128"),
            ("p2p_idle_timeout_seconds", "60"),
            ("data_dir", "/var/lib/tensorvm"),
            ("auth_enabled", "true"),
            ("max_requests", "0"),
            ("rpc_routes", "enabled"),
            ("explorer_routes", "enabled"),
            ("faucet_routes", "enabled"),
            ("telemetry_routes", "enabled"),
            ("node_store_required", "true"),
        ],
    );

    let service_status = execute_cli(&["node", "status", "--data-dir", "/var/lib/tensorvm"]);
    assert_report_fields(
        &service_status,
        &[
            ("command", "service_status"),
            ("data_dir", "/var/lib/tensorvm"),
            ("status_source", "node_store"),
        ],
    );

    let service_block = execute_cli(&[
        "node",
        "block",
        "--data-dir",
        "/var/lib/tensorvm",
        "--height",
        "3",
    ]);
    assert_report_fields(
        &service_block,
        &[
            ("command", "service_block"),
            ("data_dir", "/var/lib/tensorvm"),
            ("height", "3"),
            ("status_source", "node_store"),
        ],
    );

    let local_seed = execute_cli(&["localnet", "seed", "--data-dir", "/var/lib/tensorvm"]);
    assert_report_fields(
        &local_seed,
        &[
            ("command", "local_testnet_seed"),
            ("data_dir", "/var/lib/tensorvm"),
            ("local_cpu_seed_ready", "true"),
        ],
    );

    let local_verify = execute_cli(&["localnet", "verify", "--data-dir", "/var/lib/tensorvm"]);
    assert_report_fields(
        &local_verify,
        &[
            ("command", "local_cpu_verify"),
            ("data_dir", "/var/lib/tensorvm"),
            ("structured_verifier_ready", "true"),
        ],
    );
}
