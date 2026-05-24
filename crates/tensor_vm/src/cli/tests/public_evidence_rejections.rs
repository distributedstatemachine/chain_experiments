use super::*;

#[test]
fn execute_evidence_fixture_rejects_invalid_public_evidence_args() {
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
    assert!(parse_public_service_kind("archive").is_err());
    assert_eq!(
        parse_public_node_role("miner").unwrap(),
        PublicNodeRole::Miner
    );
    assert_eq!(
        parse_public_node_role("validator").unwrap(),
        PublicNodeRole::Validator
    );
    assert!(parse_public_node_role("observer").is_err());
    assert_eq!(
        parse_public_evidence_record_kind("block-history").unwrap(),
        PublicEvidenceRecordKind::BlockHistory
    );
    assert_eq!(
        parse_public_evidence_record_kind("finality-history").unwrap(),
        PublicEvidenceRecordKind::FinalityHistory
    );
    assert_eq!(
        parse_public_evidence_record_kind("network-runtime").unwrap(),
        PublicEvidenceRecordKind::NetworkRuntimeObservations
    );
    assert_eq!(
        parse_public_evidence_record_kind("data-availability").unwrap(),
        PublicEvidenceRecordKind::DataAvailabilityMeasurements
    );
    assert_eq!(
        parse_public_evidence_record_kind("invalid-work").unwrap(),
        PublicEvidenceRecordKind::InvalidWorkRejections
    );
    assert_eq!(
        parse_public_evidence_record_kind("reward-settlement").unwrap(),
        PublicEvidenceRecordKind::RewardSettlements
    );
    assert!(parse_public_evidence_record_kind("operator-identity").is_err());
    assert!(parse_hash_argument("12").is_err());
    assert!(parse_hash_argument(&"g".repeat(64)).is_err());
    assert!(
        execute_node_heartbeat(
            [0; 32],
            hash_bytes(b"test", &[b"miner-a-operator"]),
            0,
            9,
            10
        )
        .is_err()
    );
    assert!(execute_node_heartbeat(address(b"miner-a"), [0; 32], 0, 9, 10).is_err());
    assert!(
        execute_node_heartbeat(
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-a-operator"]),
            10,
            9,
            10,
        )
        .is_err()
    );
    assert!(
        execute_node_heartbeat(
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-a-operator"]),
            0,
            9,
            0,
        )
        .is_err()
    );
    let miner_address_hex = manifest_address(b"miner-a");
    let miner_operator_hex = manifest_hash(b"miner-a-operator");
    let heartbeat_summary = node_heartbeat_observation_summary_from_file(
            PublicNodeRole::Miner,
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-a-operator"]),
            &format!(
                "node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\nnode_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},1\n"
            ),
        )
        .unwrap();
    assert_eq!(heartbeat_summary.first_seen_block, 0);
    assert_eq!(heartbeat_summary.last_seen_block, 1);
    assert_eq!(heartbeat_summary.signed_heartbeat_count, 2);
    for invalid_heartbeat_observations in [
        "# no observations\n\n".to_owned(),
        format!(" node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\n"),
        format!(
            "node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\nnode_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\n"
        ),
        format!(
            "node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\nnode_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},2\n"
        ),
        format!(
            "node_heartbeat_observation=validator,{miner_address_hex},{miner_operator_hex},0\n"
        ),
        format!(
            "node_heartbeat_observation=miner,{},{} ,0\n",
            miner_address_hex, miner_operator_hex
        ),
        format!("node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex}\n"),
        "service_health_observation=0,reachable\n".to_owned(),
    ] {
        assert!(
            node_heartbeat_observation_summary_from_file(
                PublicNodeRole::Miner,
                address(b"miner-a"),
                hash_bytes(b"test", &[b"miner-a-operator"]),
                &invalid_heartbeat_observations,
            )
            .is_err()
        );
    }
    assert!(
        execute_node_heartbeat_file(std::env::temp_dir().join(format!(
            "missing-tensor-vm-node-heartbeat-{}.records",
            std::process::id()
        )))
        .is_err()
    );
    assert!(
        execute_operator_attestation(
            [0; 32],
            hash_bytes(b"test", &[b"miner-a-operator"]),
            "https://operators.tensorvm.net/miner-a",
            1_700_000_000,
        )
        .is_err()
    );
    assert!(
        execute_operator_attestation(
            address(b"miner-a"),
            [0; 32],
            "https://operators.tensorvm.net/miner-a",
            1_700_000_000,
        )
        .is_err()
    );
    assert!(
        execute_operator_attestation(
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-a-operator"]),
            "https://localhost/miner-a",
            1_700_000_000,
        )
        .is_err()
    );
    assert!(
        execute_operator_attestation(
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-a-operator"]),
            "https://operators.tensorvm.net/",
            1_700_000_000,
        )
        .is_err()
    );
    assert!(
        execute_operator_attestation(
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-a-operator"]),
            "https://operators.tensorvm.net/miner-a",
            0,
        )
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "summary",
            "--kind",
            "operator-identity",
            "--bundle-id",
            &manifest_hash(b"public-evidence-bundle"),
            "--manifest-signer",
            &manifest_address(b"public-evidence-publisher"),
            "--record-root",
            &manifest_hash(b"network-runtime-root"),
            "--record-count",
            "4",
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "artifact",
            "--kind",
            "operator-identity",
            "--bundle-id",
            &manifest_hash(b"public-evidence-bundle"),
            "--manifest-signer",
            &manifest_address(b"public-evidence-publisher"),
            "--artifact-uri",
            "https://evidence.tensorvm.net/network-runtime.json",
            "--record-root",
            &manifest_hash(b"network-runtime-root"),
            "--record-count",
            "4",
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "summary-roots",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &manifest_hash(b"public-evidence-bundle"),
            "--manifest-signer",
            &manifest_address(b"public-evidence-publisher"),
            "--record-roots",
            "",
        ])
        .is_err()
    );
    let root_a = manifest_hash(b"network-observation-a");
    let root_b = manifest_hash(b"network-observation-b");
    let padded_roots = format!("{root_a}, {root_b}");
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "summary-roots",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &manifest_hash(b"public-evidence-bundle"),
            "--manifest-signer",
            &manifest_address(b"public-evidence-publisher"),
            "--record-roots",
            &padded_roots,
        ])
        .is_err()
    );
    let empty_root_entry = format!("{root_a},,{root_b}");
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "artifact-roots",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &manifest_hash(b"public-evidence-bundle"),
            "--manifest-signer",
            &manifest_address(b"public-evidence-publisher"),
            "--artifact-uri",
            "https://evidence.tensorvm.net/network-runtime.json",
            "--record-roots",
            &empty_root_entry,
        ])
        .is_err()
    );
    let valid_record_summary = EvidenceFixture::RecordSummary {
        kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
        bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
        manifest_signer: address(b"public-evidence-publisher"),
        record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
        record_count: 4,
    };
    assert!(execute_evidence_fixture(&valid_record_summary).is_ok());
    let valid_record_artifact = EvidenceFixture::RecordArtifact {
        kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
        bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
        manifest_signer: address(b"public-evidence-publisher"),
        artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
        record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
        record_count: 4,
    };
    assert!(execute_evidence_fixture(&valid_record_artifact).is_ok());
    assert!(
        execute_evidence_fixture(&EvidenceFixture::RecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: [0; 32],
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::RecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: [0; 32],
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::RecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: [0; 32],
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::RecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 0,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::RecordArtifact {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: [0; 32],
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::RecordArtifact {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: [0; 32],
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::RecordArtifact {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://localhost/network-runtime.json".to_owned(),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::RecordArtifact {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/".to_owned(),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::RecordArtifact {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_root: [0; 32],
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::RecordArtifact {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 0,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::RecordSummaryFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_roots: Vec::new(),
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::RecordSummaryFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_roots: vec![[0; 32]],
        })
        .is_err()
    );
    let duplicate_record_root = hash_bytes(b"test", &[b"network-runtime-root"]);
    assert!(
        execute_evidence_fixture(&EvidenceFixture::RecordSummaryFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_roots: vec![duplicate_record_root, duplicate_record_root],
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::RecordArtifactFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_roots: Vec::new(),
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::RecordArtifactFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_roots: vec![duplicate_record_root, duplicate_record_root],
        })
        .is_err()
    );
}

fn execute_node_heartbeat(
    address: [u8; 32],
    operator_id: [u8; 32],
    first_block: u64,
    last_block: u64,
    heartbeat_count: u64,
) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Node(EvidenceNodeCommand::Heartbeat(
        NodeHeartbeatArgs {
            role: node_role_arg(PublicNodeRole::Miner),
            address: address_arg(address),
            operator_id: hash_arg(operator_id),
            first_block,
            last_block,
            heartbeat_count,
        },
    )))
}

