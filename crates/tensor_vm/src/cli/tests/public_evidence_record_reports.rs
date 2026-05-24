use super::*;

#[test]
fn execute_public_evidence_record_reports_outputs() {
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
    let aggregate_line = execute_public_evidence_command(&EvidenceCommand::Record(
        EvidenceRecordCommand::SummaryRoots(RecordSummaryFromRootsArgs {
            kind: record_kind_arg(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
            manifest_signer: address_arg(address(b"public-evidence-publisher")),
            record_roots: hash_args(roots.clone()),
        }),
    ))
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
    let aggregate_artifact_line = execute_public_evidence_command(&EvidenceCommand::Record(
        EvidenceRecordCommand::ArtifactRoots(RecordArtifactFromRootsArgs {
            kind: record_kind_arg(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
            manifest_signer: address_arg(address(b"public-evidence-publisher")),
            artifact_uri: aggregate_artifact_uri.to_owned(),
            record_roots: hash_args(roots),
        }),
    ))
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
    let record_file_summary = execute_public_evidence_command(&EvidenceCommand::Record(
        EvidenceRecordCommand::SummaryFile(RecordSummaryFromFileArgs {
            kind: record_kind_arg(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
            manifest_signer: address_arg(address(b"public-evidence-publisher")),
            record_file: record_file.clone(),
        }),
    ))
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
    let record_file_artifact = execute_public_evidence_command(&EvidenceCommand::Record(
        EvidenceRecordCommand::ArtifactFile(RecordArtifactFromFileArgs {
            kind: record_kind_arg(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
            manifest_signer: address_arg(address(b"public-evidence-publisher")),
            artifact_uri: aggregate_artifact_uri.to_owned(),
            record_file: record_file.clone(),
        }),
    ))
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
        let summary = execute_public_evidence_command(&EvidenceCommand::Record(
            EvidenceRecordCommand::SummaryFile(RecordSummaryFromFileArgs {
                kind: record_kind_arg(kind),
                bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
                manifest_signer: address_arg(address(b"public-evidence-publisher")),
                record_file: raw_record_file.clone(),
            }),
        ))
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
        let artifact = execute_public_evidence_command(&EvidenceCommand::Record(
            EvidenceRecordCommand::ArtifactFile(RecordArtifactFromFileArgs {
                kind: record_kind_arg(kind),
                bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
                manifest_signer: address_arg(address(b"public-evidence-publisher")),
                artifact_uri: artifact_uri.clone(),
                record_file: raw_record_file.clone(),
            }),
        ))
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
