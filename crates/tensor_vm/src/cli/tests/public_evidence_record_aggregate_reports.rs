use super::*;

#[test]
fn execute_public_evidence_record_aggregate_reports_outputs() {
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
        EvidenceRecordCommand::SummaryRoots(RecordSummaryFromRootsArgs::new(
            record_context_args(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            record_roots_args(roots.clone()),
        )),
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
        EvidenceRecordCommand::ArtifactRoots(RecordArtifactFromRootsArgs::new(
            record_context_args(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            record_artifact_locator_args(aggregate_artifact_uri),
            record_roots_args(roots),
        )),
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
}
