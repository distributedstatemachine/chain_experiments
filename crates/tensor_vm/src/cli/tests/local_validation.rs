use super::{execute_test_cli_args, parse_test_cli};
use libp2p::PeerId;
use std::path::PathBuf;

fn unique_test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tensor-vm-cli-validation-{name}-{}-{}",
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
fn execute_cli_rejects_invalid_local_args() {
    assert!(execute_test_cli_args(&["miner", "register", "--stake", "99"]).is_err());
    assert!(execute_test_cli_args(&["validator", "register", "--stake", "9999"]).is_err());
    assert!(
        execute_test_cli_args(&[
            "miner",
            "check",
            "--wallet",
            " ",
            "--device",
            "cpu",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
        ])
        .is_err()
    );
    for invalid_device in ["cpu-reference", "gpu0", "cuda:abc", "cuda:", " "] {
        assert!(
            parse_test_cli(&[
                "miner",
                "check",
                "--wallet",
                "miner.key",
                "--device",
                invalid_device,
                "--node",
                "/ip4/127.0.0.1/tcp/4001",
            ])
            .is_err(),
            "invalid miner device {invalid_device:?} must be rejected by Clap"
        );
    }
    assert!(
        parse_test_cli(&[
            "miner",
            "check",
            "--wallet",
            "miner.key",
            "--device",
            "cpu",
            "--node",
            "http://localhost:8545",
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "validator",
            "check",
            "--wallet",
            "validator.key",
            "--node",
            "localhost:8545",
        ])
        .is_err()
    );
    assert!(execute_test_cli_args(&["node", "init", "--data-dir", " "]).is_err());
    assert!(
        parse_test_cli(&[
            "node",
            "peer",
            "add",
            "--data-dir",
            "/var/lib/tensorvm",
            "--peer-id",
            "not-a-peer-id",
            "--address",
            "/dns/bootstrap.tensorvm.net/tcp/4001",
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "node",
            "peer",
            "add",
            "--data-dir",
            "/var/lib/tensorvm",
            "--peer-id",
            &PeerId::random().to_string(),
            "--address",
            "not-a-multiaddr",
        ])
        .is_err()
    );
    let peer_a = PeerId::random();
    let peer_b = PeerId::random();
    let peer_data_dir = unique_test_dir("peer-mismatch");
    let peer_data_dir = peer_data_dir.to_string_lossy().into_owned();
    let mismatched_peer_address = format!("/dns/bootstrap.tensorvm.net/tcp/4001/p2p/{peer_b}");
    assert!(
        execute_test_cli_args(&[
            "node",
            "peer",
            "add",
            "--data-dir",
            &peer_data_dir,
            "--peer-id",
            &peer_a.to_string(),
            "--address",
            &mismatched_peer_address,
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "node",
            "serve",
            "--listen",
            "localhost:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "node",
            "check",
            "--p2p-listen",
            "not-a-multiaddr",
            "--data-dir",
            "/var/lib/tensorvm",
        ])
        .is_err()
    );
    assert!(
        execute_test_cli_args(&[
            "node",
            "check",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/4001",
            "--data-dir",
            " ",
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "node",
            "serve",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "not-a-multiaddr",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
        ])
        .is_err()
    );
    assert!(
        execute_test_cli_args(&[
            "node",
            "serve",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/4001",
            "--data-dir",
            " ",
            "--auth-token",
            "secret",
        ])
        .is_err()
    );
    assert!(
        execute_test_cli_args(&[
            "node",
            "serve",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            " ",
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "node",
            "serve",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
            "--max-requests",
            "abc",
        ])
        .is_err()
    );
}
