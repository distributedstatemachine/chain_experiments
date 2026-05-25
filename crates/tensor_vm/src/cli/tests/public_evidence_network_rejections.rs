use super::*;

#[test]
fn execute_public_network_evidence_rejects_invalid_args() {
    let peer_id = PeerId::random().to_string();
    let operator_id = hash_bytes(b"test", &[b"network-operator"]);
    let public_listen_address = "/dns/node-a.tensorvm.net/tcp/4001";
    let invalid_network_observations = [
        (
            [0; 32],
            public_listen_address,
            1_700_000_000,
            (5, 3, 2, 1_048_576),
        ),
        (operator_id, public_listen_address, 0, (5, 3, 2, 1_048_576)),
        (
            operator_id,
            public_listen_address,
            1_700_000_000,
            (0, 3, 2, 1_048_576),
        ),
        (
            operator_id,
            public_listen_address,
            1_700_000_000,
            (5, 0, 2, 1_048_576),
        ),
        (
            operator_id,
            public_listen_address,
            1_700_000_000,
            (5, 3, 0, 1_048_576),
        ),
        (
            operator_id,
            public_listen_address,
            1_700_000_000,
            (5, 3, 2, 0),
        ),
        (
            operator_id,
            "/ip4/127.0.0.1/tcp/4001",
            1_700_000_000,
            (5, 3, 2, 1_048_576),
        ),
        (
            operator_id,
            "/ip4/8.8.8.8",
            1_700_000_000,
            (5, 3, 2, 1_048_576),
        ),
        (
            operator_id,
            "/ip4/8.8.8.8/tcp/0",
            1_700_000_000,
            (5, 3, 2, 1_048_576),
        ),
        (
            operator_id,
            "/ip4/8.8.8.8/udp/4001",
            1_700_000_000,
            (5, 3, 2, 1_048_576),
        ),
        (
            operator_id,
            "/ip4/203.0.113.10/tcp/4001",
            1_700_000_000,
            (5, 3, 2, 1_048_576),
        ),
        (
            operator_id,
            "/dns/bad_host.tensorvm.net/tcp/4001",
            1_700_000_000,
            (5, 3, 2, 1_048_576),
        ),
        (
            operator_id,
            "/dns/node.tensorvm.example/tcp/4001",
            1_700_000_000,
            (5, 3, 2, 1_048_576),
        ),
    ];
    for (operator_id, listen_address, observed_at, counts) in invalid_network_observations {
        assert!(
            execute_network_observation(operator_id, &peer_id, listen_address, observed_at, counts)
                .is_err()
        );
    }
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "network",
            "observation",
            "--operator-id",
            &manifest_hash(b"network-operator"),
            "--peer-id",
            "not-a-peer-id",
            "--listen-address",
            "/dns/node-a.tensorvm.net/tcp/4001",
            "--observed-at",
            "1700000000",
            "--gossip-topics",
            "5",
            "--request-response-protocols",
            "3",
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
        .is_err()
    );
    assert!(
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
            "not-a-multiaddr",
            "--observed-at",
            "1700000000",
            "--gossip-topics",
            "5",
            "--request-response-protocols",
            "3",
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
        .is_err()
    );
}

fn execute_network_observation(
    operator_id: [u8; 32],
    peer_id: &str,
    listen_address: &str,
    observed_at: u64,
    counts: (u64, u64, u64, u64),
) -> crate::error::Result<String> {
    let (gossip_topics, request_response_protocols, bootstrap_peers, max_transmit_bytes) = counts;
    execute_public_evidence_command(&EvidenceCommand::Network(
        EvidenceNetworkCommand::Observation(NetworkObservationArgs::new(
            network_observation_target_args(operator_id, listen_address, observed_at),
            peer_id.parse().expect("test peer ID must parse"),
            network_observation_protocol_counts_args(
                gossip_topics,
                request_response_protocols,
                bootstrap_peers,
            ),
            network_observation_transport_limits_args(max_transmit_bytes, 10, 128, 60),
        )),
    ))
}
