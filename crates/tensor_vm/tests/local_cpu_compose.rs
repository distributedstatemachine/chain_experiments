use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn repo_path(relative: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn local_cpu_compose_bundle_matches_spec_artifact_shape() {
    for path in [
        "deploy/tensorvm/local-cpu/docker-compose.yml",
        "deploy/tensorvm/local-cpu/Dockerfile",
        "deploy/tensorvm/local-cpu/README.md",
        "deploy/tensorvm/local-cpu/env/local-cpu.env.example",
        "deploy/tensorvm/local-cpu/scripts/entrypoint.sh",
        "deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh",
        "deploy/tensorvm/local-cpu/scripts/check-restart-continuity.sh",
        "deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh",
        ".dockerignore",
    ] {
        assert!(Path::new(&repo_path(path)).exists(), "missing {path}");
    }

    let compose = fs::read_to_string(repo_path("deploy/tensorvm/local-cpu/docker-compose.yml"))
        .expect("compose file should be readable");
    let dockerfile = fs::read_to_string(repo_path("deploy/tensorvm/local-cpu/Dockerfile"))
        .expect("Dockerfile should be readable");
    let entrypoint =
        fs::read_to_string(repo_path("deploy/tensorvm/local-cpu/scripts/entrypoint.sh"))
            .expect("entrypoint should be readable");
    let env_file = fs::read_to_string(repo_path(
        "deploy/tensorvm/local-cpu/env/local-cpu.env.example",
    ))
    .expect("local CPU env file should be readable");
    let check_script = fs::read_to_string(repo_path(
        "deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh",
    ))
    .expect("check script should be readable");
    let restart_script = fs::read_to_string(repo_path(
        "deploy/tensorvm/local-cpu/scripts/check-restart-continuity.sh",
    ))
    .expect("restart continuity script should be readable");
    let rolling_restart_script = fs::read_to_string(repo_path(
        "deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh",
    ))
    .expect("rolling restart continuity script should be readable");
    let spec = fs::read_to_string(repo_path("docs/tensorvm/local_cpu_testnet_spec.md"))
        .expect("local CPU spec should be readable");
    let dockerignore =
        fs::read_to_string(repo_path(".dockerignore")).expect(".dockerignore should be readable");

    let miners = [
        "miner-00", "miner-01", "miner-02", "miner-03", "miner-04", "miner-05", "miner-06",
        "miner-07", "miner-08", "miner-09",
    ];
    let validators = [
        "validator-00",
        "validator-01",
        "validator-02",
        "validator-03",
        "validator-04",
    ];
    for service in miners.into_iter().chain(validators) {
        assert!(
            compose.contains(&format!("  {service}:")),
            "compose should define service {service}"
        );
        assert!(
            spec.contains(service),
            "spec should name required service {service}"
        );
    }

    assert_eq!(compose.matches("TENSORVM_ROLE: miner").count(), 10);
    assert_eq!(compose.matches("TENSORVM_ROLE: validator").count(), 5);
    assert_eq!(compose.matches(":/var/lib/tensorvm").count(), 15);
    assert!(compose.contains("tensorvm-local"));
    assert!(compose.contains("127.0.0.1:8545:8545"));
    assert!(compose.contains("127.0.0.1:4001:4001"));
    assert!(compose.contains("  explorer:"));
    assert!(compose.contains("127.0.0.1:${TENSORVM_LOCAL_CPU_EXPLORER_PORT:-8080}:8080"));
    assert!(compose.contains("/usr/local/bin/tensorvm-explorer"));
    assert!(compose.contains("TENSORVM_EXPLORER_WS_URL"));
    assert!(compose.contains("/explorer/ws?token=local-cpu-testnet-token"));
    assert!(compose.contains("condition: service_healthy"));
    assert!(compose.contains("TENSORVM_SEED_LOCAL_TESTNET: \"true\""));
    assert!(compose.contains("TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS: \"1000\""));
    assert!(env_file.contains("TENSORVM_SEED_LOCAL_TESTNET=true"));
    assert!(env_file.contains("TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS=1000"));
    assert!(env_file.contains(
        "TENSORVM_BOOTSTRAP_PEER_ID=12D3KooWS2oXcVvmNNWTiUzwDWJavRHQmewe1NDfJB7SxP43jA7s"
    ));
    assert_eq!(
        compose
            .matches("dockerfile: deploy/tensorvm/local-cpu/Dockerfile")
            .count(),
        1
    );

    let operator_ids = compose
        .lines()
        .filter_map(|line| line.trim().strip_prefix("TENSORVM_OPERATOR_ID: "))
        .collect::<BTreeSet<_>>();
    assert_eq!(operator_ids.len(), 15);

    assert!(dockerfile.contains("cargo build -p tensor_vm --release"));
    assert!(dockerfile.contains("cargo build -p tensor_vm_explorer --release"));
    assert!(dockerfile.contains("target/release/tensorvm-explorer"));
    assert!(dockerignore.lines().any(|line| line == "target"));
    assert!(dockerignore.lines().any(|line| line == ".git"));
    assert!(!dockerfile.contains("--features cuda-kernels"));
    assert!(!compose.contains("NVIDIA_VISIBLE_DEVICES"));
    assert!(!compose.contains("cuda:"));
    assert!(!compose.contains("devices:"));

    for required in [
        "tvmd service init",
        "tvmd service peer add",
        "tvmd miner start",
        "--device cpu",
        "tvmd validator start",
        "tvmd service readiness",
        "--identity-seed",
        "tvmd local-testnet seed",
        "local-testnet-seed.out",
        "runtime_command=${ROLE}_run",
        "tvmd miner run",
        "tvmd validator run",
        "public_evidence_full_spec=false",
        "independently_checkable=false",
    ] {
        assert!(
            entrypoint.contains(required),
            "entrypoint should contain {required}"
        );
    }

    for required in [
        "docker compose",
        "compose config --quiet",
        "ready_miners=10",
        "ready_validators=5",
        "distinct_operator_ids=15",
        "distinct_libp2p_peer_ids=15",
        "libp2p_ready_node_count=15",
        "cpu_ready_miner_count=10",
        "cuda_required_miner_count=0",
        "p2p_identity_seeded=true",
        "settled_receipts=10",
        "matmul_settled=true",
        "linear_training_settled=true",
        "rewarded_miners=",
        "total_reward_balance",
        "attestation_count",
        "finality_rate_bps=10000",
        "data_availability_bps=10000",
        "gateway chain head did not advance past seeded height 2",
        "protocol did not generate synthetic jobs after seed",
        "settled_receipt_count",
        "standalone_explorer_ready=true",
        "standalone_explorer_websocket_polling=true",
        "live_block_production=true",
        "live_synthetic_jobs=true",
        "live_linear_training_jobs=true",
        "live_attestations=true",
        "live_receipt_attestations=true",
        "live receipt details did not include post-seed TensorOp receipts",
        "live receipt details did not include post-seed LinearTrainingStep receipts",
        "live_tensor_op_receipts=true",
        "live_linear_training_receipts=true",
        "live_tensor_fetch=true",
        "live_rewards=true",
        "tvmd service status",
        "all_operator_status_count=15",
        "all_operator_min_height=",
        "latest_block_height",
        "block_log_root",
        "all_operator_first_live_block_hash=",
        "all_operator_live_block_convergence=true",
        "tvmd service block",
        "all_operator_common_head_height=",
        "all_operator_common_head_hash=",
        "all_operator_common_head_convergence=true",
        "all_operator_target_head_height=",
        "all_operator_target_head_hash=",
        "all_operator_target_state_root=",
        "all_operator_target_head_convergence=true",
        "all_operator_role_status=true",
        "all_operator_role_runtime_commands=true",
        "all_operator_role_runtime_counters=true",
        "all_operator_p2p_connected_peers=true",
        "all_operator_chain_counters=true",
        "all_operator_block_log_roots_observed=true",
        "data-ui=\"ratzilla-tui\"",
        "new WebSocket",
        "cargo test -p tensor_vm local_testnet --release",
        "public_evidence_full_spec=false",
        "independently_checkable=false",
    ] {
        assert!(
            check_script.contains(required),
            "check script should contain {required}"
        );
    }

    for required in [
        "check-local-testnet.sh",
        "docker compose",
        "timeout 15s docker compose",
        "tvmd service status",
        "tvmd service block",
        "timeout 60s docker compose",
        "timeout 600s \"$CHECK_SCRIPT\"",
        "local_cpu_restart_continuity_ready=true",
        "restart_services=",
        "before_common_head_height=",
        "before_common_head_hash=",
        "before_common_state_root=",
        "after_common_head_height=",
        "after_common_head_hash=",
        "after_common_state_root=",
        "restart_peer_ids_stable=true",
        "restart_heights_non_decreasing=true",
        "restart_heights_advance=true",
        "restart_block_counts_non_decreasing=true",
        "restart_block_counts_advance=true",
        "restart_state_roots_observed=true",
        "restart_state_roots_advance=true",
        "restart_block_log_roots_observed=true",
        "restart_block_log_roots_advance=true",
        "restart_previous_common_head_preserved=true",
        "restart_previous_common_state_root_preserved=true",
        "restart_blocks_continue=true",
        "restart_common_head_convergence=true",
    ] {
        assert!(
            restart_script.contains(required),
            "restart continuity script should contain {required}"
        );
    }

    for required in [
        "check-restart-continuity.sh",
        "EXPECTED_SERVICES=\"miner-00 miner-01 miner-02 miner-03 miner-04 miner-05 miner-06 miner-07 miner-08 miner-09 validator-00 validator-01 validator-02 validator-03 validator-04\"",
        "ROLLING_SERVICES=\"${*:-$EXPECTED_SERVICES}\"",
        "\"$RESTART_SCRIPT\" \"$service\"",
        "local_cpu_rolling_restart_continuity_ready=true",
        "rolling_restart_services=",
        "rolling_restart_service_count=",
        "rolling_restart_service=%s,ready",
        "rolling_restart_peer_ids_stable=true",
        "rolling_restart_heights_advance=true",
        "rolling_restart_block_counts_advance=true",
        "rolling_restart_state_roots_advance=true",
        "rolling_restart_block_log_roots_advance=true",
        "rolling_restart_previous_common_head_preserved=true",
        "rolling_restart_previous_common_state_root_preserved=true",
        "rolling_restart_blocks_continue=true",
        "rolling_restart_common_head_convergence=true",
    ] {
        assert!(
            rolling_restart_script.contains(required),
            "rolling restart continuity script should contain {required}"
        );
    }
}
