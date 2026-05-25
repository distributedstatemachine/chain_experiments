use super::parser_support::{multiaddr, role_runtime_args, role_wallet_args};
use super::{
    StakeArgs, TvmdCommand, ValidatorCheckArgs, ValidatorCommand, ValidatorRunArgs, parse_test_cli,
};

#[test]
fn parses_documented_validator_commands() {
    assert_eq!(
        parse_test_cli(&["validator", "register", "--stake", "10000"]).unwrap(),
        TvmdCommand::Validator(ValidatorCommand::Register(StakeArgs { stake: 10_000 }))
    );
    assert_eq!(
        parse_test_cli(&[
            "validator",
            "check",
            "--wallet",
            "validator.key",
            "--node",
            "/ip4/127.0.0.1/tcp/4001"
        ])
        .unwrap(),
        TvmdCommand::Validator(ValidatorCommand::Check(ValidatorCheckArgs {
            wallet: role_wallet_args("validator.key"),
            node: multiaddr("/ip4/127.0.0.1/tcp/4001"),
        }))
    );
    assert_eq!(
        parse_test_cli(&["validator", "status"]).unwrap(),
        TvmdCommand::Validator(ValidatorCommand::Status)
    );
    assert_eq!(
        parse_test_cli(&[
            "validator",
            "run",
            "--wallet",
            "validator.key",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
            "--max-requests",
            "7",
        ])
        .unwrap(),
        TvmdCommand::Validator(ValidatorCommand::Run(ValidatorRunArgs {
            wallet: role_wallet_args("validator.key"),
            runtime: role_runtime_args(
                "/ip4/127.0.0.1/tcp/4001",
                "127.0.0.1:8545",
                "/ip4/127.0.0.1/tcp/0",
                "/var/lib/tensorvm",
                None,
                "secret",
                7,
            ),
        }))
    );
    let identity_seed = "22".repeat(32);
    assert_eq!(
        parse_test_cli(&[
            "validator",
            "run",
            "--wallet",
            "validator.key",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
            "--auth-token",
            "secret",
            "--max-requests",
            "7",
        ])
        .unwrap(),
        TvmdCommand::Validator(ValidatorCommand::Run(ValidatorRunArgs {
            wallet: role_wallet_args("validator.key"),
            runtime: role_runtime_args(
                "/ip4/127.0.0.1/tcp/4001",
                "127.0.0.1:8545",
                "/ip4/127.0.0.1/tcp/0",
                "/var/lib/tensorvm",
                Some([0x22; 32]),
                "secret",
                7,
            ),
        }))
    );
}
