use super::*;

#[test]
fn execute_command_fixture_rejects_invalid_public_evidence_args() {
    let peer_id = PeerId::random().to_string();
    let make_network_observation = |operator_id,
                                    peer_id: String,
                                    listen_address: String,
                                    observed_at_unix_seconds,
                                    gossip_topic_count,
                                    request_response_protocol_count,
                                    bootstrap_peer_count,
                                    max_transmit_bytes| {
        CommandFixture::PublicEvidenceNetworkObservation {
            operator_id,
            peer_id,
            listen_address,
            observed_at_unix_seconds,
            gossip_topic_count,
            request_response_protocol_count,
            bootstrap_peer_count,
            max_transmit_bytes,
            request_timeout_seconds: 10,
            max_concurrent_streams: 128,
            idle_connection_timeout_seconds: 60,
        }
    };
    let operator_id = hash_bytes(b"test", &[b"network-operator"]);
    let public_listen_address = "/dns/node-a.tensorvm.net/tcp/4001".to_owned();
    for invalid in [
        make_network_observation(
            [0; 32],
            peer_id.clone(),
            public_listen_address.clone(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            public_listen_address.clone(),
            0,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            public_listen_address.clone(),
            1_700_000_000,
            0,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            public_listen_address.clone(),
            1_700_000_000,
            5,
            0,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            public_listen_address.clone(),
            1_700_000_000,
            5,
            3,
            0,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            public_listen_address.clone(),
            1_700_000_000,
            5,
            3,
            2,
            0,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            "/ip4/127.0.0.1/tcp/4001".to_owned(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            "/ip4/8.8.8.8".to_owned(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            "/ip4/8.8.8.8/tcp/0".to_owned(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            "/ip4/8.8.8.8/udp/4001".to_owned(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            "/ip4/203.0.113.10/tcp/4001".to_owned(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            "/dns/bad_host.tensorvm.net/tcp/4001".to_owned(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
        make_network_observation(
            operator_id,
            peer_id.clone(),
            "/dns/node.tensorvm.example/tcp/4001".to_owned(),
            1_700_000_000,
            5,
            3,
            2,
            1_048_576,
        ),
    ] {
        assert!(execute_command_fixture(&invalid).is_err());
    }
    assert!(
        parse_test_cli(&[
            "public-evidence",
            "network-observation",
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
            "public-evidence",
            "network-observation",
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
        execute_command_fixture(&CommandFixture::PublicEvidenceRunWindow {
            bundle_id: [0; 32],
            manifest_signer: address(b"public-evidence-publisher"),
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 10,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceRunWindow {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: [0; 32],
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 10,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceRunWindow {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            run_started_at_unix_seconds: 1_700_000_060,
            run_ended_at_unix_seconds: 1_700_000_000,
            observed_blocks: 10,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceRunWindow {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 0,
        })
        .is_err()
    );
    let run_window_summary = run_window_observation_summary_from_file(
        "run_window_observation=7,1700000000\nrun_window_observation=8,1700000006\n",
    )
    .unwrap();
    assert_eq!(
        run_window_summary.run_started_at_unix_seconds,
        1_700_000_000
    );
    assert_eq!(run_window_summary.run_ended_at_unix_seconds, 1_700_000_006);
    assert_eq!(run_window_summary.observed_blocks, 2);
    for invalid_run_window_observations in [
        "# no observations\n\n",
        " run_window_observation=0,1700000000\n",
        "run_window_observation=0,1700000000\nrun_window_observation=0,1700000001\n",
        "run_window_observation=0,1700000000\nrun_window_observation=2,1700000012\n",
        "run_window_observation=0,1700000006\nrun_window_observation=1,1700000000\n",
        "run_window_observation=0,0\n",
        "run_window_observation=0\n",
        "service_health_observation=0,reachable\n",
    ] {
        assert!(run_window_observation_summary_from_file(invalid_run_window_observations).is_err());
    }
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceRunWindowFromFile {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            block_observation_file: std::env::temp_dir()
                .join(format!(
                    "missing-tensor-vm-run-window-{}.records",
                    std::process::id()
                ))
                .to_string_lossy()
                .into_owned(),
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceNodeHeartbeat {
            role: PublicNodeRole::Miner,
            address: [0; 32],
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            first_seen_block: 0,
            last_seen_block: 9,
            signed_heartbeat_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceNodeHeartbeat {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: [0; 32],
            first_seen_block: 0,
            last_seen_block: 9,
            signed_heartbeat_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceNodeHeartbeat {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            first_seen_block: 10,
            last_seen_block: 9,
            signed_heartbeat_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceNodeHeartbeat {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            first_seen_block: 0,
            last_seen_block: 9,
            signed_heartbeat_count: 0,
        })
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
        execute_command_fixture(&CommandFixture::PublicEvidenceNodeHeartbeatFromFile {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            heartbeat_file: std::env::temp_dir()
                .join(format!(
                    "missing-tensor-vm-node-heartbeat-{}.records",
                    std::process::id()
                ))
                .to_string_lossy()
                .into_owned(),
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceOperatorAttestation {
            role: PublicNodeRole::Miner,
            address: [0; 32],
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            identity_uri: "https://operators.tensorvm.net/miner-a".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceOperatorAttestation {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: [0; 32],
            identity_uri: "https://operators.tensorvm.net/miner-a".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceOperatorAttestation {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            identity_uri: "https://localhost/miner-a".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceOperatorAttestation {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            identity_uri: "https://operators.tensorvm.net/".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceOperatorAttestation {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            identity_uri: "https://operators.tensorvm.net/miner-a".to_owned(),
            observed_at_unix_seconds: 0,
        })
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "public-evidence",
            "record-summary",
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
            "public-evidence",
            "record-artifact",
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
            "public-evidence",
            "record-summary-from-roots",
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
            "public-evidence",
            "record-summary-from-roots",
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
            "public-evidence",
            "record-artifact-from-roots",
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
    let valid_record_summary = CommandFixture::PublicEvidenceRecordSummary {
        kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
        bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
        manifest_signer: address(b"public-evidence-publisher"),
        record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
        record_count: 4,
    };
    assert!(execute_command_fixture(&valid_record_summary).is_ok());
    let valid_record_artifact = CommandFixture::PublicEvidenceRecordArtifact {
        kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
        bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
        manifest_signer: address(b"public-evidence-publisher"),
        artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
        record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
        record_count: 4,
    };
    assert!(execute_command_fixture(&valid_record_artifact).is_ok());
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceRecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: [0; 32],
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceRecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: [0; 32],
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceRecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: [0; 32],
            record_count: 4,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceRecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 0,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceRecordArtifact {
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
        execute_command_fixture(&CommandFixture::PublicEvidenceRecordArtifact {
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
        execute_command_fixture(&CommandFixture::PublicEvidenceRecordArtifact {
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
        execute_command_fixture(&CommandFixture::PublicEvidenceRecordArtifact {
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
        execute_command_fixture(&CommandFixture::PublicEvidenceRecordArtifact {
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
        execute_command_fixture(&CommandFixture::PublicEvidenceRecordArtifact {
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
        execute_command_fixture(&CommandFixture::PublicEvidenceRecordSummaryFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_roots: Vec::new(),
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceRecordSummaryFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_roots: vec![[0; 32]],
        })
        .is_err()
    );
    let duplicate_record_root = hash_bytes(b"test", &[b"network-runtime-root"]);
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceRecordSummaryFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_roots: vec![duplicate_record_root, duplicate_record_root],
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceRecordArtifactFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_roots: Vec::new(),
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::PublicEvidenceRecordArtifactFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_roots: vec![duplicate_record_root, duplicate_record_root],
        })
        .is_err()
    );
}
