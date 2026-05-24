use super::parser_support::{miner_device, multiaddr, path, role_runtime_args};
use super::{
    MinerCheckArgs, MinerCommand, MinerRunArgs, ProposerCommand, StakeArgs, TvmdCommand,
    ValidatorCheckArgs, ValidatorCommand, ValidatorRunArgs, parse_test_cli,
};

#[test]
fn parses_documented_miner_commands() {
    assert_eq!(
        parse_test_cli(&["miner", "register", "--stake", "100"]).unwrap(),
        TvmdCommand::Miner(MinerCommand::Register(StakeArgs { stake: 100 }))
    );
    assert_eq!(
        parse_test_cli(&[
            "miner",
            "check",
            "--wallet",
            "miner.key",
            "--device",
            "cpu",
            "--node",
            "/ip4/127.0.0.1/tcp/4001"
        ])
        .unwrap(),
        TvmdCommand::Miner(MinerCommand::Check(MinerCheckArgs {
            wallet: path("miner.key"),
            device: miner_device("cpu"),
            node: multiaddr("/ip4/127.0.0.1/tcp/4001"),
        }))
    );
    assert_eq!(
        parse_test_cli(&["miner", "status"]).unwrap(),
        TvmdCommand::Miner(MinerCommand::Status)
    );
    assert_eq!(
        parse_test_cli(&[
            "miner",
            "run",
            "--wallet",
            "miner.key",
            "--device",
            "cpu",
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
        TvmdCommand::Miner(MinerCommand::Run(MinerRunArgs {
            wallet: path("miner.key"),
            device: miner_device("cpu"),
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
    let identity_seed = "11".repeat(32);
    assert_eq!(
        parse_test_cli(&[
            "miner",
            "run",
            "--wallet",
            "miner.key",
            "--device",
            "cpu",
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
        TvmdCommand::Miner(MinerCommand::Run(MinerRunArgs {
            wallet: path("miner.key"),
            device: miner_device("cpu"),
            runtime: role_runtime_args(
                "/ip4/127.0.0.1/tcp/4001",
                "127.0.0.1:8545",
                "/ip4/127.0.0.1/tcp/0",
                "/var/lib/tensorvm",
                Some([0x11; 32]),
                "secret",
                7,
            ),
        }))
    );
}

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
            wallet: path("validator.key"),
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
            wallet: path("validator.key"),
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
            wallet: path("validator.key"),
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
            wallet: path("proposer.key"),
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
            wallet: path("proposer.key"),
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
