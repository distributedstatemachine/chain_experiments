use super::*;
use std::path::PathBuf;

fn execute_cli(args: &[&str]) -> String {
    let command = parse_test_cli(args).expect("test CLI args must parse");
    execute_test_cli_command(&command).expect("test CLI command must execute")
}

fn unique_test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tensor-vm-cli-dispatch-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time must be after unix epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("test dir must be created");
    dir
}

#[test]
fn app_dispatch_reports_local_operator_checks() {
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
    assert_report_fields(
        &miner_start,
        &[
            ("command", "miner_start"),
            ("wallet", "miner.key"),
            ("address", miner_address.as_str()),
            ("device", "cpu"),
            ("node", "/ip4/127.0.0.1/tcp/4001"),
            ("device_backend", "cpu-reference"),
            ("reference_backend_ready", "true"),
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
}

#[test]
fn app_dispatch_reports_node_store_commands() {
    let data_dir = unique_test_dir("node-store");
    let data_dir = data_dir.to_string_lossy().into_owned();

    let service_init = execute_cli(&["node", "init", "--data-dir", &data_dir]);
    assert_report_fields(
        &service_init,
        &[
            ("command", "service_init"),
            ("data_dir", data_dir.as_str()),
            ("existing_store", "false"),
            ("recovered_store", "false"),
            ("block_count", "0"),
        ],
    );

    let bootstrap_peer = PeerId::random().to_string();
    let service_peer_add = execute_cli(&[
        "node",
        "peer",
        "add",
        "--data-dir",
        &data_dir,
        "--peer-id",
        &bootstrap_peer,
        "--address",
        "/ip4/127.0.0.1/tcp/4001",
    ]);
    assert_report_fields(
        &service_peer_add,
        &[
            ("command", "service_peer_add"),
            ("data_dir", data_dir.as_str()),
            ("peer_id", bootstrap_peer.as_str()),
            ("address", "/ip4/127.0.0.1/tcp/4001"),
            ("bootstrap_peers", "1"),
        ],
    );

    let identity_seed = "11".repeat(32);
    let service_readiness = execute_cli(&[
        "node",
        "check",
        "--p2p-listen",
        "/ip4/127.0.0.1/tcp/0",
        "--data-dir",
        &data_dir,
        "--identity-seed",
        &identity_seed,
    ]);
    assert_report_fields(
        &service_readiness,
        &[
            ("command", "service_readiness"),
            ("p2p_listen", "/ip4/127.0.0.1/tcp/0"),
            ("p2p_bootstrap_peers", "1"),
            ("p2p_identity_seeded", "true"),
            ("p2p_identity_seed", identity_seed.as_str()),
            ("p2p_max_transmit_bytes", "1048576"),
            ("p2p_request_timeout_seconds", "10"),
            ("p2p_max_concurrent_streams", "128"),
            ("p2p_idle_timeout_seconds", "60"),
            ("data_dir", data_dir.as_str()),
            ("node_store_ready", "true"),
            ("libp2p_ready", "true"),
        ],
    );

    let local_seed = execute_cli(&["localnet", "seed", "--data-dir", &data_dir]);
    assert_report_fields(
        &local_seed,
        &[
            ("command", "local_testnet_seed"),
            ("data_dir", data_dir.as_str()),
            ("miners", "10"),
            ("validators", "5"),
            ("matmul_settled", "true"),
            ("linear_training_settled", "true"),
            ("node_store_ready", "true"),
            ("public_evidence_full_spec", "false"),
            ("independently_checkable", "false"),
        ],
    );

    let local_verify = execute_cli(&["localnet", "verify", "--data-dir", &data_dir]);
    assert_report_fields(
        &local_verify,
        &[
            ("command", "local_cpu_verify"),
            ("data_dir", data_dir.as_str()),
            ("structured_verifier_ready", "true"),
            ("ready", "true"),
            ("block_count", "2"),
            ("finalized_block_count", "2"),
            ("node_store_ready", "true"),
        ],
    );

    let service_status = execute_cli(&["node", "status", "--data-dir", &data_dir]);
    assert_report_fields(
        &service_status,
        &[
            ("command", "service_status"),
            ("data_dir", data_dir.as_str()),
            ("node_store_ready", "true"),
            ("status_source", "node_store"),
            ("role", "unknown"),
        ],
    );

    let service_block = execute_cli(&["node", "block", "--data-dir", &data_dir, "--height", "1"]);
    assert_report_fields(
        &service_block,
        &[
            ("command", "service_block"),
            ("height", "1"),
            ("block_validation", "useful_verification_pow"),
            ("proposer_role", "validator"),
            ("proposer_registered", "true"),
            ("pow_valid", "true"),
            ("finalized", "true"),
        ],
    );
}
