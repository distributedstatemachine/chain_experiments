use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use libp2p::PeerId;
use tensor_vm::hash::hex;
use tensor_vm::types::address;

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

fn run_tvmd_failure(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_tvmd"))
        .args(args)
        .current_dir(workspace_root())
        .output()
        .expect("tvmd command must execute");

    assert!(
        !output.status.success(),
        "tvmd unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    (
        output.status.code().unwrap_or_default(),
        String::from_utf8(output.stdout).expect("tvmd stdout must be utf8"),
        String::from_utf8(output.stderr).expect("tvmd stderr must be utf8"),
    )
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

fn json_number_field(body: &str, key: &str) -> u64 {
    let marker = format!("\"{key}\":");
    let tail = body
        .split_once(&marker)
        .map(|(_, tail)| tail)
        .expect("JSON numeric field must exist");
    let digits = tail
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    digits.parse().expect("JSON numeric field must parse")
}

fn json_positive_field_count(body: &str, key: &str) -> usize {
    let marker = format!("\"{key}\":");
    body.match_indices(&marker)
        .filter(|(idx, _)| {
            let tail = &body[idx + marker.len()..];
            let digits = tail
                .chars()
                .take_while(|character| character.is_ascii_digit())
                .collect::<String>();
            digits.parse::<u64>().is_ok_and(|value| value > 0)
        })
        .count()
}

fn stdout_value<'a>(stdout: &'a str, key: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|line| line.strip_prefix(key))
        .and_then(|value| value.strip_prefix('='))
        .expect("expected service stdout field")
}

fn stdout_u64(stdout: &str, key: &str) -> u64 {
    stdout_value(stdout, key)
        .parse()
        .expect("expected numeric service stdout field")
}

fn trimmed_tvmd(args: &[&str]) -> String {
    run_tvmd(args).trim_end().to_owned()
}

fn network_observation_root(line: &str) -> &str {
    let fields = line
        .trim()
        .strip_prefix("network_runtime_observation=")
        .expect("network observation line must have expected prefix")
        .split(',')
        .collect::<Vec<_>>();
    assert_eq!(fields.len(), 13);
    fields[11]
}

fn comma_record_fields<'a>(line: &'a str, prefix: &str, expected_len: usize) -> Vec<&'a str> {
    let record = line
        .trim()
        .strip_prefix(prefix)
        .unwrap_or_else(|| panic!("record missing prefix {prefix:?}: {line}"));
    let fields = record.split(',').collect::<Vec<_>>();
    assert_eq!(
        fields.len(),
        expected_len,
        "unexpected field count for {prefix:?}: {line}"
    );
    fields
}

