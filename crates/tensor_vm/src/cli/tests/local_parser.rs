use super::parser_support::{
    data_dir_args, hash_arg, miner_device, multiaddr, node_runtime_args, path, role_runtime_args,
};
use super::{
    LocalnetCommand, MinerCheckArgs, MinerCommand, MinerRunArgs, NodeBlockArgs, NodeCheckArgs,
    NodeCommand, NodePeerAddArgs, NodePeerCommand, NodeServeArgs, TvmdCommand, parse_test_cli,
};
use libp2p::PeerId;

#[test]
fn parses_documented_node_commands() {
    assert_eq!(
        parse_test_cli(&["node", "init", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        TvmdCommand::Node(NodeCommand::Init(data_dir_args("/var/lib/tensorvm")))
    );
    let bootstrap_peer = PeerId::random().to_string();
    assert_eq!(
        parse_test_cli(&[
            "node",
            "peer",
            "add",
            "--data-dir",
            "/var/lib/tensorvm",
            "--peer-id",
            &bootstrap_peer,
            "--address",
            "/dns/bootstrap.tensorvm.net/tcp/4001",
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::Peer(NodePeerCommand::Add(NodePeerAddArgs {
            data_dir: path("/var/lib/tensorvm"),
            peer_id: bootstrap_peer.parse().expect("test peer ID must parse"),
            address: multiaddr("/dns/bootstrap.tensorvm.net/tcp/4001"),
        })))
    );
    assert_eq!(
        parse_test_cli(&[
            "node",
            "check",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::Check(NodeCheckArgs {
            p2p_listen: multiaddr("/ip4/0.0.0.0/tcp/4001"),
            data_dir: path("/var/lib/tensorvm"),
            identity_seed: None,
        }))
    );
    let identity_seed = "11".repeat(32);
    assert_eq!(
        parse_test_cli(&[
            "node",
            "check",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::Check(NodeCheckArgs {
            p2p_listen: multiaddr("/ip4/0.0.0.0/tcp/4001"),
            data_dir: path("/var/lib/tensorvm"),
            identity_seed: Some(hash_arg([0x11; 32])),
        }))
    );
    assert_eq!(
        parse_test_cli(&[
            "node",
            "serve",
            "--listen",
            "0.0.0.0:8545",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
            "--max-requests",
            "0",
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::Serve(NodeServeArgs {
            runtime: node_runtime_args(
                "0.0.0.0:8545",
                "/ip4/0.0.0.0/tcp/4001",
                "/var/lib/tensorvm",
                None,
                "secret",
                0,
            ),
        }))
    );
    assert_eq!(
        parse_test_cli(&[
            "node",
            "serve",
            "--listen",
            "0.0.0.0:8545",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
            "--auth-token",
            "secret",
            "--max-requests",
            "0",
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::Serve(NodeServeArgs {
            runtime: node_runtime_args(
                "0.0.0.0:8545",
                "/ip4/0.0.0.0/tcp/4001",
                "/var/lib/tensorvm",
                Some([0x11; 32]),
                "secret",
                0,
            ),
        }))
    );
    assert_eq!(
        parse_test_cli(&["node", "status", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        TvmdCommand::Node(NodeCommand::Status(data_dir_args("/var/lib/tensorvm")))
    );
    assert_eq!(
        parse_test_cli(&[
            "node",
            "block",
            "--data-dir",
            "/var/lib/tensorvm",
            "--height",
            "3"
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::Block(NodeBlockArgs {
            data_dir: path("/var/lib/tensorvm"),
            height: 3,
        }))
    );
}

#[test]
fn parses_documented_localnet_commands() {
    assert_eq!(
        parse_test_cli(&["localnet", "seed", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        TvmdCommand::Localnet(LocalnetCommand::Seed(data_dir_args("/var/lib/tensorvm")))
    );
}

#[test]
fn rejects_invalid_local_cli() {
    assert!(parse_test_cli(&["miner", "register"]).is_err());
    assert!(parse_test_cli(&["validator", "register", "--stake", "abc"]).is_err());
    assert!(
        parse_test_cli(&[
            "node",
            "serve",
            "--listen",
            "not-a-socket",
            "--auth-token",
            "secret"
        ])
        .is_err()
    );
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
fn clap_cli_defaults_runtime_arguments() {
    assert_eq!(
        parse_test_cli(&["miner", "check", "--wallet", "miner.key"]).unwrap(),
        TvmdCommand::Miner(MinerCommand::Check(MinerCheckArgs {
            wallet: path("miner.key"),
            device: miner_device("cpu"),
            node: multiaddr("/ip4/127.0.0.1/tcp/4001"),
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
            wallet: path("miner.key"),
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
    assert_eq!(
        parse_test_cli(&["node", "serve", "--auth-token", "secret"]).unwrap(),
        TvmdCommand::Node(NodeCommand::Serve(NodeServeArgs {
            runtime: node_runtime_args(
                "127.0.0.1:8545",
                "/ip4/127.0.0.1/tcp/4001",
                ".tensorvm",
                None,
                "secret",
                0,
            ),
        }))
    );
    assert_eq!(
        parse_test_cli(&["node", "init"]).unwrap(),
        TvmdCommand::Node(NodeCommand::Init(data_dir_args(".tensorvm")))
    );
}
