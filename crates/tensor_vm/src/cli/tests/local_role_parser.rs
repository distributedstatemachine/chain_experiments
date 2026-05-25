use super::parser_support::{role_runtime_args, role_wallet_args};
use super::{ProposerCommand, TvmdCommand, ValidatorRunArgs, parse_test_cli};

#[test]
fn parses_documented_proposer_commands() {
    assert_eq!(
        parse_test_cli(&[
            "proposer",
            "run",
            "--wallet",
            "proposer.key",
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
        TvmdCommand::Proposer(ProposerCommand::Run(ValidatorRunArgs {
            wallet: role_wallet_args("proposer.key"),
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
    let identity_seed = "33".repeat(32);
    assert_eq!(
        parse_test_cli(&[
            "proposer",
            "run",
            "--wallet",
            "proposer.key",
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
        TvmdCommand::Proposer(ProposerCommand::Run(ValidatorRunArgs {
            wallet: role_wallet_args("proposer.key"),
            runtime: role_runtime_args(
                "/ip4/127.0.0.1/tcp/4001",
                "127.0.0.1:8545",
                "/ip4/127.0.0.1/tcp/0",
                "/var/lib/tensorvm",
                Some([0x33; 32]),
                "secret",
                7,
            ),
        }))
    );
}
