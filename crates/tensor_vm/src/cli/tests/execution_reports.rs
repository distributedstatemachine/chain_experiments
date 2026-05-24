use super::*;

#[test]
fn execute_command_fixture_reports_public_evidence_outputs() {
    let publication = execute_command_fixture(&CommandFixture::PublicEvidencePublication {
        bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
        public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
        manifest_signer: address(b"public-evidence-publisher"),
        manifest_signature_count: 1,
        independent_auditor_count: 1,
    })
    .unwrap();
    let bundle_id = manifest_hash(b"public-evidence-bundle");
    let manifest_signer = manifest_address(b"public-evidence-publisher");
    let manifest_signature = manifest_publication_signature();
    assert_report_fields(
        &publication,
        &[
            ("bundle_id", bundle_id.as_str()),
            (
                "public_uri",
                "https://tensorvm.net/tensorvm/public-evidence.json",
            ),
            ("manifest_signer", manifest_signer.as_str()),
            ("manifest_signature", manifest_signature.as_str()),
            ("manifest_signature_count", "1"),
            ("independent_auditor_count", "1"),
        ],
    );

    let auditor_record = execute_command_fixture(&CommandFixture::PublicEvidenceAuditorRecord {
        bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
        public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
        auditor_id: address(b"public-evidence-auditor-0"),
        audit_uri: manifest_auditor_uri(),
        observed_at_unix_seconds: 1_700_000_060,
    })
    .unwrap();
    assert_eq!(
        auditor_record,
        format!(
            "auditor={},{},1700000060,{}",
            manifest_address(b"public-evidence-auditor-0"),
            manifest_auditor_uri(),
            manifest_auditor_signature()
        )
    );

    let run_window = execute_command_fixture(&CommandFixture::PublicEvidenceRunWindow {
        bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
        manifest_signer: address(b"public-evidence-publisher"),
        run_started_at_unix_seconds: 1_700_000_000,
        run_ended_at_unix_seconds: 1_700_000_060,
        observed_blocks: 10,
    })
    .unwrap();
    assert_eq!(
        run_window,
        format!(
            "run_started_at_unix_seconds=1700000000\nrun_ended_at_unix_seconds=1700000060\nrun_window_signature={}\nobserved_blocks=10",
            hex(&manifest_bundle().run_window_signature)
        )
    );
    let run_window_observation_file = std::env::temp_dir().join(format!(
        "tensor-vm-run-window-{}.records",
        std::process::id()
    ));
    let run_window_observations = (0..10)
        .map(|block| {
            let timestamp = if block == 9 {
                1_700_000_060
            } else {
                1_700_000_000 + block * 6
            };
            format!("run_window_observation={block},{timestamp}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&run_window_observation_file, run_window_observations).unwrap();
    let run_window_from_file =
        execute_command_fixture(&CommandFixture::PublicEvidenceRunWindowFromFile {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            block_observation_file: run_window_observation_file.to_string_lossy().into_owned(),
        })
        .unwrap();
    std::fs::remove_file(&run_window_observation_file).unwrap();
    assert_eq!(run_window_from_file, run_window);

    let node_cases = [
        (
            PublicNodeRole::Miner,
            b"miner-a".as_slice(),
            b"miner-a-operator".as_slice(),
            "miner",
        ),
        (
            PublicNodeRole::Validator,
            b"validator-a".as_slice(),
            b"validator-a-operator".as_slice(),
            "validator",
        ),
    ];
    for (role, address_label, operator_label, tag) in node_cases {
        let node = execute_command_fixture(&CommandFixture::PublicEvidenceNodeHeartbeat {
            role,
            address: address(address_label),
            operator_id: hash_bytes(b"test", &[operator_label]),
            first_seen_block: 0,
            last_seen_block: 9,
            signed_heartbeat_count: 10,
        })
        .unwrap();
        let node_address = hex(&address(address_label));
        let operator_id = hex(&hash_bytes(b"test", &[operator_label]));
        let node_signature = manifest_node_signature(role, address_label, operator_label);
        assert_eq!(
            comma_record_fields(&node, "node=", 7),
            [
                tag,
                node_address.as_str(),
                operator_id.as_str(),
                "0",
                "9",
                "10",
                node_signature.as_str(),
            ]
        );
        let heartbeat_file = std::env::temp_dir().join(format!(
            "tensor-vm-node-heartbeat-{}-{}.records",
            std::process::id(),
            tag
        ));
        let heartbeat_records = (0..10)
            .map(|block| {
                format!(
                    "node_heartbeat_observation={tag},{},{},{}",
                    node_address, operator_id, block
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&heartbeat_file, heartbeat_records).unwrap();
        let node_from_file =
            execute_command_fixture(&CommandFixture::PublicEvidenceNodeHeartbeatFromFile {
                role,
                address: address(address_label),
                operator_id: hash_bytes(b"test", &[operator_label]),
                heartbeat_file: heartbeat_file.to_string_lossy().into_owned(),
            })
            .unwrap();
        std::fs::remove_file(&heartbeat_file).unwrap();
        assert_eq!(node_from_file, node);
    }

    let operator_id = hash_bytes(b"test", &[b"miner-a-operator"]);
    let operator_identity_uri = manifest_operator_identity_uri(&operator_id);
    let operator_attestation =
        execute_command_fixture(&CommandFixture::PublicEvidenceOperatorAttestation {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id,
            identity_uri: operator_identity_uri.clone(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .unwrap();
    assert_eq!(
        operator_attestation,
        format!(
            "operator=miner,{},{},{operator_identity_uri},1700000000,{}",
            manifest_address(b"miner-a"),
            manifest_hash(b"miner-a-operator"),
            manifest_operator_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator")
        )
    );

    let service_health = execute_command_fixture(&CommandFixture::PublicEvidenceServiceHealth {
        kind: PublicServiceKind::Rpc,
        endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
        public_url: "https://rpc.tensorvm.net/health".to_owned(),
        health_path: "/health".to_owned(),
        first_seen_block: 0,
        last_seen_block: 9,
        reachable_observation_count: 10,
        signed_health_check_count: 10,
    })
    .unwrap();
    let rpc_service_id = manifest_hash(b"rpc-service");
    let rpc_service_signature = manifest_service_signature(PublicServiceKind::Rpc, b"rpc-service");
    assert_eq!(
        comma_record_fields(&service_health, "service=", 9),
        [
            "rpc",
            rpc_service_id.as_str(),
            "https://rpc.tensorvm.net/health",
            "/health",
            "0",
            "9",
            "10",
            "10",
            rpc_service_signature.as_str(),
        ]
    );
    let health_observation_file = std::env::temp_dir().join(format!(
        "tensor-vm-service-health-{}-{}.records",
        std::process::id(),
        manifest_hash(b"rpc-service").as_bytes()[0]
    ));
    let health_observations = (0..10)
        .map(|block| format!("service_health_observation={block},reachable"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&health_observation_file, health_observations).unwrap();
    let service_health_from_file =
        execute_command_fixture(&CommandFixture::PublicEvidenceServiceHealthFromFile {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            observation_file: health_observation_file.to_string_lossy().into_owned(),
        })
        .unwrap();
    std::fs::remove_file(&health_observation_file).unwrap();
    assert_eq!(service_health_from_file, service_health);
    let additional_service_cases: [(PublicServiceKind, &[u8], &str); 3] = [
        (PublicServiceKind::Explorer, b"explorer-service", "explorer"),
        (PublicServiceKind::Faucet, b"faucet-service", "faucet"),
        (
            PublicServiceKind::Telemetry,
            b"telemetry-service",
            "telemetry",
        ),
    ];
    for (kind, label, tag) in additional_service_cases {
        let line = execute_command_fixture(&CommandFixture::PublicEvidenceServiceHealth {
            kind,
            endpoint_id: hash_bytes(b"test", &[label]),
            public_url: public_service_url(kind).to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .unwrap();
        let endpoint_id = manifest_hash(label);
        let service_signature = manifest_service_signature(kind, label);
        assert_eq!(
            comma_record_fields(&line, "service=", 9),
            [
                tag,
                endpoint_id.as_str(),
                public_service_url(kind),
                "/health",
                "0",
                "9",
                "10",
                "10",
                service_signature.as_str(),
            ]
        );
    }

    let service_content = execute_command_fixture(&CommandFixture::PublicEvidenceServiceContent {
        kind: PublicServiceKind::Rpc,
        endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
        public_url: public_service_content_url(PublicServiceKind::Rpc).to_owned(),
        content_path: public_service_content_path(PublicServiceKind::Rpc).to_owned(),
        content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
        observed_at_unix_seconds: 1_700_000_000,
        min_content_bytes: 64,
    })
    .unwrap();
    let rpc_content_root = hex(&hash_bytes(b"test", &[b"rpc-service", b"content-root"]));
    let rpc_content_signature =
        public_service_content(PublicServiceKind::Rpc, b"rpc-service").content_signature;
    let rpc_content_signature = hex(&rpc_content_signature);
    assert_eq!(
        comma_record_fields(&service_content, "service_content=", 8),
        [
            "rpc",
            rpc_service_id.as_str(),
            public_service_content_url(PublicServiceKind::Rpc),
            public_service_content_path(PublicServiceKind::Rpc),
            rpc_content_root.as_str(),
            "1700000000",
            "64",
            rpc_content_signature.as_str(),
        ]
    );
    assert_eq!(
        service_content,
        manifest_service_content_line(PublicServiceKind::Rpc, b"rpc-service")
    );
    let observed_content = vec![7_u8; 80];
    let observed_content_root = public_service_content_root(&observed_content);
    let service_content_from_bytes =
        execute_command_fixture(&CommandFixture::PublicEvidenceServiceContentFromBytes {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: public_service_content_url(PublicServiceKind::Rpc).to_owned(),
            content_path: public_service_content_path(PublicServiceKind::Rpc).to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            content_bytes: observed_content.clone(),
        })
        .unwrap();
    let observed_content_root_hex = hex(&observed_content_root);
    let service_content_from_bytes_fields =
        comma_record_fields(&service_content_from_bytes, "service_content=", 8);
    assert_eq!(
        service_content_from_bytes_fields[..7],
        [
            "rpc",
            rpc_service_id.as_str(),
            public_service_content_url(PublicServiceKind::Rpc),
            public_service_content_path(PublicServiceKind::Rpc),
            observed_content_root_hex.as_str(),
            "1700000000",
            "80",
        ]
    );
    let content_file = std::env::temp_dir().join(format!(
        "tensor-vm-service-content-{}-{}.body",
        std::process::id(),
        observed_content_root[0]
    ));
    std::fs::write(&content_file, &observed_content).unwrap();
    let service_content_from_file =
        execute_command_fixture(&CommandFixture::PublicEvidenceServiceContentFromFile {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: public_service_content_url(PublicServiceKind::Rpc).to_owned(),
            content_path: public_service_content_path(PublicServiceKind::Rpc).to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            content_file: content_file.to_string_lossy().into_owned(),
        })
        .unwrap();
    std::fs::remove_file(&content_file).unwrap();
    assert_eq!(service_content_from_file, service_content_from_bytes);

    let peer_id = PeerId::random().to_string();
    let network_observation =
        execute_command_fixture(&CommandFixture::PublicEvidenceNetworkObservation {
            operator_id: hash_bytes(b"test", &[b"network-operator"]),
            peer_id: peer_id.clone(),
            listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            gossip_topic_count: 5,
            request_response_protocol_count: 4,
            bootstrap_peer_count: 2,
            max_transmit_bytes: 1_048_576,
            request_timeout_seconds: 10,
            max_concurrent_streams: 128,
            idle_connection_timeout_seconds: 60,
        })
        .unwrap();
    let observation_input = NetworkObservationEvidenceLine {
        operator_id: hash_bytes(b"test", &[b"network-operator"]),
        peer_id: &peer_id,
        listen_address: "/dns/node-a.tensorvm.net/tcp/4001",
        observed_at_unix_seconds: 1_700_000_000,
        gossip_topic_count: 5,
        request_response_protocol_count: 4,
        bootstrap_peer_count: 2,
        max_transmit_bytes: 1_048_576,
        request_timeout_seconds: 10,
        max_concurrent_streams: 128,
        idle_connection_timeout_seconds: 60,
    };
    let observation_root = network_observation_root(
        &observation_input,
        &peer_id,
        "/dns/node-a.tensorvm.net/tcp/4001",
    );
    let observation_signature = hash_bytes(
        b"tensor-vm-network-runtime-observation-signature-v1",
        &[&observation_input.operator_id, &observation_root],
    );
    assert_eq!(
        network_observation,
        format!(
            "network_runtime_observation={},{peer_id},/dns/node-a.tensorvm.net/tcp/4001,1700000000,5,4,2,1048576,10,128,60,{},{}",
            hex(&observation_input.operator_id),
            hex(&observation_root),
            hex(&observation_signature)
        )
    );
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation,
        )
        .unwrap(),
        observation_root
    );
    let network_observation_bad_peer =
        network_observation.replace(&format!(",{peer_id},"), ",not-a-peer,");
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_bad_peer,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid libp2p peer id"
    );
    let network_observation_bad_multiaddr =
        network_observation.replace(",/dns/node-a.tensorvm.net/tcp/4001,", ",not-a-multiaddr,");
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_bad_multiaddr,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid libp2p multiaddr"
    );
    let network_observation_local_multiaddr = network_observation.replace(
        ",/dns/node-a.tensorvm.net/tcp/4001,",
        ",/ip4/127.0.0.1/tcp/4001,",
    );
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_local_multiaddr,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: network observation address is not public"
    );
    let network_observation_whitespace_field =
        network_observation.replace(&format!(",{peer_id},"), &format!(", {peer_id},"));
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_whitespace_field,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid network observation record line"
    );
    let network_observation_zero_operator =
        network_observation.replace(&hex(&observation_input.operator_id), &"00".repeat(32));
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_zero_operator,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: operator id argument is empty"
    );
    let network_observation_zero_count =
        network_observation.replace(",1700000000,5,4,2,", ",1700000000,0,4,2,");
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_zero_count,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid network observation record line"
    );
    let network_observation_tampered_root = network_observation.replace(
        &hex(&observation_root),
        &hex(&hash_bytes(b"test", &[b"tampered-network-root"])),
    );
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_tampered_root,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid network observation record line"
    );
    let network_observation_tampered_signature = network_observation.replace(
        &hex(&observation_signature),
        &hex(&hash_bytes(b"test", &[b"tampered-network-signature"])),
    );
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &network_observation_tampered_signature,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid network observation record line"
    );
    let service_log = format!(
        "\
command=service_serve
p2p_runtime=libp2p
p2p_peer_id={peer_id}
p2p_gossipsub_topics=5
p2p_request_response_protocols=4
p2p_bootstrap_peers=2
p2p_max_transmit_bytes=1048576
p2p_request_timeout_seconds=10
p2p_max_concurrent_streams=128
p2p_idle_timeout_seconds=60
"
    );
    assert_eq!(
        service_log_field(&service_log, "p2p_peer_id").unwrap(),
        peer_id
    );
    let network_observation_from_service_log = network_observation_evidence_line_from_service_log(
        hash_bytes(b"test", &[b"network-operator"]),
        "/dns/node-a.tensorvm.net/tcp/4001",
        1_700_000_000,
        &service_log,
    )
    .unwrap();
    assert_eq!(network_observation_from_service_log, network_observation);

    let service_log_file = std::env::temp_dir().join(format!(
        "tensor-vm-service-log-{}-{}.log",
        std::process::id(),
        observation_root[0]
    ));
    std::fs::write(&service_log_file, &service_log).unwrap();
    let network_observation_from_file = execute_command_fixture(
        &CommandFixture::PublicEvidenceNetworkObservationFromServiceLog {
            operator_id: hash_bytes(b"test", &[b"network-operator"]),
            listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            service_log: service_log_file.to_string_lossy().into_owned(),
        },
    )
    .unwrap();
    std::fs::remove_file(&service_log_file).unwrap();
    assert_eq!(network_observation_from_file, network_observation);

    assert_eq!(
        execute_command_fixture(
            &CommandFixture::PublicEvidenceNetworkObservationFromServiceLog {
                operator_id: hash_bytes(b"test", &[b"network-operator"]),
                listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                service_log: service_log_file.to_string_lossy().into_owned(),
            }
        )
        .unwrap_err()
        .to_string(),
        "storage error: failed to read service log file"
    );
    assert_eq!(
        network_observation_evidence_line_from_service_log(
            hash_bytes(b"test", &[b"network-operator"]),
            "/dns/node-a.tensorvm.net/tcp/4001",
            1_700_000_000,
            "command=service_init\np2p_runtime=libp2p\n",
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: service log is not service_serve"
    );
    assert_eq!(
        network_observation_evidence_line_from_service_log(
            hash_bytes(b"test", &[b"network-operator"]),
            "/dns/node-a.tensorvm.net/tcp/4001",
            1_700_000_000,
            "command=service_serve\np2p_runtime=disabled\n",
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: service log does not prove libp2p runtime"
    );
    assert_eq!(
        service_log_field("command=service_serve\n", "p2p_peer_id")
            .unwrap_err()
            .to_string(),
        "invalid receipt: missing service log field"
    );
    assert_eq!(
        service_log_field("p2p_runtime=libp2p\np2p_runtime=libp2p\n", "p2p_runtime")
            .unwrap_err()
            .to_string(),
        "invalid receipt: duplicate service log field"
    );
    assert_eq!(
        service_log_field("p2p_runtime= libp2p\n", "p2p_runtime")
            .unwrap_err()
            .to_string(),
        "invalid receipt: invalid service log field"
    );
}

fn comma_record_fields<'a>(line: &'a str, prefix: &str, expected_len: usize) -> Vec<&'a str> {
    let record = line
        .strip_prefix(prefix)
        .unwrap_or_else(|| panic!("record missing prefix {prefix:?}: {line}"));
    let fields = record.split(',').collect::<Vec<_>>();
    assert_eq!(
        fields.len(),
        expected_len,
        "unexpected field count for {prefix:?}: {line}"
    );
    fields
}
