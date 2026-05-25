use super::*;
use crate::hash::hex;
use crate::testnet::{
    PublicEvidenceAuditorRecord, PublicEvidencePublication, PublicEvidenceRecordKind,
};
use crate::types::{Hash, address, hash_bytes};

fn manifest_bundle_hash() -> String {
    hex(&hash_bytes(b"test", &[b"public-evidence-bundle"]))
}

pub(super) fn manifest_publication_signature() -> String {
    let publication = manifest_publication();
    hex(&publication.manifest_signature)
}

pub(super) fn manifest_publication() -> PublicEvidencePublication {
    PublicEvidencePublication::new(
        hash_bytes(b"test", &[b"public-evidence-bundle"]),
        String::from("https://tensorvm.net/tensorvm/public-evidence.json"),
        address(b"public-evidence-publisher"),
        1,
        1,
    )
}

pub(super) fn manifest_auditor_uri() -> String {
    format!("https://auditors.tensorvm.net/{}/0", manifest_bundle_hash())
}

pub(super) fn manifest_auditor_signature() -> String {
    let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
    let record = PublicEvidenceAuditorRecord::new(
        &bundle_id,
        "https://tensorvm.net/tensorvm/public-evidence.json",
        address(b"public-evidence-auditor-0"),
        manifest_auditor_uri(),
        1_700_000_060,
    );
    hex(&record.auditor_signature)
}

pub(super) fn manifest_artifact_line(
    kind: PublicEvidenceRecordKind,
    root_label: &[u8],
    record_count: u64,
) -> String {
    manifest_artifact_line_for_root(kind, hash_bytes(b"test", &[root_label]), record_count)
}

pub(super) fn manifest_artifact_line_for_root(
    kind: PublicEvidenceRecordKind,
    record_root: Hash,
    record_count: u64,
) -> String {
    let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
    let artifact_uri = format!(
        "https://evidence.tensorvm.net/{}/{}.json",
        manifest_bundle_hash(),
        public_evidence_record_kind_tag(kind)
    );
    let signature = crate::testnet::sign_public_evidence_artifact(
        &address(b"public-evidence-publisher"),
        &bundle_id,
        kind,
        &artifact_uri,
        &record_root,
        record_count,
    );
    format!(
        "record_artifact={},{},{},{},{}",
        public_evidence_record_kind_tag(kind),
        artifact_uri,
        hex(&record_root),
        record_count,
        hex(&signature)
    )
}
