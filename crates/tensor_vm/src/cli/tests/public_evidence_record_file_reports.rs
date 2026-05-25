use super::*;

#[test]
fn execute_public_evidence_record_file_reports_outputs() {
    let peer_id = PeerId::random().to_string();
    let network_observation = execute_public_evidence_command(&EvidenceCommand::Network(
        EvidenceNetworkCommand::Observation(NetworkObservationArgs::new(
            network_observation_target_args(
                hash_bytes(b"test", &[b"network-operator"]),
                "/dns/node-a.tensorvm.net/tcp/4001",
                1_700_000_000,
            ),
            peer_id.parse().expect("test peer ID must parse"),
            network_observation_protocol_counts_args(5, 4, 2),
            network_observation_transport_limits_args(1_048_576, 10, 128, 60),
        )),
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
            context: record_context_args(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            file: record_file_args(record_file.clone()),
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
    let aggregate_artifact_uri = "https://evidence.tensorvm.net/network-runtime.json";
    let record_file_artifact = execute_public_evidence_command(&EvidenceCommand::Record(
        EvidenceRecordCommand::ArtifactFile(RecordArtifactFromFileArgs {
            context: record_context_args(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            artifact: record_artifact_locator_args(aggregate_artifact_uri),
            file: record_file_args(record_file.clone()),
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
}
