use super::{CommandFixture, execute_command_fixture, parse_test_cli};
use libp2p::PeerId;

#[test]
fn miner_start_requires_real_cuda_readiness_for_cuda_devices() {
    let cuda_start = CommandFixture::MinerStart {
        wallet: "miner.key".to_owned(),
        device: "cuda:0".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
    };

    #[cfg(not(feature = "cuda-kernels"))]
    assert_eq!(
        execute_command_fixture(&cuda_start)
            .unwrap_err()
            .to_string(),
        "invalid receipt: cuda kernels not compiled"
    );

    #[cfg(feature = "cuda-kernels")]
    {
        let device_count = crate::runtime::cuda_device_count().unwrap_or(0);
        if device_count > 0 {
            let report = execute_command_fixture(&cuda_start).unwrap();
            let device_count_field = device_count.to_string();
            super::assert_report_fields(
                &report,
                &[
                    ("command", "miner_start"),
                    ("device", "cuda:0"),
                    ("device_backend", "cuda"),
                    ("gpu_backend_ready", "true"),
                    ("cuda_kernels_compiled", "true"),
                    ("cuda_device_index", "0"),
                    ("cuda_device_count", device_count_field.as_str()),
                ],
            );
        }
        assert!(
            execute_command_fixture(&CommandFixture::MinerStart {
                wallet: "miner.key".to_owned(),
                device: format!("cuda:{device_count}"),
                node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            })
            .is_err()
        );
    }
}

#[test]
fn execute_command_fixture_rejects_invalid_local_args() {
    assert!(execute_command_fixture(&CommandFixture::MinerRegister { stake: 99 }).is_err());
    assert!(execute_command_fixture(&CommandFixture::ValidatorRegister { stake: 9_999 }).is_err());
    assert!(
        execute_command_fixture(&CommandFixture::MinerStart {
            wallet: " ".to_owned(),
            device: "cpu".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::MinerStart {
            wallet: "miner.key".to_owned(),
            device: "gpu0".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::MinerStart {
            wallet: "miner.key".to_owned(),
            device: "cuda:abc".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::MinerStart {
            wallet: "miner.key".to_owned(),
            device: "cuda:".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::MinerStart {
            wallet: "miner.key".to_owned(),
            device: " ".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        })
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "miner",
            "start",
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
            "start",
            "--wallet",
            "validator.key",
            "--node",
            "localhost:8545",
        ])
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::ServiceInit {
            data_dir: " ".to_owned(),
        })
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "service",
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
            "service",
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
    assert!(
        execute_command_fixture(&CommandFixture::ServicePeerAdd {
            data_dir: "/var/lib/tensorvm".to_owned(),
            peer_id: peer_a.to_string(),
            address: format!("/dns/bootstrap.tensorvm.net/tcp/4001/p2p/{peer_b}"),
        })
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "service",
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
            "service",
            "readiness",
            "--p2p-listen",
            "not-a-multiaddr",
            "--data-dir",
            "/var/lib/tensorvm",
        ])
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::ServiceReadiness {
            p2p_listen: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            data_dir: " ".to_owned(),
            identity_seed: None,
        })
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "service",
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
        execute_command_fixture(&CommandFixture::ServiceServe {
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            data_dir: " ".to_owned(),
            identity_seed: None,
            auth_token: "secret".to_owned(),
            max_requests: 0,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::ServiceServe {
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
            auth_token: " ".to_owned(),
            max_requests: 0,
        })
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "service",
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
