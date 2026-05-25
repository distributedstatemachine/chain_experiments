use super::*;

#[test]
fn direct_public_record_evidence_rejects_invalid_args() {
    let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
    let manifest_signer = address(b"public-evidence-publisher");
    let record_root = hash_bytes(b"test", &[b"network-runtime-root"]);
    let artifact_uri = "https://evidence.tensorvm.net/network-runtime.json";
    assert!(execute_record_summary(bundle_id, manifest_signer, record_root, 4).is_ok());
    assert!(
        execute_record_artifact(bundle_id, manifest_signer, artifact_uri, record_root, 4).is_ok()
    );
    assert!(execute_record_summary([0; 32], manifest_signer, record_root, 4).is_err());
    assert!(execute_record_summary(bundle_id, [0; 32], record_root, 4).is_err());
    assert!(execute_record_summary(bundle_id, manifest_signer, [0; 32], 4).is_err());
    assert!(execute_record_summary(bundle_id, manifest_signer, record_root, 0).is_err());
    assert!(
        execute_record_artifact([0; 32], manifest_signer, artifact_uri, record_root, 4).is_err()
    );
    assert!(execute_record_artifact(bundle_id, [0; 32], artifact_uri, record_root, 4).is_err());
    assert!(
        execute_record_artifact(
            bundle_id,
            manifest_signer,
            "https://localhost/network-runtime.json",
            record_root,
            4,
        )
        .is_err()
    );
    assert!(
        execute_record_artifact(
            bundle_id,
            manifest_signer,
            "https://evidence.tensorvm.net/",
            record_root,
            4,
        )
        .is_err()
    );
    assert!(execute_record_artifact(bundle_id, manifest_signer, artifact_uri, [0; 32], 4).is_err());
    assert!(
        execute_record_artifact(bundle_id, manifest_signer, artifact_uri, record_root, 0).is_err()
    );
    assert!(execute_record_summary_roots(Vec::new()).is_err());
    assert!(execute_record_summary_roots(vec![[0; 32]]).is_err());
    let duplicate_record_root = hash_bytes(b"test", &[b"network-runtime-root"]);
    assert!(
        execute_record_summary_roots(vec![duplicate_record_root, duplicate_record_root]).is_err()
    );
    assert!(execute_record_artifact_roots(Vec::new()).is_err());
    assert!(
        execute_record_artifact_roots(vec![duplicate_record_root, duplicate_record_root]).is_err()
    );
}

fn execute_record_summary(
    bundle_id: [u8; 32],
    manifest_signer: [u8; 32],
    record_root: [u8; 32],
    record_count: u64,
) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Record(EvidenceRecordCommand::Summary(
        RecordSummaryArgs::new(
            record_context_args_from(
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id,
                manifest_signer,
            ),
            record_root_args(record_root, record_count),
        ),
    )))
}

fn execute_record_artifact(
    bundle_id: [u8; 32],
    manifest_signer: [u8; 32],
    artifact_uri: &str,
    record_root: [u8; 32],
    record_count: u64,
) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Record(EvidenceRecordCommand::Artifact(
        RecordArtifactArgs::new(
            record_context_args_from(
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id,
                manifest_signer,
            ),
            record_artifact_locator_args(artifact_uri),
            record_root_args(record_root, record_count),
        ),
    )))
}

fn execute_record_summary_roots(record_roots: Vec<[u8; 32]>) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Record(
        EvidenceRecordCommand::SummaryRoots(RecordSummaryFromRootsArgs::new(
            record_context_args(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            record_roots_args(record_roots),
        )),
    ))
}

fn execute_record_artifact_roots(record_roots: Vec<[u8; 32]>) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Record(
        EvidenceRecordCommand::ArtifactRoots(RecordArtifactFromRootsArgs::new(
            record_context_args(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            record_artifact_locator_args("https://evidence.tensorvm.net/network-runtime.json"),
            record_roots_args(record_roots),
        )),
    ))
}