fn assert_service_health_evidence_from_response(
    kind: &str,
    endpoint_id: &str,
    public_url: &str,
    response: &str,
) {
    assert!(response.contains("HTTP/1.1 200 OK"));
    assert!(response.contains("\"status\":\"ok\""));
    assert!(response.contains(&format!("\"service\":\"{kind}\"")));
    let health = run_tvmd(&[
        "evidence",
        "service",
        "health",
        "--kind",
        kind,
        "--endpoint-id",
        endpoint_id,
        "--public-url",
        public_url,
        "--health-path",
        "/health",
        "--first-block",
        "0",
        "--last-block",
        "9",
        "--reachable-count",
        "10",
        "--signed-health-check-count",
        "10",
    ]);
    let fields = comma_record_fields(&health, "service=", 9);
    assert_eq!(
        fields[..8],
        [
            kind,
            endpoint_id,
            public_url,
            "/health",
            "0",
            "9",
            "10",
            "10"
        ]
    );
    assert_eq!(fields[8].len(), 64);
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
        "evidence",
        "service",
        "content-bytes",
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
    let min_content_bytes = body.len().to_string();
    let fields = comma_record_fields(&content_from_bytes, "service_content=", 8);
    assert_eq!(fields[..4], [kind, endpoint_id, public_url, content_path]);
    assert_eq!(fields[4].len(), 64);
    assert_eq!(fields[5..7], ["1700000000", min_content_bytes.as_str()]);
    assert_eq!(fields[7].len(), 64);

    let content_file = data_dir.join(file_name);
    std::fs::write(&content_file, body.as_bytes()).expect("service body fixture must be written");
    let content_file_text = content_file.to_string_lossy().into_owned();
    let content_from_file = run_tvmd(&[
        "evidence",
        "service",
        "content-file",
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

#[path = "tvmd_cli/public_evidence.rs"]
mod public_evidence;
#[path = "tvmd_cli/service_lifecycle.rs"]
mod service_lifecycle;

#[test]
fn local_testnet_service_gateway_does_not_produce_local_blocks() {
    let data_dir = unique_test_dir("local-testnet-seed");
    let data_dir_text = data_dir.to_string_lossy().into_owned();

    let seed = run_tvmd(&["testnet", "seed", "--data-dir", &data_dir_text]);
    assert_eq!(stdout_value(&seed, "command"), "local_testnet_seed");
    assert_eq!(stdout_u64(&seed, "miners"), 10);
    assert_eq!(stdout_u64(&seed, "validators"), 5);
    assert_eq!(stdout_u64(&seed, "height"), 2);
    assert_eq!(stdout_u64(&seed, "blocks"), 2);
    assert_eq!(stdout_value(&seed, "matmul_settled"), "true");
    assert_eq!(stdout_value(&seed, "linear_training_settled"), "true");
    assert!(stdout_u64(&seed, "rewarded_miners") > 0);
    assert!(stdout_u64(&seed, "total_reward_balance") > 0);
    assert!(stdout_u64(&seed, "attestation_count") > 0);
    assert_eq!(stdout_u64(&seed, "data_availability_bps"), 10_000);
    assert_eq!(stdout_value(&seed, "node_store_ready"), "true");
    assert_eq!(stdout_u64(&seed, "persisted_block_count"), 2);
    assert_eq!(stdout_value(&seed, "public_evidence_full_spec"), "false");
    assert_eq!(stdout_value(&seed, "independently_checkable"), "false");

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
            "4",
        ])
        .env("TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS", "25")
        .env("TENSORVM_LOCAL_CPU_ROLE_PRODUCER", "true")
        .current_dir(workspace_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("tvmd service serve must spawn");

    let initial_chain_head = authenticated_get_request(rpc_port, "/chain/head");
    assert!(initial_chain_head.contains("HTTP/1.1 200 OK"));
    let initial_height = json_number_field(response_body(&initial_chain_head), "height");
    let initial_block_count = json_number_field(response_body(&initial_chain_head), "block_count");
    assert!(initial_height >= 2);
    assert!(initial_block_count >= 2);

    std::thread::sleep(Duration::from_millis(150));

    let overview = authenticated_get_request(rpc_port, "/explorer/overview");
    assert!(overview.contains("HTTP/1.1 200 OK"));
    let overview_body = response_body(&overview);
    assert!(json_number_field(overview_body, "job_count") >= 2);
    assert!(json_number_field(overview_body, "receipt_count") >= 10);
    assert!(json_number_field(overview_body, "settled_receipt_count") >= 10);

    let receipts = authenticated_get_request(rpc_port, "/explorer/receipts/latest/500");
    assert!(receipts.contains("HTTP/1.1 200 OK"));
    let receipts_body = response_body(&receipts);
    assert!(receipts_body.contains("\"validator_attestations\""));
    assert!(json_positive_field_count(receipts_body, "attestation_count") >= 10);

    let later_chain_head = authenticated_get_request(rpc_port, "/chain/head");
    assert!(later_chain_head.contains("HTTP/1.1 200 OK"));
    assert_eq!(
        json_number_field(response_body(&later_chain_head), "height"),
        initial_height
    );
    assert_eq!(
        json_number_field(response_body(&later_chain_head), "block_count"),
        initial_block_count
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
    assert_eq!(stdout_value(&stdout, "command"), "service_serve");
    assert_eq!(stdout_value(&stdout, "chain_profile"), "local_cpu");
    assert_eq!(stdout_value(&stdout, "role_can_produce_blocks"), "false");
    assert_eq!(stdout_value(&stdout, "local_producer"), "false");
    assert_eq!(stdout_u64(&stdout, "served_requests"), 4);
    assert_eq!(stdout_value(&stdout, "produced_blocks"), "0");

    let status = run_tvmd(&["service", "status", "--data-dir", &data_dir_text]);
    assert_eq!(stdout_value(&status, "command"), "service_status");
    assert_eq!(stdout_value(&status, "node_store_ready"), "true");
    assert_eq!(stdout_value(&status, "status_source"), "node_store");
    assert_eq!(stdout_value(&status, "operator_name"), "unknown");
    assert_eq!(stdout_value(&status, "role"), "unknown");
    assert_eq!(stdout_value(&status, "role_chain_profile"), "local_cpu");
    assert_eq!(stdout_value(&status, "role_can_produce_blocks"), "false");
    assert_eq!(stdout_value(&status, "role_local_producer"), "false");
    assert_eq!(stdout_value(&status, "role_produced_blocks"), "0");
    assert_eq!(stdout_value(&status, "registered_miner_count"), "10");
    assert_eq!(stdout_value(&status, "registered_validator_count"), "5");
    assert!(
        stdout_value(&status, "job_count")
            .parse::<u64>()
            .expect("service status job count must parse")
            >= 2
    );
    assert!(
        stdout_value(&status, "receipt_count")
            .parse::<u64>()
            .expect("service status receipt count must parse")
            >= 10
    );
    assert!(
        stdout_value(&status, "attestation_count")
            .parse::<u64>()
            .expect("service status attestation count must parse")
            >= 10
    );
    assert_eq!(
        stdout_value(&status, "height")
            .parse::<u64>()
            .expect("service status height must parse"),
        initial_height
    );
    assert_eq!(
        stdout_value(&status, "block_count")
            .parse::<u64>()
            .expect("service status block count must parse"),
        initial_block_count
    );
    let latest_block_height = stdout_value(&status, "latest_block_height")
        .parse::<u64>()
        .expect("service status latest block height must parse");
    assert!(latest_block_height >= 1);
    let latest_block_height_text = latest_block_height.to_string();
    assert_ne!(stdout_value(&status, "block_log_root"), "0".repeat(64));
    assert!(
        stdout_value(&status, "finalized_block_count")
            .parse::<u64>()
            .expect("service status finalized block count must parse")
            >= 2
    );
    assert_eq!(stdout_value(&status, "first_live_block_height"), "0");
    let first_live_block_hash = stdout_value(&status, "first_live_block_hash");
    assert_eq!(first_live_block_hash, "0".repeat(64));

    let block = run_tvmd(&[
        "service",
        "block",
        "--data-dir",
        &data_dir_text,
        "--height",
        &latest_block_height_text,
    ]);
    assert!(block.contains("command=service_block"));
    assert_eq!(stdout_value(&block, "height"), latest_block_height_text);
    assert_eq!(
        stdout_value(&block, "block_validation"),
        "useful_verification_pow"
    );
    assert_eq!(stdout_value(&block, "proposer_role"), "validator");
    assert_eq!(stdout_value(&block, "proposer_registered"), "true");
    assert_eq!(
        stdout_value(&block, "tensorwork_proposer_selection"),
        "false"
    );
    assert!(block.contains("settled_receipt_set_root="));
    assert!(block.contains("checks_root="));
    assert!(block.contains("difficulty_target="));
    assert!(block.contains("nonce="));
    assert!(block.contains("pow_hash="));
    assert_eq!(stdout_value(&block, "pow_valid"), "true");
    assert_ne!(stdout_value(&block, "state_root"), "0".repeat(64));
    assert_eq!(stdout_value(&block, "finalized"), "true");
    assert!(
        stdout_value(&block, "block_vote_count")
            .parse::<u64>()
            .expect("service block vote count must parse")
            > 0
    );
    assert_ne!(stdout_value(&block, "block_vote_validators"), "none");
    assert_eq!(stdout_value(&block, "finality_validated_block"), "true");
    assert!(
        stdout_value(&block, "receipt_count")
            .parse::<u64>()
            .expect("service block receipt count must parse")
            > 0
    );
    assert_ne!(stdout_value(&block, "receipt_ids"), "none");
    assert!(
        stdout_value(&block, "settled_receipt_count")
            .parse::<u64>()
            .expect("service block settled receipt count must parse")
            > 0
    );
    assert!(block.contains("tensor_op_receipt_count="));
    assert!(block.contains("linear_training_receipt_count="));
    assert!(
        stdout_value(&block, "latest_height")
            .parse::<u64>()
            .expect("service block latest height must parse")
            >= 1
    );

    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn validator_run_with_local_producer_advances_cpu_chain() {
    let data_dir = unique_test_dir("validator-local-producer");
    let data_dir_text = data_dir.to_string_lossy().into_owned();

    let seed = run_tvmd(&["testnet", "seed", "--data-dir", &data_dir_text]);
    assert_eq!(stdout_value(&seed, "command"), "local_testnet_seed");

    let rpc_port = free_local_port();
    let listen = format!("127.0.0.1:{rpc_port}");
    let child = Command::new(env!("CARGO_BIN_EXE_tvmd"))
        .args([
            "validator",
            "run",
            "--wallet",
            "testnet-validator-0",
            "--node",
            "/ip4/127.0.0.1/tcp/4002",
            "--listen",
            &listen,
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            &data_dir_text,
            "--auth-token",
            "service-token",
            "--max-requests",
            "3",
        ])
        .env("TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS", "25")
        .env("TENSORVM_LOCAL_CPU_ROLE_PRODUCER", "true")
        .current_dir(workspace_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("tvmd validator run must spawn");

    let initial_chain_head = authenticated_get_request(rpc_port, "/chain/head");
    assert!(initial_chain_head.contains("HTTP/1.1 200 OK"));
    let initial_height = json_number_field(response_body(&initial_chain_head), "height");
    let initial_block_count = json_number_field(response_body(&initial_chain_head), "block_count");
    assert!(initial_height >= 2);
    assert!(initial_block_count >= 2);

    std::thread::sleep(Duration::from_millis(150));

    let overview = authenticated_get_request(rpc_port, "/explorer/overview");
    assert!(overview.contains("HTTP/1.1 200 OK"));
    let later_chain_head = authenticated_get_request(rpc_port, "/chain/head");
    assert!(later_chain_head.contains("HTTP/1.1 200 OK"));
    assert!(json_number_field(response_body(&later_chain_head), "height") > initial_height);
    assert!(
        json_number_field(response_body(&later_chain_head), "block_count") > initial_block_count
    );

    let output = child
        .wait_with_output()
        .expect("validator process must exit");
    assert!(
        output.status.success(),
        "validator run failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("validator stdout must be utf8");
    assert_eq!(stdout_value(&stdout, "command"), "validator_run");
    assert_eq!(stdout_value(&stdout, "role"), "validator");
    assert_eq!(stdout_value(&stdout, "runtime_command"), "validator_run");
    assert_eq!(stdout_value(&stdout, "role_can_produce_blocks"), "true");
    assert_eq!(
        stdout_value(&stdout, "role_wallet_registration"),
        "validator"
    );
    assert_eq!(stdout_value(&stdout, "role_wallet_registered"), "true");
    assert_eq!(stdout_value(&stdout, "local_producer"), "true");
    assert!(stdout_u64(&stdout, "produced_blocks") > 0);

    let status = run_tvmd(&["service", "status", "--data-dir", &data_dir_text]);
    assert_eq!(stdout_value(&status, "role_loop_role"), "validator");
    assert_eq!(stdout_value(&status, "role_can_produce_blocks"), "true");
    assert_eq!(
        stdout_value(&status, "role_wallet_registration"),
        "validator"
    );
    assert_eq!(stdout_value(&status, "role_local_producer"), "true");
    assert!(stdout_u64(&status, "role_produced_blocks") > 0);
    assert_eq!(stdout_value(&status, "first_live_block_height"), "3");
    let block = run_tvmd(&[
        "service",
        "block",
        "--data-dir",
        &data_dir_text,
        "--height",
        "3",
    ]);
    assert_eq!(stdout_value(&block, "proposer_role"), "validator");
    assert_eq!(stdout_value(&block, "proposer_registered"), "true");
    assert_eq!(
        stdout_value(&block, "tensorwork_proposer_selection"),
        "false"
    );
    assert_eq!(stdout_value(&block, "pow_valid"), "true");
    assert!(block.contains("canonical_blockspace_valid="));

    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn role_run_commands_serve_through_role_specific_surfaces() {
    for role in ["miner", "validator", "proposer"] {
        let data_dir = unique_test_dir(&format!("{role}-run"));
        let data_dir_text = data_dir.to_string_lossy().into_owned();
        let seed = run_tvmd(&["testnet", "seed", "--data-dir", &data_dir_text]);
        assert!(seed.contains("command=local_testnet_seed"));

        let rpc_port = free_local_port();
        let listen = format!("127.0.0.1:{rpc_port}");
        let mut args = vec![role.to_owned(), "run".to_owned(), "--wallet".to_owned()];
        let (wallet, expected_registration) = match role {
            "miner" => ("testnet-miner-0", "miner"),
            "validator" => ("testnet-validator-0", "validator"),
            "proposer" => ("testnet-validator-0", "validator"),
            _ => unreachable!("covered role set"),
        };
        if role == "miner" {
            args.extend([
                wallet.to_owned(),
                "--device".to_owned(),
                "cpu".to_owned(),
                "--node".to_owned(),
                "/ip4/127.0.0.1/tcp/4001".to_owned(),
            ]);
        } else if role == "validator" {
            args.extend([
                wallet.to_owned(),
                "--node".to_owned(),
                "/ip4/127.0.0.1/tcp/4002".to_owned(),
            ]);
        } else {
            args.extend([
                wallet.to_owned(),
                "--node".to_owned(),
                "/ip4/127.0.0.1/tcp/4003".to_owned(),
            ]);
        }
        args.extend([
            "--listen".to_owned(),
            listen,
            "--p2p-listen".to_owned(),
            "/ip4/127.0.0.1/tcp/0".to_owned(),
            "--data-dir".to_owned(),
            data_dir_text.clone(),
            "--auth-token".to_owned(),
            "service-token".to_owned(),
            "--max-requests".to_owned(),
            "1".to_owned(),
        ]);

        let child = Command::new(env!("CARGO_BIN_EXE_tvmd"))
            .args(&args)
            .current_dir(workspace_root())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("tvmd role run must spawn");

        let health = authenticated_get_request(rpc_port, "/health");
        assert!(health.contains("HTTP/1.1 200 OK"));

        let output = child.wait_with_output().expect("role process must exit");
        assert!(
            output.status.success(),
            "{role} run failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("role stdout must be utf8");
        assert!(stdout.contains(&format!("command={role}_run")));
        assert!(stdout.contains(&format!("role={role}")));
        assert!(stdout.contains("role_runtime_ready=true"));
        if role == "proposer" {
            assert!(stdout.contains("proposer_ready=true"));
        }
        assert!(stdout.contains("command=service_serve"));
        assert!(stdout.contains("role_loop_ready=true"));
        assert!(stdout.contains(&format!("runtime_command={role}_run")));
        assert!(stdout.contains("chain_profile=local_cpu"));
        let role_can_produce_blocks = if role == "validator" { "true" } else { "false" };
        let wallet_address = hex(&address(wallet.as_bytes()));
        assert!(stdout.contains(&format!(
            "role_can_produce_blocks={role_can_produce_blocks}"
        )));
        assert!(stdout.contains(&format!("role_wallet_address={wallet_address}")));
        assert!(stdout.contains(&format!("role_wallet_registration={expected_registration}")));
        assert!(stdout.contains("role_wallet_registered=true"));
        assert!(stdout.contains("miner_work_ready="));
        assert!(stdout.contains("miner_assigned_jobs_seen="));
        assert!(stdout.contains("miner_unreceipted_jobs="));
        assert!(stdout.contains("miner_receipts_submitted="));
        assert!(stdout.contains("miner_tensors_inserted="));
        assert!(stdout.contains("validator_work_ready="));
        assert!(stdout.contains("validator_assigned_receipts_seen="));
        assert!(stdout.contains("validator_unattested_receipts="));
        assert!(stdout.contains("validator_artifact_ready_receipts="));
        assert!(stdout.contains("validator_artifact_missing_receipts="));
        assert!(stdout.contains("validator_remote_tensor_fetch_attempts="));
        assert!(stdout.contains("validator_remote_tensor_fetch_successes="));
        assert!(stdout.contains("validator_remote_tensor_fetch_failures="));
        assert!(stdout.contains("validator_remote_tensor_fetch_bytes="));
        assert!(stdout.contains("validator_remote_tensors_inserted="));
        assert!(stdout.contains("validator_attestations_submitted="));
        assert!(stdout.contains("validator_block_votes_submitted="));
        assert!(stdout.contains("local_producer=false"));
        assert!(stdout.contains("p2p_runtime=libp2p"));
        assert!(stdout.contains("p2p_connected_peers="));
        assert!(stdout.contains("p2p_observed_block_gossip_count="));
        assert!(stdout.contains("p2p_observed_block_payload_gossip_count="));
        assert!(stdout.contains("p2p_observed_block_vote_gossip_count="));
        assert!(stdout.contains("p2p_observed_job_gossip_count="));
        assert!(stdout.contains("p2p_observed_receipt_gossip_count="));
        assert!(stdout.contains("p2p_observed_attestation_gossip_count="));
        assert!(stdout.contains("p2p_latest_observed_block_height="));
        assert!(stdout.contains("p2p_latest_observed_block_hash="));
        assert!(stdout.contains("p2p_observed_block_hashes="));
        assert!(stdout.contains("p2p_latest_observed_block_payload_height="));
        assert!(stdout.contains("p2p_latest_observed_block_payload_hash="));
        assert!(stdout.contains("p2p_observed_block_payload_hashes="));
        assert!(stdout.contains("served_requests=1"));
        assert!(stdout.contains("network_applied_blocks=0"));
        assert!(stdout.contains("network_events_ingested=0"));
        assert!(stdout.contains("network_block_payloads_ingested=0"));
        assert!(stdout.contains("network_block_payloads_applied=0"));
        assert!(stdout.contains("network_block_votes_ingested=0"));
        assert!(stdout.contains("network_block_votes_applied=0"));
        assert!(stdout.contains("network_invalid_events=0"));

        let status = run_tvmd(&["service", "status", "--data-dir", &data_dir_text]);
        assert!(status.contains(&format!("role_runtime_command={role}_run")));
        assert!(status.contains(&format!("role_loop_role={role}")));
        assert!(status.contains("role_loop_ready=true"));
        assert!(status.contains("role_chain_profile=local_cpu"));
        assert!(status.contains(&format!(
            "role_can_produce_blocks={role_can_produce_blocks}"
        )));
        assert!(status.contains(&format!("role_wallet_address={wallet_address}")));
        assert!(status.contains(&format!("role_wallet_registration={expected_registration}")));
        assert!(status.contains("role_wallet_registered=true"));
        assert!(status.contains("role_miner_work_ready="));
        assert!(status.contains("role_miner_assigned_jobs_seen="));
        assert!(status.contains("role_miner_unreceipted_jobs="));
        assert!(status.contains("role_miner_receipts_submitted="));
        assert!(status.contains("role_miner_tensors_inserted="));
        assert!(status.contains("role_validator_work_ready="));
        assert!(status.contains("role_validator_assigned_receipts_seen="));
        assert!(status.contains("role_validator_unattested_receipts="));
        assert!(status.contains("role_validator_artifact_ready_receipts="));
        assert!(status.contains("role_validator_artifact_missing_receipts="));
        assert!(status.contains("role_validator_remote_tensor_fetch_attempts="));
        assert!(status.contains("role_validator_remote_tensor_fetch_successes="));
        assert!(status.contains("role_validator_remote_tensor_fetch_failures="));
        assert!(status.contains("role_validator_remote_tensor_fetch_bytes="));
        assert!(status.contains("role_validator_remote_tensors_inserted="));
        assert!(status.contains("role_validator_attestations_submitted="));
        assert!(status.contains("role_validator_block_votes_submitted="));
        assert!(status.contains("role_local_producer=false"));
        assert!(status.contains("role_served_requests=1"));
        assert!(status.contains("role_network_applied_blocks=0"));
        assert!(status.contains("role_network_events_ingested=0"));
        assert!(status.contains("role_network_block_events_ingested=0"));
        assert!(status.contains("role_network_block_headers_ingested=0"));
        assert!(status.contains("role_network_block_payloads_ingested=0"));
        assert!(status.contains("role_network_block_payloads_applied=0"));
        assert!(status.contains("role_network_block_votes_ingested=0"));
        assert!(status.contains("role_network_block_votes_applied=0"));
        assert!(status.contains("role_network_job_events_ingested=0"));
        assert!(status.contains("role_network_job_payloads_ingested=0"));
        assert!(status.contains("role_network_job_payloads_applied=0"));
        assert!(status.contains("role_network_receipt_payloads_ingested=0"));
        assert!(status.contains("role_network_receipt_payloads_applied=0"));
        assert!(status.contains("role_network_attestation_payloads_ingested=0"));
        assert!(status.contains("role_network_attestation_payloads_applied=0"));
        assert!(status.contains("role_network_receipt_events_ingested=0"));
        assert!(status.contains("role_network_attestation_events_ingested=0"));
        assert!(status.contains("role_network_peer_events_ingested=0"));
        assert!(status.contains("role_network_invalid_events=0"));
        assert!(status.contains("role_p2p_connected_peers="));
        assert!(status.contains("role_p2p_observed_blocks="));
        assert!(status.contains("role_p2p_observed_block_payloads="));
        assert!(status.contains("role_p2p_observed_block_votes="));
        assert!(status.contains("role_p2p_observed_jobs="));
        assert!(status.contains("role_p2p_observed_receipts="));
        assert!(status.contains("role_p2p_observed_attestations="));
        assert!(status.contains("role_p2p_latest_observed_block_height="));
        assert!(status.contains("role_p2p_latest_observed_block_hash="));
        assert!(status.contains("role_p2p_observed_block_hashes="));
        assert!(status.contains("role_p2p_latest_observed_block_payload_height="));
        assert!(status.contains("role_p2p_latest_observed_block_payload_hash="));
        assert!(status.contains("role_p2p_observed_block_payload_hashes="));

        std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
    }
}
