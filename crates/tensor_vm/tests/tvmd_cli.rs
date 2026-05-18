use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use libp2p::PeerId;

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

fn authenticated_health_request(port: u16) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(mut stream) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .expect("read timeout must be set");
                stream
                    .write_all(
                        b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nx-tensorchain-auth: service-token\r\nConnection: close\r\n\r\n",
                    )
                    .expect("health request must write");
                let mut response = String::new();
                stream
                    .read_to_string(&mut response)
                    .expect("health response must read");
                return response;
            }
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => panic!("service did not accept health request: {error}"),
        }
    }
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
fn service_cli_lifecycle_starts_libp2p_and_serves_health_once() {
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
            "1",
        ])
        .current_dir(workspace_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("tvmd service serve must spawn");

    let response = authenticated_health_request(rpc_port);
    assert!(response.contains("HTTP/1.1 200 OK"));
    assert!(response.contains("\"status\":\"ok\""));
    assert!(response.contains("\"service\":\"all\""));

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
    assert!(stdout.contains("served_requests=1"));

    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}
