use super::parser_support::{hash_arg, path, record_artifact_locator_args, record_root_args};
use super::{
    EvidenceCommand, EvidenceRecordCommand, PublicCommand, RecordArtifactArgs,
    RecordArtifactFromFileArgs, RecordArtifactFromRootsArgs, RecordSummaryArgs,
    RecordSummaryFromFileArgs, RecordSummaryFromRootsArgs, TvmdCommand, manifest_address,
    manifest_hash, parse_test_cli, record_context_args,
};
use crate::testnet::PublicEvidenceRecordKind;
use crate::types::hash_bytes;

#[test]
fn parses_record_evidence_commands() {
    let bundle_id = manifest_hash(b"public-evidence-bundle");
    let manifest_signer = manifest_address(b"public-evidence-publisher");
    let record_root = manifest_hash(b"network-runtime-root");

    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "summary",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--record-root",
            &record_root,
            "--record-count",
            "4",
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Record(
            EvidenceRecordCommand::Summary(RecordSummaryArgs {
                context: record_context_args(PublicEvidenceRecordKind::NetworkRuntimeObservations),
                root: record_root_args(hash_bytes(b"test", &[b"network-runtime-root"]), 4),
            }),
        )))
    );

    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "artifact",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--artifact-uri",
            "https://evidence.tensorvm.net/network-runtime.json",
            "--record-root",
            &record_root,
            "--record-count",
            "4",
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Record(
            EvidenceRecordCommand::Artifact(RecordArtifactArgs {
                context: record_context_args(PublicEvidenceRecordKind::NetworkRuntimeObservations),
                artifact: record_artifact_locator_args(
                    "https://evidence.tensorvm.net/network-runtime.json",
                ),
                root: record_root_args(hash_bytes(b"test", &[b"network-runtime-root"]), 4),
            }),
        )))
    );

    let record_roots = format!(
        "{},{}",
        manifest_hash(b"network-observation-a"),
        manifest_hash(b"network-observation-b")
    );
    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "summary-roots",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--record-roots",
            &record_roots,
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Record(
            EvidenceRecordCommand::SummaryRoots(RecordSummaryFromRootsArgs {
                context: record_context_args(PublicEvidenceRecordKind::NetworkRuntimeObservations),
                record_roots: vec![
                    hash_arg(hash_bytes(b"test", &[b"network-observation-a"])),
                    hash_arg(hash_bytes(b"test", &[b"network-observation-b"])),
                ],
            }),
        )))
    );

    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "artifact-roots",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--artifact-uri",
            "https://evidence.tensorvm.net/network-runtime.json",
            "--record-roots",
            &record_roots,
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Record(
            EvidenceRecordCommand::ArtifactRoots(RecordArtifactFromRootsArgs {
                context: record_context_args(PublicEvidenceRecordKind::NetworkRuntimeObservations),
                artifact: record_artifact_locator_args(
                    "https://evidence.tensorvm.net/network-runtime.json",
                ),
                record_roots: vec![
                    hash_arg(hash_bytes(b"test", &[b"network-observation-a"])),
                    hash_arg(hash_bytes(b"test", &[b"network-observation-b"])),
                ],
            }),
        )))
    );

    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "summary-file",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--record-file",
            "artifacts/network-runtime.records",
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Record(
            EvidenceRecordCommand::SummaryFile(RecordSummaryFromFileArgs {
                context: record_context_args(PublicEvidenceRecordKind::NetworkRuntimeObservations),
                record_file: path("artifacts/network-runtime.records"),
            }),
        )))
    );

    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "artifact-file",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--artifact-uri",
            "https://evidence.tensorvm.net/network-runtime.json",
            "--record-file",
            "artifacts/network-runtime.records",
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Record(
            EvidenceRecordCommand::ArtifactFile(RecordArtifactFromFileArgs {
                context: record_context_args(PublicEvidenceRecordKind::NetworkRuntimeObservations),
                artifact: record_artifact_locator_args(
                    "https://evidence.tensorvm.net/network-runtime.json",
                ),
                record_file: path("artifacts/network-runtime.records"),
            }),
        )))
    );
}