fn execute_node_heartbeat_file(heartbeat_file: std::path::PathBuf) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Node(EvidenceNodeCommand::HeartbeatFile(
        NodeHeartbeatFromFileArgs {
            role: node_role_arg(PublicNodeRole::Miner),
            address: address_arg(address(b"miner-a")),
            operator_id: hash_arg(hash_bytes(b"test", &[b"miner-a-operator"])),
            heartbeat_file,
        },
    )))
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
        EvidenceNetworkCommand::Observation(NetworkObservationArgs {
            operator_id: hash_arg(operator_id),
            peer_id: peer_id.parse().expect("fixture peer ID must parse"),
            listen_address: multiaddr_arg(listen_address.to_owned()),
            observed_at,
            gossip_topics,
            request_response_protocols,
            bootstrap_peers,
            max_transmit_bytes,
            request_timeout_seconds: 10,
            max_concurrent_streams: 128,
            idle_timeout_seconds: 60,
        }),
    ))
}

fn execute_operator_attestation(
    address: [u8; 32],
    operator_id: [u8; 32],
    identity_uri: &str,
    observed_at: u64,
) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Node(
        EvidenceNodeCommand::OperatorAttestation(OperatorAttestationArgs {
            role: node_role_arg(PublicNodeRole::Miner),
            address: address_arg(address),
            operator_id: hash_arg(operator_id),
            identity_uri: identity_uri.to_owned(),
            observed_at,
        }),
    ))
}
