use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use libp2p::PeerId;
use tensor_vm::hash::hex;

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run_tvmd(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_tvmd"))
        .args(args)
        .current_dir(workspace_root())
        .output()
        .expect("tvmd command must execute");

    assert!(
        output.status.success(),
        "tvmd failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("tvmd stdout must be utf8")
}

fn unique_test_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tensor-vm-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time must be after epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("test dir must be created");
    dir
}

fn free_local_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("local ephemeral port must bind")
        .local_addr()
        .expect("local addr must be available")
        .port()
}

fn service_request(
    port: u16,
    method: &str,
    path: &str,
    body: &str,
    auth_token: Option<&str>,
) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(mut stream) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .expect("read timeout must be set");
                let auth_header = auth_token
                    .map(|token| format!("x-tensorchain-auth: {token}\r\n"))
                    .unwrap_or_default();
                let request = format!(
                    "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{auth_header}content-length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream
                    .write_all(request.as_bytes())
                    .expect("service request must write");
                let mut response = String::new();
                stream
                    .read_to_string(&mut response)
                    .expect("service response must read");
                return response;
            }
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => panic!("service did not accept request: {error}"),
        }
    }
}

fn authenticated_request(port: u16, method: &str, path: &str, body: &str) -> String {
    service_request(port, method, path, body, Some("service-token"))
}

fn authenticated_get_request(port: u16, path: &str) -> String {
    authenticated_request(port, "GET", path, "")
}

fn unauthenticated_get_request(port: u16, path: &str) -> String {
    service_request(port, "GET", path, "", None)
}

fn response_body(response: &str) -> &str {
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .expect("HTTP response must contain a body separator")
}

fn assert_service_content_evidence_from_response(
    data_dir: &Path,
    kind: &str,
    endpoint_id: &str,
    public_url: &str,
    content_path: &str,
    file_name: &str,
    response: &str,
) {
    let body = response_body(response);
    assert!(
        body.len() >= 64,
        "{content_path} body must satisfy service-content byte minimum"
    );
    let body_hex = hex(body.as_bytes());
    let content_from_bytes = run_tvmd(&[
        "public-evidence",
        "service-content-from-bytes",
        "--kind",
        kind,
        "--endpoint-id",
        endpoint_id,
        "--public-url",
        public_url,
        "--content-path",
        content_path,
        "--observed-at",
        "1700000000",
        "--content-hex",
        &body_hex,
    ]);
    assert!(content_from_bytes.starts_with(&format!("service_content={kind},")));
    assert!(content_from_bytes.contains(endpoint_id));
    assert!(content_from_bytes.contains(&format!("{public_url},{content_path}")));
    assert!(content_from_bytes.contains(&format!(",{},", body.len())));

    let content_file = data_dir.join(file_name);
    std::fs::write(&content_file, body.as_bytes()).expect("service body fixture must be written");
    let content_file_text = content_file.to_string_lossy().into_owned();
    let content_from_file = run_tvmd(&[
        "public-evidence",
        "service-content-from-file",
        "--kind",
        kind,
        "--endpoint-id",
        endpoint_id,
        "--public-url",
        public_url,
        "--content-path",
        content_path,
        "--observed-at",
        "1700000000",
        "--content-file",
        &content_file_text,
    ]);
    assert_eq!(content_from_file, content_from_bytes);
}

#[test]
fn documented_public_testnet_preflight_command_reports_pending_status() {
    let stdout = run_tvmd(&[
        "public-testnet",
        "preflight",
        "--manifest",
        "docs/tensorvm/public-testnet.preflight",
    ]);

    assert!(stdout.contains("public_testnet_preflight_ready=false"));
    assert!(stdout.contains("local_shape_ready=true"));
    assert!(stdout.contains("deployment_plan_ready=false"));
    assert!(stdout.contains("production_libp2p_runtime=true"));
    assert!(stdout.contains("public_services_planned=false"));
}

#[test]
fn documented_public_testnet_evidence_command_reports_non_full_spec_status() {
    let stdout = run_tvmd(&[
        "public-evidence",
        "validate",
        "--manifest",
        "docs/tensorvm/public-testnet.evidence",
    ]);

    assert!(stdout.contains("public_evidence_full_spec=false"));
    assert!(stdout.contains("public_criterion=false"));
    assert!(stdout.contains("independently_checkable=false"));
    assert!(stdout.contains("published_evidence_bundle=false"));
    assert!(stdout.contains("signed_run_window=true"));
    assert!(stdout.contains("supporting_record_artifacts=false"));
    assert!(stdout.contains("required_run_duration=false"));
    assert!(stdout.contains("required_block_count=false"));
}

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

    for (path, service) in [
        ("/rpc/health", "rpc"),
        ("/explorer/health", "explorer"),
        ("/faucet/health", "faucet"),
        ("/telemetry/health", "telemetry"),
    ] {
        let response = authenticated_get_request(rpc_port, path);
        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("\"status\":\"ok\""));
        assert!(response.contains(&format!("\"service\":\"{service}\"")));
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

    let telemetry = authenticated_get_request(rpc_port, "/telemetry/dashboard");
    assert!(telemetry.contains("HTTP/1.1 200 OK"));
    assert!(telemetry.contains("TensorVM Telemetry"));

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
    assert!(stdout.contains("served_requests=19"));

    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}
