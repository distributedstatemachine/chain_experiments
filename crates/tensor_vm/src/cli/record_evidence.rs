use super::evidence_fields::{
    public_evidence_record_field_prefix, public_evidence_record_kind_tag,
};
use crate::app::KeyValueReportWriter;
use crate::error::{Result, TvmError};
use crate::hash::hex;
use crate::testnet::{
    PublicEvidenceRecordKind, PublicEvidenceSupportingArtifact, sign_public_evidence_record,
};
use crate::types::{Address, Hash};

pub(super) fn record_summary_evidence_lines(
    kind: PublicEvidenceRecordKind,
    bundle_id: Hash,
    manifest_signer: Address,
    record_root: Hash,
    record_count: u64,
) -> Result<String> {
    if bundle_id == [0; 32] {
        return Err(TvmError::InvalidReceipt("bundle id argument is empty"));
    }
    if manifest_signer == [0; 32] {
        return Err(TvmError::InvalidReceipt(
            "manifest signer argument is empty",
        ));
    }
    if record_root == [0; 32] {
        return Err(TvmError::InvalidReceipt("record root argument is empty"));
    }
    if record_count == 0 {
        return Err(TvmError::InvalidReceipt("record count argument is empty"));
    }
    let field_prefix = public_evidence_record_field_prefix(kind);
    let signature = sign_public_evidence_record(
        &manifest_signer,
        &bundle_id,
        kind,
        &record_root,
        record_count,
    );
    let mut report = KeyValueReportWriter::new();
    report.field(&format!("{field_prefix}_records"), record_count);
    report.field(&format!("{field_prefix}_root"), hex(&record_root));
    report.field(&format!("{field_prefix}_signature"), hex(&signature));
    Ok(report.finish())
}

pub(super) fn record_artifact_evidence_line(
    kind: PublicEvidenceRecordKind,
    bundle_id: Hash,
    manifest_signer: Address,
    artifact_uri: &str,
    record_root: Hash,
    record_count: u64,
) -> Result<String> {
    if bundle_id == [0; 32] {
        return Err(TvmError::InvalidReceipt("bundle id argument is empty"));
    }
    if manifest_signer == [0; 32] {
        return Err(TvmError::InvalidReceipt(
            "manifest signer argument is empty",
        ));
    }
    if record_root == [0; 32] {
        return Err(TvmError::InvalidReceipt("record root argument is empty"));
    }
    if record_count == 0 {
        return Err(TvmError::InvalidReceipt("record count argument is empty"));
    }
    let artifact = PublicEvidenceSupportingArtifact::new(
        &bundle_id,
        &manifest_signer,
        kind,
        artifact_uri.to_owned(),
        record_root,
        record_count,
    );
    if !artifact.is_public_and_signed(&bundle_id, &manifest_signer) {
        return Err(TvmError::InvalidReceipt("invalid public evidence artifact"));
    }
    let mut report = KeyValueReportWriter::new();
    report.field(
        "record_artifact",
        format!(
            "{},{},{},{},{}",
            public_evidence_record_kind_tag(kind),
            artifact.artifact_uri,
            hex(&artifact.record_root),
            artifact.record_count,
            hex(&artifact.artifact_signature)
        ),
    );
    Ok(report.finish())
}
