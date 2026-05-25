use super::parser_support::{
    data_dir_args, miner_device, role_node_args, role_runtime_args, role_wallet_args,
};
use super::{
    LocalnetCommand, MinerCheckArgs, MinerCommand, MinerRunArgs, TvmdCommand, parse_test_cli,
};

#[test]
fn parses_documented_localnet_commands() {
    assert_eq!(
        parse_test_cli(&["localnet", "seed", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        TvmdCommand::Localnet(LocalnetCommand::Seed(data_dir_args("/var/lib/tensorvm")))
    );
}

#[test]
fn rejects_invalid_local_role_cli() {
    assert!(parse_test_cli(&["miner", "register"]).is_err());
    assert!(parse_test_cli(&["validator", "register", "--stake", "abc"]).is_err());
    assert!(
        parse_test_cli(&[
            "miner",
            "run",
            "--wallet",
            "miner.key",
            "--node",
            "not-a-multiaddr",
            "--auth-token",
            "secret"
        ])
        .is_err()
    );
}

#[test]
fn clap_role_defaults_runtime_arguments() {
    assert_eq!(
        parse_test_cli(&["miner", "check", "--wallet", "miner.key"]).unwrap(),
        TvmdCommand::Miner(MinerCommand::Check(MinerCheckArgs {
            wallet: role_wallet_args("miner.key"),
            device: miner_device("cpu"),
            node: role_node_args("/ip4/127.0.0.1/tcp/4001"),
        }))
    );
    assert_eq!(
        parse_test_cli(&[
            "miner",
            "run",
            "--wallet",
            "miner.key",
            "--auth-token",
            "secret"
        ])
        .unwrap(),
        TvmdCommand::Miner(MinerCommand::Run(MinerRunArgs {
            wallet: role_wallet_args("miner.key"),
            device: miner_device("cpu"),
            runtime: role_runtime_args(
                "/ip4/127.0.0.1/tcp/4001",
                "127.0.0.1:8545",
                "/ip4/127.0.0.1/tcp/4001",
                ".tensorvm",
                None,
                "secret",
                0,
            ),
        }))
    );
}
