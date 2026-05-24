use super::*;

#[test]
fn execute_public_record_evidence_rejects_invalid_args() {
    let bundle_id = manifest_hash(b"public-evidence-bundle");
    let manifest_signer = manifest_address(b"public-evidence-publisher");
    let record_root = manifest_hash(b"network-runtime-root");
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "summary",
            "--kind",
            "operator-identity",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--record-root",
            &record_root,
            "--record-count",
            "4",
        ])
        .is_err()
    );
    let invalid_hash = "g".repeat(64);
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "summary",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &invalid_hash,
            "--manifest-signer",
            &manifest_signer,
            "--record-root",
            &record_root,
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
        RecordSummaryArgs {
            kind: record_kind_arg(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            bundle_id: hash_arg(bundle_id),
            manifest_signer: address_arg(manifest_signer),
            record_root: hash_arg(record_root),
            record_count,
        },
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
        RecordArtifactArgs {
            kind: record_kind_arg(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            bundle_id: hash_arg(bundle_id),
            manifest_signer: address_arg(manifest_signer),
            artifact_uri: artifact_uri.to_owned(),
            record_root: hash_arg(record_root),
            record_count,
        },
    )))
}

fn execute_record_summary_roots(record_roots: Vec<[u8; 32]>) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Record(
        EvidenceRecordCommand::SummaryRoots(RecordSummaryFromRootsArgs {
            kind: record_kind_arg(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
            manifest_signer: address_arg(address(b"public-evidence-publisher")),
            record_roots: hash_args(record_roots),
        }),
    ))
}

fn execute_record_artifact_roots(record_roots: Vec<[u8; 32]>) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Record(
        EvidenceRecordCommand::ArtifactRoots(RecordArtifactFromRootsArgs {
            kind: record_kind_arg(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
            manifest_signer: address_arg(address(b"public-evidence-publisher")),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_roots: hash_args(record_roots),
        }),
    ))
}
