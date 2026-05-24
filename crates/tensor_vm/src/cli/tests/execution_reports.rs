use super::*;

#[test]
fn execute_reference_cli_command_reports_miner_and_validator_readiness() {
    let miner_register =
        execute_reference_cli_command(&ExpectedCommand::MinerRegister { stake: 100 }).unwrap();
    assert!(miner_register.contains("command=miner_register"));
    assert!(miner_register.contains("min_stake=100"));
    assert!(miner_register.contains("stake_sufficient=true"));

    let miner_start = execute_reference_cli_command(&ExpectedCommand::MinerStart {
        wallet: "miner.key".to_owned(),
        device: "cpu".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
    })
    .unwrap();
    assert!(miner_start.contains("command=miner_start"));
    assert!(miner_start.contains("wallet=miner.key"));
    assert!(miner_start.contains("device=cpu"));
    assert!(miner_start.contains("device_backend=cpu-reference"));
    assert!(miner_start.contains(&format!(
        "cuda_kernels_compiled={}",
        cuda_kernels_compiled()
    )));
    assert!(miner_start.contains("node=/ip4/127.0.0.1/tcp/4001"));
    assert!(miner_start.contains(&format!("address={}", hex(&address(b"miner.key")))));
    assert!(miner_start.contains("reference_backend_ready=true"));

    let miner_run = execute_reference_cli_command(&ExpectedCommand::MinerRun {
        wallet: "miner.key".to_owned(),
        device: "cpu".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        listen: "127.0.0.1:8545".to_owned(),
        p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: Some([0x11; 32]),
        auth_token: "secret".to_owned(),
        max_requests: 7,
    })
    .unwrap();
    assert!(miner_run.contains("command=miner_run"));
    assert!(miner_run.contains("role=miner"));
    assert!(miner_run.contains("device_backend=cpu-reference"));
    assert!(miner_run.contains("p2p_runtime=libp2p"));
    assert!(miner_run.contains("p2p_identity_seeded=true"));
    assert!(miner_run.contains("role_runtime_ready=true"));

    let validator_register =
        execute_reference_cli_command(&ExpectedCommand::ValidatorRegister { stake: 10_000 })
            .unwrap();
    assert!(validator_register.contains("command=validator_register"));
    assert!(validator_register.contains("min_stake=10000"));

    let validator_start = execute_reference_cli_command(&ExpectedCommand::ValidatorStart {
        wallet: "validator.key".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
    })
    .unwrap();
    assert!(validator_start.contains("command=validator_start"));
    assert!(validator_start.contains("reference_verifier_ready=true"));

    let validator_run = execute_reference_cli_command(&ExpectedCommand::ValidatorRun {
        wallet: "validator.key".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        listen: "127.0.0.1:8545".to_owned(),
        p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: None,
        auth_token: "secret".to_owned(),
        max_requests: 7,
    })
    .unwrap();
    assert!(validator_run.contains("command=validator_run"));
    assert!(validator_run.contains("role=validator"));
    assert!(validator_run.contains("reference_verifier_ready=true"));
    assert!(validator_run.contains("p2p_runtime=libp2p"));
    assert!(validator_run.contains("p2p_identity_seeded=false"));
    assert!(validator_run.contains("role_runtime_ready=true"));

    let proposer_run = execute_reference_cli_command(&ExpectedCommand::ProposerRun {
        wallet: "proposer.key".to_owned(),
        node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        listen: "127.0.0.1:8545".to_owned(),
        p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: Some([0x33; 32]),
        auth_token: "secret".to_owned(),
        max_requests: 7,
    })
    .unwrap();
    assert!(proposer_run.contains("command=proposer_run"));
    assert!(proposer_run.contains("role=proposer"));
    assert!(proposer_run.contains("proposer_ready=true"));
    assert!(proposer_run.contains("p2p_runtime=libp2p"));
    assert!(proposer_run.contains("p2p_identity_seeded=true"));
    assert!(proposer_run.contains("role_runtime_ready=true"));

    let miner_status = execute_reference_cli_command(&ExpectedCommand::MinerStatus).unwrap();
    assert!(miner_status.contains("command=miner_status"));
    assert!(miner_status.contains("status_source=rpc_or_node_store_required"));

    let validator_status =
        execute_reference_cli_command(&ExpectedCommand::ValidatorStatus).unwrap();
    assert!(validator_status.contains("command=validator_status"));
    assert!(validator_status.contains("status_source=rpc_or_node_store_required"));

    let service_init = execute_reference_cli_command(&ExpectedCommand::ServiceInit {
        data_dir: "/var/lib/tensorvm".to_owned(),
    })
    .unwrap();
    assert!(service_init.contains("command=service_init"));
    assert!(service_init.contains("node_store_ready=true"));

    let bootstrap_peer = PeerId::random().to_string();
    let service_peer_add = execute_reference_cli_command(&ExpectedCommand::ServicePeerAdd {
        data_dir: "/var/lib/tensorvm".to_owned(),
        peer_id: bootstrap_peer.clone(),
        address: "/dns/bootstrap.tensorvm.net/tcp/4001".to_owned(),
    })
    .unwrap();
    assert!(service_peer_add.contains("command=service_peer_add"));
    assert!(service_peer_add.contains(&format!("peer_id={bootstrap_peer}")));
    assert!(service_peer_add.contains("peer_book_ready=true"));

    let service_readiness = execute_reference_cli_command(&ExpectedCommand::ServiceReadiness {
        p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: Some([0x11; 32]),
    })
    .unwrap();
    assert!(service_readiness.contains("command=service_readiness"));
    assert!(service_readiness.contains("p2p_runtime=libp2p"));
    assert!(service_readiness.contains("p2p_gossipsub=enabled"));
    assert!(service_readiness.contains("p2p_identify=enabled"));
    assert!(service_readiness.contains("p2p_kademlia=enabled"));
    assert!(service_readiness.contains("p2p_request_response=enabled"));
    assert!(service_readiness.contains("p2p_identity_seeded=true"));
    assert!(service_readiness.contains(&format!("p2p_identity_seed={}", "11".repeat(32))));
    assert!(service_readiness.contains("p2p_max_transmit_bytes=1048576"));
    assert!(service_readiness.contains("p2p_request_timeout_seconds=10"));
    assert!(service_readiness.contains("p2p_max_concurrent_streams=128"));
    assert!(service_readiness.contains("p2p_idle_timeout_seconds=60"));
    assert!(service_readiness.contains("node_store_required=true"));
    assert!(service_readiness.contains("libp2p_ready=true"));

    let unseeded_service_readiness =
        execute_reference_cli_command(&ExpectedCommand::ServiceReadiness {
            p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
        })
        .unwrap();
    assert!(unseeded_service_readiness.contains("p2p_identity_seeded=false"));

    let service_serve = execute_reference_cli_command(&ExpectedCommand::ServiceServe {
        listen: "0.0.0.0:8545".to_owned(),
        p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
        data_dir: "/var/lib/tensorvm".to_owned(),
        identity_seed: Some([0x22; 32]),
        auth_token: "secret".to_owned(),
        max_requests: 0,
    })
    .unwrap();
    assert!(service_serve.contains("command=service_serve"));
    assert!(service_serve.contains("p2p_runtime=libp2p"));
    assert!(service_serve.contains("p2p_gossipsub=enabled"));
    assert!(service_serve.contains("p2p_identify=enabled"));
    assert!(service_serve.contains("p2p_kademlia=enabled"));
    assert!(service_serve.contains("p2p_request_response=enabled"));
    assert!(service_serve.contains("p2p_identity_seeded=true"));
    assert!(service_serve.contains(&format!("p2p_identity_seed={}", "22".repeat(32))));
    assert!(service_serve.contains("p2p_max_transmit_bytes=1048576"));
    assert!(service_serve.contains("p2p_request_timeout_seconds=10"));
    assert!(service_serve.contains("p2p_max_concurrent_streams=128"));
    assert!(service_serve.contains("p2p_idle_timeout_seconds=60"));
    assert!(service_serve.contains("auth_enabled=true"));
    assert!(service_serve.contains("rpc_routes=enabled"));
    assert!(service_serve.contains("explorer_routes=enabled"));
    assert!(service_serve.contains("faucet_routes=enabled"));
    assert!(service_serve.contains("telemetry_routes=enabled"));
    assert!(service_serve.contains("node_store_required=true"));

    let service_status = execute_reference_cli_command(&ExpectedCommand::ServiceStatus {
        data_dir: "/var/lib/tensorvm".to_owned(),
    })
    .unwrap();
    assert!(service_status.contains("command=service_status"));
    assert!(service_status.contains("data_dir=/var/lib/tensorvm"));
    assert!(service_status.contains("status_source=node_store"));

    let service_block = execute_reference_cli_command(&ExpectedCommand::ServiceBlock {
        data_dir: "/var/lib/tensorvm".to_owned(),
        height: 3,
    })
    .unwrap();
    assert!(service_block.contains("command=service_block"));
    assert!(service_block.contains("data_dir=/var/lib/tensorvm"));
    assert!(service_block.contains("height=3"));
    assert!(service_block.contains("status_source=node_store"));

    let local_seed = execute_reference_cli_command(&ExpectedCommand::LocalTestnetSeed {
        data_dir: "/var/lib/tensorvm".to_owned(),
    })
    .unwrap();
    assert!(local_seed.contains("command=local_testnet_seed"));
    assert!(local_seed.contains("data_dir=/var/lib/tensorvm"));
    assert!(local_seed.contains("local_cpu_seed_ready=true"));

    let public_command = ExpectedCommand::PublicEvidenceValidate {
        manifest: "evidence.txt".to_owned(),
    };
    assert_eq!(
        execute_reference_cli_command(&public_command).unwrap(),
        describe_command(&public_command)
    );

    let publication = execute_reference_cli_command(&ExpectedCommand::PublicEvidencePublication {
        bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
        public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
        manifest_signer: address(b"public-evidence-publisher"),
        manifest_signature_count: 1,
        independent_auditor_count: 1,
    })
    .unwrap();
    assert!(publication.contains(&format!(
        "bundle_id={}",
        manifest_hash(b"public-evidence-bundle")
    )));
    assert!(publication.contains("public_uri=https://tensorvm.net/tensorvm/public-evidence.json"));
    assert!(publication.contains(&format!(
        "manifest_signer={}",
        manifest_address(b"public-evidence-publisher")
    )));
    assert!(publication.contains(&format!(
        "manifest_signature={}",
        manifest_publication_signature()
    )));
    assert!(publication.contains("manifest_signature_count=1"));
    assert!(publication.contains("independent_auditor_count=1"));

    let auditor_record =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceAuditorRecord {
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

    let run_window = execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRunWindow {
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
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRunWindowFromFile {
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
        let node = execute_reference_cli_command(&ExpectedCommand::PublicEvidenceNodeHeartbeat {
            role,
            address: address(address_label),
            operator_id: hash_bytes(b"test", &[operator_label]),
            first_seen_block: 0,
            last_seen_block: 9,
            signed_heartbeat_count: 10,
        })
        .unwrap();
        assert!(node.starts_with(&format!(
            "node={tag},{},{}",
            hex(&address(address_label)),
            hex(&hash_bytes(b"test", &[operator_label]))
        )));
        assert!(node.ends_with(&manifest_node_signature(
            role,
            address_label,
            operator_label
        )));
        let heartbeat_file = std::env::temp_dir().join(format!(
            "tensor-vm-node-heartbeat-{}-{}.records",
            std::process::id(),
            tag
        ));
        let heartbeat_records = (0..10)
            .map(|block| {
                format!(
                    "node_heartbeat_observation={tag},{},{},{}",
                    hex(&address(address_label)),
                    hex(&hash_bytes(b"test", &[operator_label])),
                    block
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&heartbeat_file, heartbeat_records).unwrap();
        let node_from_file =
            execute_reference_cli_command(&ExpectedCommand::PublicEvidenceNodeHeartbeatFromFile {
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
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceOperatorAttestation {
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

    let service_health =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealth {
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
    assert!(service_health.starts_with("service=rpc,"));
    assert!(service_health.contains("https://rpc.tensorvm.net/health,/health,0,9,10,10"));
    assert!(service_health.ends_with(&manifest_service_signature(
        PublicServiceKind::Rpc,
        b"rpc-service"
    )));
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
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealthFromFile {
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
        let line = execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceHealth {
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
        assert!(line.starts_with(&format!("service={tag},")));
        assert!(line.contains(public_service_url(kind)));
        assert!(line.ends_with(&manifest_service_signature(kind, label)));
    }

    let service_content =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: public_service_content_url(PublicServiceKind::Rpc).to_owned(),
            content_path: public_service_content_path(PublicServiceKind::Rpc).to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .unwrap();
    assert!(service_content.starts_with("service_content=rpc,"));
    assert!(service_content.contains("https://rpc.tensorvm.net/chain/head,/chain/head"));
    assert_eq!(
        service_content,
        manifest_service_content_line(PublicServiceKind::Rpc, b"rpc-service")
    );
    let observed_content = vec![7_u8; 80];
    let observed_content_root = public_service_content_root(&observed_content);
    let service_content_from_bytes =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContentFromBytes {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: public_service_content_url(PublicServiceKind::Rpc).to_owned(),
            content_path: public_service_content_path(PublicServiceKind::Rpc).to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            content_hex: hex(&observed_content),
        })
        .unwrap();
    assert!(service_content_from_bytes.starts_with("service_content=rpc,"));
    assert!(
        service_content_from_bytes
            .contains(&format!("{},1700000000,80,", hex(&observed_content_root)))
    );
    let content_file = std::env::temp_dir().join(format!(
        "tensor-vm-service-content-{}-{}.body",
        std::process::id(),
        observed_content_root[0]
    ));
    std::fs::write(&content_file, &observed_content).unwrap();
    let service_content_from_file =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceServiceContentFromFile {
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
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceNetworkObservation {
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
    let network_observation_from_file = execute_reference_cli_command(
        &ExpectedCommand::PublicEvidenceNetworkObservationFromServiceLog {
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
        execute_reference_cli_command(
            &ExpectedCommand::PublicEvidenceNetworkObservationFromServiceLog {
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

    let record_cases: [(PublicEvidenceRecordKind, &[u8], u64, &str, String); 6] = [
        (
            PublicEvidenceRecordKind::BlockHistory,
            b"block-history-root",
            10,
            "block_history",
            hex(&manifest_bundle().block_history_signature),
        ),
        (
            PublicEvidenceRecordKind::FinalityHistory,
            b"finality-history-root",
            10,
            "finality_history",
            hex(&manifest_bundle().finality_history_signature),
        ),
        (
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            b"network-runtime-root",
            3,
            "network_runtime_observation",
            hex(&manifest_bundle().network_runtime_observation_signature),
        ),
        (
            PublicEvidenceRecordKind::DataAvailabilityMeasurements,
            b"data-availability-root",
            20,
            "data_availability_measurement",
            hex(&manifest_bundle().data_availability_measurement_signature),
        ),
        (
            PublicEvidenceRecordKind::InvalidWorkRejections,
            b"invalid-work-root",
            1,
            "invalid_work_rejection",
            hex(&manifest_bundle().invalid_work_rejection_signature),
        ),
        (
            PublicEvidenceRecordKind::RewardSettlements,
            b"reward-settlement-root",
            1,
            "reward_settlement",
            hex(&manifest_bundle().reward_settlement_signature),
        ),
    ];
    for (kind, root_label, count, field_prefix, expected_signature) in record_cases {
        let record_root = if matches!(kind, PublicEvidenceRecordKind::NetworkRuntimeObservations) {
            manifest_bundle().network_runtime_observation_root
        } else {
            hash_bytes(b"test", &[root_label])
        };
        let root = hex(&record_root);
        let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
        let manifest_signer = address(b"public-evidence-publisher");
        let line = execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordSummary {
            kind,
            bundle_id,
            manifest_signer,
            record_root,
            record_count: count,
        })
        .unwrap();
        assert_eq!(
            line,
            format!(
                "{field_prefix}_records={count}\n{field_prefix}_root={root}\n{field_prefix}_signature={expected_signature}"
            )
        );

        let artifact_uri = format!(
            "https://evidence.tensorvm.net/{}/{}.json",
            manifest_hash(b"public-evidence-bundle"),
            public_evidence_record_kind_tag(kind)
        );
        let artifact_signature = crate::testnet::sign_public_evidence_artifact(
            &manifest_signer,
            &bundle_id,
            kind,
            &artifact_uri,
            &record_root,
            count,
        );
        let artifact_line =
            execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifact {
                kind,
                bundle_id,
                manifest_signer,
                artifact_uri: artifact_uri.clone(),
                record_root,
                record_count: count,
            })
            .unwrap();
        assert_eq!(
            artifact_line,
            format!(
                "record_artifact={},{artifact_uri},{root},{count},{}",
                public_evidence_record_kind_tag(kind),
                hex(&artifact_signature)
            )
        );
    }

    let roots = vec![
        hash_bytes(b"test", &[b"network-observation-a"]),
        hash_bytes(b"test", &[b"network-observation-b"]),
    ];
    let aggregate_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &roots,
    )
    .unwrap();
    let aggregate_signature = sign_public_evidence_record(
        &address(b"public-evidence-publisher"),
        &hash_bytes(b"test", &[b"public-evidence-bundle"]),
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &aggregate_root,
        roots.len() as u64,
    );
    let aggregate_line =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordSummaryFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_roots: roots.clone(),
        })
        .unwrap();
    assert_eq!(
        aggregate_line,
        format!(
            "network_runtime_observation_records=2\nnetwork_runtime_observation_root={}\nnetwork_runtime_observation_signature={}",
            hex(&aggregate_root),
            hex(&aggregate_signature)
        )
    );
    let aggregate_artifact_uri = "https://evidence.tensorvm.net/network-runtime.json";
    let aggregate_artifact_signature = crate::testnet::sign_public_evidence_artifact(
        &address(b"public-evidence-publisher"),
        &hash_bytes(b"test", &[b"public-evidence-bundle"]),
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        aggregate_artifact_uri,
        &aggregate_root,
        roots.len() as u64,
    );
    let aggregate_artifact_line =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifactFromRoots {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: aggregate_artifact_uri.to_owned(),
            record_roots: roots,
        })
        .unwrap();
    assert_eq!(
        aggregate_artifact_line,
        format!(
            "record_artifact=network-runtime,{aggregate_artifact_uri},{},2,{}",
            hex(&aggregate_root),
            hex(&aggregate_artifact_signature)
        )
    );

    let record_file_roots = vec![
        observation_root,
        hash_bytes(b"test", &[b"network-observation-b"]),
    ];
    let record_file_aggregate_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &record_file_roots,
    )
    .unwrap();
    let record_file = std::env::temp_dir().join(format!(
        "tensor-vm-network-records-{}-{}.records",
        std::process::id(),
        record_file_aggregate_root[0]
    ));
    std::fs::write(
        &record_file,
        format!(
            "# captured network-runtime records\n\n{network_observation}\nrecord_root={}\n",
            hex(&record_file_roots[1])
        ),
    )
    .unwrap();
    let record_file_path = record_file.to_string_lossy().into_owned();
    let record_file_roots_from_disk = public_evidence_record_roots_from_file(
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &record_file_path,
    )
    .unwrap();
    assert_eq!(record_file_roots_from_disk, record_file_roots);
    let record_file_summary =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordSummaryFromFile {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_file: record_file_path.clone(),
        })
        .unwrap();
    let record_file_signature = sign_public_evidence_record(
        &address(b"public-evidence-publisher"),
        &hash_bytes(b"test", &[b"public-evidence-bundle"]),
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &record_file_aggregate_root,
        record_file_roots.len() as u64,
    );
    assert_eq!(
        record_file_summary,
        format!(
            "network_runtime_observation_records=2\nnetwork_runtime_observation_root={}\nnetwork_runtime_observation_signature={}",
            hex(&record_file_aggregate_root),
            hex(&record_file_signature)
        )
    );
    let record_file_artifact =
        execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifactFromFile {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: aggregate_artifact_uri.to_owned(),
            record_file: record_file_path.clone(),
        })
        .unwrap();
    let record_file_artifact_signature = crate::testnet::sign_public_evidence_artifact(
        &address(b"public-evidence-publisher"),
        &hash_bytes(b"test", &[b"public-evidence-bundle"]),
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        aggregate_artifact_uri,
        &record_file_aggregate_root,
        record_file_roots.len() as u64,
    );
    assert_eq!(
        record_file_artifact,
        format!(
            "record_artifact=network-runtime,{aggregate_artifact_uri},{},2,{}",
            hex(&record_file_aggregate_root),
            hex(&record_file_artifact_signature)
        )
    );
    std::fs::remove_file(&record_file).unwrap();
    assert_eq!(
        supporting_record_line_prefix(PublicEvidenceRecordKind::NetworkRuntimeObservations),
        None
    );

    let supporting_record_cases = [
        (
            PublicEvidenceRecordKind::BlockHistory,
            "block_history_record=0,aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "block_history",
        ),
        (
            PublicEvidenceRecordKind::FinalityHistory,
            "finality_history_record=0,aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,finalized",
            "finality_history",
        ),
        (
            PublicEvidenceRecordKind::DataAvailabilityMeasurements,
            "data_availability_measurement=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,available,0",
            "data_availability_measurement",
        ),
        (
            PublicEvidenceRecordKind::InvalidWorkRejections,
            "invalid_work_rejection=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,rejected,0",
            "invalid_work_rejection",
        ),
        (
            PublicEvidenceRecordKind::RewardSettlements,
            concat!(
                "reward_settlement=",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,",
                "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc,0"
            ),
            "reward_settlement",
        ),
    ];
    for (kind, raw_line, field_prefix) in supporting_record_cases {
        let raw_root = supporting_record_root_from_line(
            kind,
            raw_line,
            supporting_record_line_prefix(kind).unwrap(),
        )
        .unwrap();
        assert_eq!(
            public_evidence_record_root_from_line(kind, raw_line).unwrap(),
            raw_root
        );
        let extra_root = hash_bytes(b"test", &[public_evidence_record_kind_tag(kind).as_bytes()]);
        let roots = vec![raw_root, extra_root];
        let aggregate_root = aggregate_public_evidence_record_roots(kind, &roots).unwrap();
        let raw_record_file = std::env::temp_dir().join(format!(
            "tensor-vm-{}-records-{}-{}.records",
            public_evidence_record_kind_tag(kind),
            std::process::id(),
            aggregate_root[0]
        ));
        std::fs::write(
            &raw_record_file,
            format!(
                "# raw supporting records\n{raw_line}\nrecord_root={}\n",
                hex(&extra_root)
            ),
        )
        .unwrap();
        let raw_record_file_path = raw_record_file.to_string_lossy().into_owned();
        assert_eq!(
            public_evidence_record_roots_from_file(kind, &raw_record_file_path).unwrap(),
            roots
        );
        let summary =
            execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordSummaryFromFile {
                kind,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_file: raw_record_file_path.clone(),
            })
            .unwrap();
        let signature = sign_public_evidence_record(
            &address(b"public-evidence-publisher"),
            &hash_bytes(b"test", &[b"public-evidence-bundle"]),
            kind,
            &aggregate_root,
            roots.len() as u64,
        );
        assert_eq!(
            summary,
            format!(
                "{field_prefix}_records=2\n{field_prefix}_root={}\n{field_prefix}_signature={}",
                hex(&aggregate_root),
                hex(&signature)
            )
        );
        let artifact_uri = format!(
            "https://evidence.tensorvm.net/{}.json",
            public_evidence_record_kind_tag(kind)
        );
        let artifact =
            execute_reference_cli_command(&ExpectedCommand::PublicEvidenceRecordArtifactFromFile {
                kind,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: artifact_uri.clone(),
                record_file: raw_record_file_path,
            })
            .unwrap();
        let artifact_signature = crate::testnet::sign_public_evidence_artifact(
            &address(b"public-evidence-publisher"),
            &hash_bytes(b"test", &[b"public-evidence-bundle"]),
            kind,
            &artifact_uri,
            &aggregate_root,
            roots.len() as u64,
        );
        assert_eq!(
            artifact,
            format!(
                "record_artifact={},{},{},2,{}",
                public_evidence_record_kind_tag(kind),
                artifact_uri,
                hex(&aggregate_root),
                hex(&artifact_signature)
            )
        );
        std::fs::remove_file(&raw_record_file).unwrap();
    }
    let malformed_supporting_record_cases = [
        (
            PublicEvidenceRecordKind::BlockHistory,
            "block_history_record=0",
        ),
        (
            PublicEvidenceRecordKind::BlockHistory,
            "block_history_record=0,not-a-root",
        ),
        (
            PublicEvidenceRecordKind::FinalityHistory,
            "finality_history_record=0,aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,pending",
        ),
        (
            PublicEvidenceRecordKind::DataAvailabilityMeasurements,
            "data_availability_measurement=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,missing,0",
        ),
        (
            PublicEvidenceRecordKind::InvalidWorkRejections,
            "invalid_work_rejection=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,accepted,0",
        ),
        (
            PublicEvidenceRecordKind::RewardSettlements,
            "reward_settlement=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,miner,,0",
        ),
        (
            PublicEvidenceRecordKind::RewardSettlements,
            concat!(
                "reward_settlement=",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,",
                "miner,",
                "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc,0"
            ),
        ),
        (
            PublicEvidenceRecordKind::RewardSettlements,
            concat!(
                "reward_settlement=",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,",
                "validator,0"
            ),
        ),
    ];
    for (kind, raw_line) in malformed_supporting_record_cases {
        assert!(matches!(
            public_evidence_record_root_from_line(kind, raw_line),
            Err(TvmError::InvalidReceipt(_))
        ));
    }
    assert!(matches!(
        validate_supporting_record_payload(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        ),
        Err(TvmError::InvalidReceipt(_))
    ));
    assert_eq!(
        public_evidence_record_roots_from_file(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &record_file_path,
        )
        .unwrap_err()
        .to_string(),
        "storage error: failed to read public evidence record file"
    );
    let empty_record_file = std::env::temp_dir().join(format!(
        "tensor-vm-empty-records-{}-{}.records",
        std::process::id(),
        record_file_aggregate_root[1]
    ));
    std::fs::write(&empty_record_file, "# no roots yet\n\n").unwrap();
    assert_eq!(
        public_evidence_record_roots_from_file(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &empty_record_file.to_string_lossy(),
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: record file has no roots"
    );
    std::fs::remove_file(&empty_record_file).unwrap();
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::FinalityHistory,
            "network_runtime_observation=bad",
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: unsupported public evidence record line"
    );
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::BlockHistory,
            &format!(
                "record_root= {}",
                hex(&hash_bytes(b"test", &[b"bad-whitespace"]))
            ),
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid record root file line"
    );
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            "network_runtime_observation=bad",
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid network observation record line"
    );
    assert_eq!(
        public_evidence_record_root_from_line(
            PublicEvidenceRecordKind::BlockHistory,
            "block_history_record= ",
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: invalid public evidence supporting record line"
    );
    let whitespace_record_file = std::env::temp_dir().join(format!(
        "tensor-vm-whitespace-record-{}.records",
        std::process::id()
    ));
    std::fs::write(&whitespace_record_file, " block_history_record=0\n").unwrap();
    let whitespace_record_path = whitespace_record_file.to_string_lossy().into_owned();
    assert_eq!(
        public_evidence_record_roots_from_file(
            PublicEvidenceRecordKind::BlockHistory,
            &whitespace_record_path,
        )
        .unwrap_err()
        .to_string(),
        "invalid receipt: public evidence record line has leading or trailing whitespace"
    );
    std::fs::remove_file(&whitespace_record_file).unwrap();
}
