use super::parser_support::{address_arg, hash_arg, path};
use super::{
    EvidenceCommand, EvidenceNodeCommand, NodeHeartbeatArgs, NodeHeartbeatFromFileArgs,
    OperatorAttestationArgs, PublicCommand, PublicNodeIdentityArgs, PublicNodeRoleArg, TvmdCommand,
    block_height_window_args, manifest_address, manifest_hash, parse_test_cli,
};
use crate::types::{address, hash_bytes};

#[test]
fn parses_node_evidence_commands() {
    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "node",
            "heartbeat",
            "--role",
            "miner",
            "--address",
            &manifest_address(b"miner-a"),
            "--operator-id",
            &manifest_hash(b"miner-a-operator"),
            "--first-block",
            "0",
            "--last-block",
            "9",
            "--heartbeat-count",
            "10",
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Node(
            EvidenceNodeCommand::Heartbeat(NodeHeartbeatArgs {
                node: miner_node_identity_args(),
                window: block_height_window_args(0, 9),
                heartbeat_count: 10,
            }),
        )))
    );

    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "node",
            "heartbeat-file",
            "--role",
            "miner",
            "--address",
            &manifest_address(b"miner-a"),
            "--operator-id",
            &manifest_hash(b"miner-a-operator"),
            "--heartbeat-file",
            "artifacts/miner-a-heartbeats.records",
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Node(
            EvidenceNodeCommand::HeartbeatFile(NodeHeartbeatFromFileArgs {
                node: miner_node_identity_args(),
                heartbeat_file: path("artifacts/miner-a-heartbeats.records"),
            }),
        )))
    );

    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "node",
            "operator-attestation",
            "--role",
            "miner",
            "--address",
            &manifest_address(b"miner-a"),
            "--operator-id",
            &manifest_hash(b"miner-a-operator"),
            "--identity-uri",
            "https://operators.tensorvm.net/miner-a",
            "--observed-at",
            "1700000000",
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Node(
            EvidenceNodeCommand::OperatorAttestation(OperatorAttestationArgs {
                node: miner_node_identity_args(),
                identity_uri: "https://operators.tensorvm.net/miner-a".to_owned(),
                observed_at: 1_700_000_000,
            }),
        )))
    );
}

fn miner_node_identity_args() -> PublicNodeIdentityArgs {
    PublicNodeIdentityArgs {
        role: PublicNodeRoleArg::Miner,
        address: address_arg(address(b"miner-a")),
        operator_id: hash_arg(hash_bytes(b"test", &[b"miner-a-operator"])),
    }
}
