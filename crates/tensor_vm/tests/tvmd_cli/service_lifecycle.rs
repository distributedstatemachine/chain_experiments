use super::*;

#[test]
fn service_cli_lifecycle_starts_libp2p_and_serves_public_surfaces() {
    let data_dir = unique_test_dir("service-cli-lifecycle");
    let data_dir_text = data_dir.to_string_lossy().into_owned();

    let init = run_tvmd(&["service", "init", "--data-dir", &data_dir_text]);
    assert!(init.contains("command=service_init"));
    assert!(init.contains("existing_store=false"));
    assert!(init.contains("block_count="));

    let peer_id = PeerId::random().to_string();
    let peer_add = run_tvmd(&[
        "service",
        "peer",
        "add",
        "--data-dir",
        &data_dir_text,
        "--peer-id",
        &peer_id,
        "--address",
        "/ip4/127.0.0.1/tcp/4001",
    ]);
    assert!(peer_add.contains("command=service_peer_add"));
    assert!(peer_add.contains(&format!("peer_id={peer_id}")));
    assert!(peer_add.contains("/p2p/"));
    assert!(peer_add.contains("bootstrap_peers=1"));

    let readiness = run_tvmd(&[
        "service",
        "readiness",
        "--p2p-listen",
        "/ip4/127.0.0.1/tcp/0",
        "--data-dir",
        &data_dir_text,
    ]);
    assert!(readiness.contains("command=service_readiness"));
    assert!(readiness.contains("p2p_runtime=libp2p"));
    assert!(readiness.contains("p2p_peer_id="));
    assert!(readiness.contains("p2p_gossipsub_topics="));
    assert!(readiness.contains("p2p_request_response_protocols="));
    assert!(readiness.contains("p2p_bootstrap_peers=1"));
    assert!(readiness.contains("p2p_max_transmit_bytes=1048576"));
    assert!(readiness.contains("p2p_request_timeout_seconds=10"));
    assert!(readiness.contains("p2p_max_concurrent_streams=128"));
    assert!(readiness.contains("p2p_idle_timeout_seconds=60"));
    assert!(readiness.contains("node_store_ready=true"));
    assert!(readiness.contains("libp2p_ready=true"));

    let rpc_port = free_local_port();
    let listen = format!("127.0.0.1:{rpc_port}");
    let child = Command::new(env!("CARGO_BIN_EXE_tvmd"))
        .args([
            "service",
            "serve",
            "--listen",
            &listen,
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            &data_dir_text,
            "--auth-token",
            "service-token",
            "--max-requests",
            "19",
        ])
        .current_dir(workspace_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("tvmd service serve must spawn");

    let unauthenticated_health = unauthenticated_get_request(rpc_port, "/health");
    assert!(unauthenticated_health.contains("HTTP/1.1 401 Unauthorized"));
    assert!(unauthenticated_health.contains("unauthorized"));

    let health = authenticated_get_request(rpc_port, "/health");
    assert!(health.contains("HTTP/1.1 200 OK"));
    assert!(health.contains("\"status\":\"ok\""));
    assert!(health.contains("\"service\":\"all\""));

    for (path, service, endpoint_id, public_url) in [
        (
            "/rpc/health",
            "rpc",
            "55",
            "https://rpc.tensorvm.net/health",
        ),
        (
            "/explorer/health",
            "explorer",
            "66",
            "https://explorer.tensorvm.net/health",
        ),
        (
            "/faucet/health",
            "faucet",
            "77",
            "https://faucet.tensorvm.net/health",
        ),
        (
            "/telemetry/health",
            "telemetry",
            "88",
            "https://telemetry.tensorvm.net/health",
        ),
    ] {
        let response = authenticated_get_request(rpc_port, path);
        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("\"status\":\"ok\""));
        assert!(response.contains(&format!("\"service\":\"{service}\"")));
        assert_service_health_evidence_from_response(
            service,
            &endpoint_id.repeat(32),
            public_url,
            &response,
        );
    }

    let chain_head = authenticated_get_request(rpc_port, "/chain/head");
    assert!(chain_head.contains("HTTP/1.1 200 OK"));
    assert!(chain_head.contains("\"height\""));
    assert!(chain_head.contains("\"block_count\""));
    assert!(chain_head.contains("\"state_root\""));
    assert_service_content_evidence_from_response(
        &data_dir,
        "rpc",
        &"55".repeat(32),
        "https://rpc.tensorvm.net/chain/head",
        "/chain/head",
        "rpc-chain-head.body",
        &chain_head,
    );

    let current_epoch = authenticated_get_request(rpc_port, "/epoch/current");
    assert!(current_epoch.contains("HTTP/1.1 200 OK"));
    assert!(current_epoch.contains("\"epoch\""));

    let current_jobs = authenticated_get_request(rpc_port, "/jobs/current");
    assert!(current_jobs.contains("HTTP/1.1 200 OK"));
    assert!(current_jobs.contains("\"jobs\""));

    let genesis_block = authenticated_get_request(rpc_port, "/chain/block/0");
    assert!(genesis_block.contains("HTTP/1.1 404 Not Found"));
    assert!(genesis_block.contains("block not found"));

    let miner_address = "11".repeat(32);
    let tx = authenticated_request(
        rpc_port,
        "POST",
        "/tx",
        &format!("register_miner {miner_address}"),
    );
    assert!(tx.contains("HTTP/1.1 202 Accepted"));
    assert!(tx.contains("\"accepted\":true"));

    let validator_address = "44".repeat(32);
    let validator_tx = authenticated_request(
        rpc_port,
        "POST",
        "/tx",
        &format!("register_validator {validator_address}"),
    );
    assert!(validator_tx.contains("HTTP/1.1 202 Accepted"));
    assert!(validator_tx.contains("\"accepted\":true"));

    let miner_state = authenticated_get_request(rpc_port, &format!("/miners/{miner_address}"));
    assert!(miner_state.contains("HTTP/1.1 200 OK"));
    assert!(miner_state.contains(&format!("\"address\":\"{miner_address}\"")));
    assert!(miner_state.contains("\"stake\":100"));

    let validator_state =
        authenticated_get_request(rpc_port, &format!("/validators/{validator_address}"));
    assert!(validator_state.contains("HTTP/1.1 200 OK"));
    assert!(validator_state.contains(&format!("\"address\":\"{validator_address}\"")));
    assert!(validator_state.contains("\"stake\":10000"));

    let receipt = authenticated_request(rpc_port, "POST", "/receipt", &"22".repeat(32));
    assert!(receipt.contains("HTTP/1.1 202 Accepted"));
    assert!(receipt.contains("\"accepted\":true"));

    let attestation = authenticated_request(rpc_port, "POST", "/attestation", &"33".repeat(32));
    assert!(attestation.contains("HTTP/1.1 202 Accepted"));
    assert!(attestation.contains("\"accepted\":true"));

    let explorer = authenticated_get_request(rpc_port, "/explorer");
    assert!(explorer.contains("HTTP/1.1 200 OK"));
    assert!(explorer.contains("TensorVM Explorer"));
    assert_service_content_evidence_from_response(
        &data_dir,
        "explorer",
        &"66".repeat(32),
        "https://explorer.tensorvm.net/explorer",
        "/explorer",
        "explorer.body",
        &explorer,
    );

    let faucet = authenticated_get_request(rpc_port, "/faucet/page");
    assert!(faucet.contains("HTTP/1.1 200 OK"));
    assert!(faucet.contains("TensorVM Faucet"));
    assert_service_content_evidence_from_response(
        &data_dir,
        "faucet",
        &"77".repeat(32),
        "https://faucet.tensorvm.net/faucet/page",
        "/faucet/page",
        "faucet-page.body",
        &faucet,
    );

    let telemetry = authenticated_get_request(rpc_port, "/telemetry/dashboard");
    assert!(telemetry.contains("HTTP/1.1 200 OK"));
    assert!(telemetry.contains("TensorVM Telemetry"));
    assert_service_content_evidence_from_response(
        &data_dir,
        "telemetry",
        &"88".repeat(32),
        "https://telemetry.tensorvm.net/telemetry/dashboard",
        "/telemetry/dashboard",
        "telemetry-dashboard.body",
        &telemetry,
    );

    let output = child.wait_with_output().expect("service process must exit");
    assert!(
        output.status.success(),
        "service serve failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("service stdout must be utf8");
    assert!(stdout.contains("command=service_serve"));
    assert!(stdout.contains("p2p_runtime=libp2p"));
    assert!(stdout.contains("p2p_peer_id="));
    assert!(stdout.contains("p2p_gossipsub_topics="));
    assert!(stdout.contains("p2p_request_response_protocols="));
    assert!(stdout.contains("p2p_bootstrap_peers=1"));
    assert!(stdout.contains("p2p_max_transmit_bytes=1048576"));
    assert!(stdout.contains("p2p_request_timeout_seconds=10"));
    assert!(stdout.contains("p2p_max_concurrent_streams=128"));
    assert!(stdout.contains("p2p_idle_timeout_seconds=60"));
    assert!(stdout.contains("served_requests=19"));
    let p2p_peer_id = stdout_value(&stdout, "p2p_peer_id");
    let p2p_gossipsub_topics = stdout_value(&stdout, "p2p_gossipsub_topics");
    let p2p_request_response_protocols = stdout_value(&stdout, "p2p_request_response_protocols");
    let p2p_bootstrap_peers = stdout_value(&stdout, "p2p_bootstrap_peers");
    let service_log = data_dir.join("service.log");
    std::fs::write(&service_log, stdout.as_bytes()).expect("service log fixture must be written");
    let service_log_text = service_log.to_string_lossy().into_owned();
    let public_observation = run_tvmd(&[
        "evidence",
        "network",
        "observation",
        "--operator-id",
        &"99".repeat(32),
        "--peer-id",
        p2p_peer_id,
        "--listen-address",
        "/dns/node-a.tensorvm.net/tcp/4001",
        "--observed-at",
        "1700000000",
        "--gossip-topics",
        p2p_gossipsub_topics,
        "--request-response-protocols",
        p2p_request_response_protocols,
        "--bootstrap-peers",
        p2p_bootstrap_peers,
        "--max-transmit-bytes",
        "1048576",
        "--request-timeout-seconds",
        "10",
        "--max-concurrent-streams",
        "128",
        "--idle-timeout-seconds",
        "60",
    ]);
    assert!(public_observation.starts_with("network_runtime_observation="));
    assert!(public_observation.contains(p2p_peer_id));
    assert!(public_observation.contains("/dns/node-a.tensorvm.net/tcp/4001"));
    let public_observation_from_service_log = run_tvmd(&[
        "evidence",
        "network",
        "from-service-log",
        "--operator-id",
        &"99".repeat(32),
        "--listen-address",
        "/dns/node-a.tensorvm.net/tcp/4001",
        "--observed-at",
        "1700000000",
        "--service-log",
        &service_log_text,
    ]);
    assert_eq!(public_observation_from_service_log, public_observation);
    let observation_root = network_observation_root(&public_observation);
    let bundle_id = "aa".repeat(32);
    let manifest_signer = "bb".repeat(32);
    let summary_from_root = run_tvmd(&[
        "evidence",
        "record",
        "summary-roots",
        "--kind",
        "network-runtime",
        "--bundle-id",
        &bundle_id,
        "--manifest-signer",
        &manifest_signer,
        "--record-roots",
        observation_root,
    ]);
    assert!(summary_from_root.contains("network_runtime_observation_records=1"));
    assert!(summary_from_root.contains("network_runtime_observation_root="));
    assert!(summary_from_root.contains("network_runtime_observation_signature="));
    let artifact_from_root = run_tvmd(&[
        "evidence",
        "record",
        "artifact-roots",
        "--kind",
        "network-runtime",
        "--bundle-id",
        &bundle_id,
        "--manifest-signer",
        &manifest_signer,
        "--artifact-uri",
        "https://evidence.tensorvm.net/network-runtime.json",
        "--record-roots",
        observation_root,
    ]);
    assert!(artifact_from_root.starts_with(
        "record_artifact=network-runtime,https://evidence.tensorvm.net/network-runtime.json,"
    ));
    assert!(artifact_from_root.contains(",1,"));
    let (status, public_observation_stdout, public_observation_stderr) = run_tvmd_failure(&[
        "evidence",
        "network",
        "observation",
        "--operator-id",
        &"99".repeat(32),
        "--peer-id",
        p2p_peer_id,
        "--listen-address",
        "/ip4/127.0.0.1/tcp/4001",
        "--observed-at",
        "1700000000",
        "--gossip-topics",
        p2p_gossipsub_topics,
        "--request-response-protocols",
        p2p_request_response_protocols,
        "--bootstrap-peers",
        p2p_bootstrap_peers,
        "--max-transmit-bytes",
        "1048576",
        "--request-timeout-seconds",
        "10",
        "--max-concurrent-streams",
        "128",
        "--idle-timeout-seconds",
        "60",
    ]);
    assert_eq!(status, 1);
    assert!(public_observation_stdout.is_empty());
    assert!(public_observation_stderr.contains("network observation address is not public"));
    let (status, log_observation_stdout, log_observation_stderr) = run_tvmd_failure(&[
        "evidence",
        "network",
        "from-service-log",
        "--operator-id",
        &"99".repeat(32),
        "--listen-address",
        "/ip4/127.0.0.1/tcp/4001",
        "--observed-at",
        "1700000000",
        "--service-log",
        &service_log_text,
    ]);
    assert_eq!(status, 1);
    assert!(log_observation_stdout.is_empty());
    assert!(log_observation_stderr.contains("network observation address is not public"));

    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}
