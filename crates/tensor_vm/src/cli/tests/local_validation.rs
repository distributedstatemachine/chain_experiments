use super::{ExpectedCommand, execute_reference_cli_command, parse_test_cli};
use libp2p::PeerId;

#[test]
fn miner_start_requires_real_cuda_readiness_for_cuda_devices() {
    let cuda_start = ExpectedCommand::MinerStart {
        wallet: "miner.key".to_owned(),
        device: "cuda:0".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
    };

    #[cfg(not(feature = "cuda-kernels"))]
    assert_eq!(
        execute_reference_cli_command(&cuda_start)
            .unwrap_err()
            .to_string(),
        "invalid receipt: cuda kernels not compiled"
    );

    #[cfg(feature = "cuda-kernels")]
    {
        let device_count = crate::runtime::cuda_device_count().unwrap_or(0);
        if device_count > 0 {
            let report = execute_reference_cli_command(&cuda_start).unwrap();
            assert!(report.contains("device_backend=cuda"));
            assert!(report.contains("gpu_backend_ready=true"));
            assert!(report.contains("cuda_kernels_compiled=true"));
            assert!(report.contains("cuda_device_index=0"));
            assert!(report.contains(&format!("cuda_device_count={device_count}")));
        }
        assert!(
            execute_reference_cli_command(&ExpectedCommand::MinerStart {
                wallet: "miner.key".to_owned(),
                device: format!("cuda:{device_count}"),
                node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            })
            .is_err()
        );
    }
}

#[test]
fn execute_reference_cli_command_rejects_invalid_local_args() {
    assert!(execute_reference_cli_command(&ExpectedCommand::MinerRegister { stake: 99 }).is_err());
    assert!(
        execute_reference_cli_command(&ExpectedCommand::ValidatorRegister { stake: 9_999 })
            .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::MinerStart {
            wallet: " ".to_owned(),
            device: "cpu".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::MinerStart {
            wallet: "miner.key".to_owned(),
            device: "gpu0".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::MinerStart {
            wallet: "miner.key".to_owned(),
            device: "cuda:abc".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::MinerStart {
            wallet: "miner.key".to_owned(),
            device: "cuda:".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::MinerStart {
            wallet: "miner.key".to_owned(),
            device: " ".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::MinerStart {
            wallet: "miner.key".to_owned(),
            device: "cpu".to_owned(),
            node: "http://localhost:8545".to_owned(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::ValidatorStart {
            wallet: "validator.key".to_owned(),
            node: "localhost:8545".to_owned(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::ServiceInit {
            data_dir: " ".to_owned(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::ServicePeerAdd {
            data_dir: "/var/lib/tensorvm".to_owned(),
            peer_id: "not-a-peer-id".to_owned(),
            address: "/dns/bootstrap.tensorvm.net/tcp/4001".to_owned(),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::ServicePeerAdd {
            data_dir: "/var/lib/tensorvm".to_owned(),
            peer_id: PeerId::random().to_string(),
            address: "not-a-multiaddr".to_owned(),
        })
        .is_err()
    );
    let peer_a = PeerId::random();
    let peer_b = PeerId::random();
    assert!(
        execute_reference_cli_command(&ExpectedCommand::ServicePeerAdd {
            data_dir: "/var/lib/tensorvm".to_owned(),
            peer_id: peer_a.to_string(),
            address: format!("/dns/bootstrap.tensorvm.net/tcp/4001/p2p/{peer_b}"),
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::ServiceServe {
            listen: "localhost:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
            auth_token: "secret".to_owned(),
            max_requests: 0,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::ServiceReadiness {
            p2p_listen: "not-a-multiaddr".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::ServiceReadiness {
            p2p_listen: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            data_dir: " ".to_owned(),
            identity_seed: None,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::ServiceServe {
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "not-a-multiaddr".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
            auth_token: "secret".to_owned(),
            max_requests: 0,
        })
        .is_err()
    );
    assert!(
        execute_reference_cli_command(&ExpectedCommand::ServiceServe {
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
        execute_reference_cli_command(&ExpectedCommand::ServiceServe {
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
