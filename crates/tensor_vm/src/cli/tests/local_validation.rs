use super::{execute_test_cli_args, parse_test_cli};

#[test]
fn local_role_cli_rejects_invalid_args() {
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
}
