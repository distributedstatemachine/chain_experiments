use super::parser_support::{hash_arg, multiaddr, path};
use super::{
    EvidenceCommand, EvidenceNetworkCommand, NetworkObservationArgs,
    NetworkObservationFromServiceLogArgs, PublicCommand, TvmdCommand, manifest_hash,
    parse_test_cli,
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
            EvidenceNetworkCommand::Observation(NetworkObservationArgs {
                operator_id: hash_arg(hash_bytes(b"test", &[b"network-operator"])),
                peer_id: peer_id.parse().expect("test peer ID must parse"),
                listen_address: multiaddr("/dns/node-a.tensorvm.net/tcp/4001"),
                observed_at: 1_700_000_000,
                gossip_topics: 5,
                request_response_protocols: 4,
                bootstrap_peers: 2,
                max_transmit_bytes: 1_048_576,
                request_timeout_seconds: 10,
                max_concurrent_streams: 128,
                idle_timeout_seconds: 60,
            }),
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
            EvidenceNetworkCommand::FromServiceLog(NetworkObservationFromServiceLogArgs {
                operator_id: hash_arg(hash_bytes(b"test", &[b"network-operator"])),
                listen_address: multiaddr("/dns/node-a.tensorvm.net/tcp/4001"),
                observed_at: 1_700_000_000,
                service_log: path("artifacts/node-a-service.log"),
            }),
        )))
    );
}
