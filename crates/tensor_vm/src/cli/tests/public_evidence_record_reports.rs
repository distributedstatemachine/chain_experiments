use super::*;

#[test]
fn execute_public_evidence_record_reports_outputs() {
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
