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
    let check_script = fs::read_to_string(repo_path(
        "deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh",
    ))
    .expect("check script should be readable");
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
        "tvmd service serve",
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
        "rewarded_miners=9",
        "finality_rate_bps=10000",
        "data_availability_bps=10000",
        "\"height\":2",
        "\"block_count\":2",
        "standalone_explorer_ready=true",
        "standalone_explorer_websocket_polling=true",
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
}
