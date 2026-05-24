use super::*;

#[test]
fn execute_public_evidence_reports_outputs() {
    let service_health = execute_public_evidence_command(&EvidenceCommand::Service(
        EvidenceServiceCommand::Health(ServiceHealthArgs {
            kind: service_kind_arg(PublicServiceKind::Rpc),
            endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            first_block: 0,
            last_block: 9,
            reachable_count: 10,
            signed_health_check_count: 10,
        }),
    ))
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
    let service_health_from_file = execute_public_evidence_command(&EvidenceCommand::Service(
        EvidenceServiceCommand::HealthFile(ServiceHealthFromFileArgs {
            kind: service_kind_arg(PublicServiceKind::Rpc),
            endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            observation_file: health_observation_file.clone(),
        }),
    ))
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
        let line = execute_public_evidence_command(&EvidenceCommand::Service(
            EvidenceServiceCommand::Health(ServiceHealthArgs {
                kind: service_kind_arg(kind),
                endpoint_id: hash_arg(hash_bytes(b"test", &[label])),
                public_url: public_service_url(kind).to_owned(),
                health_path: "/health".to_owned(),
                first_block: 0,
                last_block: 9,
                reachable_count: 10,
                signed_health_check_count: 10,
            }),
        ))
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

    let service_content = execute_public_evidence_command(&EvidenceCommand::Service(
        EvidenceServiceCommand::Content(ServiceContentArgs {
            kind: service_kind_arg(PublicServiceKind::Rpc),
            endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
            public_url: public_service_content_url(PublicServiceKind::Rpc).to_owned(),
            content_path: public_service_content_path(PublicServiceKind::Rpc).to_owned(),
            content_root: hash_arg(hash_bytes(b"test", &[b"rpc-service", b"content-root"])),
            observed_at: 1_700_000_000,
            min_content_bytes: 64,
        }),
    ))
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
    let service_content_from_bytes = execute_public_evidence_command(&EvidenceCommand::Service(
        EvidenceServiceCommand::ContentBytes(ServiceContentFromBytesArgs {
            kind: service_kind_arg(PublicServiceKind::Rpc),
            endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
            public_url: public_service_content_url(PublicServiceKind::Rpc).to_owned(),
            content_path: public_service_content_path(PublicServiceKind::Rpc).to_owned(),
            observed_at: 1_700_000_000,
            content: HexBytesArg::new(observed_content.clone()),
        }),
    ))
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
    let service_content_from_file = execute_public_evidence_command(&EvidenceCommand::Service(
        EvidenceServiceCommand::ContentFile(ServiceContentFromFileArgs {
            kind: service_kind_arg(PublicServiceKind::Rpc),
            endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
            public_url: public_service_content_url(PublicServiceKind::Rpc).to_owned(),
            content_path: public_service_content_path(PublicServiceKind::Rpc).to_owned(),
            observed_at: 1_700_000_000,
            content_file: content_file.clone(),
        }),
    ))
    .unwrap();
    std::fs::remove_file(&content_file).unwrap();
    assert_eq!(service_content_from_file, service_content_from_bytes);

    let peer_id = PeerId::random().to_string();
    let network_observation = execute_public_evidence_command(&EvidenceCommand::Network(
        EvidenceNetworkCommand::Observation(NetworkObservationArgs {
            operator_id: hash_arg(hash_bytes(b"test", &[b"network-operator"])),
            peer_id: peer_id.parse().expect("test peer ID must parse"),
            listen_address: multiaddr_arg("/dns/node-a.tensorvm.net/tcp/4001".to_owned()),
            observed_at: 1_700_000_000,
            gossip_topics: 5,
            request_response_protocols: 4,
            bootstrap_peers: 2,
            max_transmit_bytes: 1_048_576,
            request_timeout_seconds: 10,
            max_concurrent_streams: 128,
            idle_timeout_seconds: 60,
        }),
    ))
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
    let network_observation_from_file = execute_public_evidence_command(&EvidenceCommand::Network(
        EvidenceNetworkCommand::FromServiceLog(NetworkObservationFromServiceLogArgs {
            operator_id: hash_arg(hash_bytes(b"test", &[b"network-operator"])),
            listen_address: multiaddr_arg("/dns/node-a.tensorvm.net/tcp/4001".to_owned()),
            observed_at: 1_700_000_000,
            service_log: service_log_file.clone(),
        }),
    ))
    .unwrap();
    std::fs::remove_file(&service_log_file).unwrap();
    assert_eq!(network_observation_from_file, network_observation);

    assert_eq!(
        execute_public_evidence_command(&EvidenceCommand::Network(
            EvidenceNetworkCommand::FromServiceLog(NetworkObservationFromServiceLogArgs {
                operator_id: hash_arg(hash_bytes(b"test", &[b"network-operator"])),
                listen_address: multiaddr_arg("/dns/node-a.tensorvm.net/tcp/4001".to_owned()),
                observed_at: 1_700_000_000,
                service_log: service_log_file.clone(),
            }),
        ))
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
