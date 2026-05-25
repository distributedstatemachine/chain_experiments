use super::parser_support::{multiaddr, operator_id_args, path};
use super::{
    EvidenceCommand, EvidenceNetworkCommand, NetworkObservationArgs,
    NetworkObservationFromServiceLogArgs, NetworkObservationProtocolCountsArgs,
    NetworkObservationTargetArgs, NetworkObservationTransportLimitsArgs, PublicCommand,
    TvmdCommand, manifest_hash, observation_timestamp_args, parse_test_cli,
};
use crate::types::hash_bytes;
use libp2p::PeerId;

#[test]
fn parses_network_evidence_commands() {
    let peer_id = PeerId::random().to_string();
    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "network",
            "observation",
            "--operator-id",
            &manifest_hash(b"network-operator"),
            "--peer-id",
            &peer_id,
            "--listen-address",
            "/dns/node-a.tensorvm.net/tcp/4001",
            "--observed-at",
            "1700000000",
            "--gossip-topics",
            "5",
            "--request-response-protocols",
            "4",
            "--bootstrap-peers",
            "2",
            "--max-transmit-bytes",
            "1048576",
            "--request-timeout-seconds",
            "10",
            "--max-concurrent-streams",
            "128",
            "--idle-timeout-seconds",
            "60",
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Network(
            EvidenceNetworkCommand::Observation(NetworkObservationArgs::new(
                network_observation_target_args(),
                peer_id.parse().expect("test peer ID must parse"),
                NetworkObservationProtocolCountsArgs::new(5, 4, 2),
                NetworkObservationTransportLimitsArgs::new(1_048_576, 10, 128, 60),
            )),
        )))
    );

    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "network",
            "from-service-log",
            "--operator-id",
            &manifest_hash(b"network-operator"),
            "--listen-address",
            "/dns/node-a.tensorvm.net/tcp/4001",
            "--observed-at",
            "1700000000",
            "--service-log",
            "artifacts/node-a-service.log",
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Network(
            EvidenceNetworkCommand::FromServiceLog(NetworkObservationFromServiceLogArgs::new(
                network_observation_target_args(),
                path("artifacts/node-a-service.log"),
            )),
        )))
    );
}

fn network_observation_target_args() -> NetworkObservationTargetArgs {
    NetworkObservationTargetArgs::new(
        operator_id_args(hash_bytes(b"test", &[b"network-operator"])),
        multiaddr("/dns/node-a.tensorvm.net/tcp/4001"),
        observation_timestamp_args(1_700_000_000),
    )
}
