use crate::app::KeyValueReportWriter;
use crate::error::{Result, TvmError};
use crate::hash::hex;
use crate::testnet::{PublicEvidenceAuditorRecord, PublicEvidencePublication};
use crate::types::{Address, Hash};

pub(super) fn publication_evidence_lines(
    bundle_id: Hash,
    public_uri: &str,
    manifest_signer: Address,
    manifest_signature_count: u64,
    independent_auditor_count: u64,
) -> Result<String> {
    let publication = PublicEvidencePublication::new(
        bundle_id,
        public_uri.to_owned(),
        manifest_signer,
        manifest_signature_count,
        independent_auditor_count,
    );
    if !publication.is_published_and_independently_checkable() {
        return Err(TvmError::InvalidReceipt(
            "invalid public evidence publication",
        ));
    }
    let mut report = KeyValueReportWriter::new();
    report.field("bundle_id", hex(&publication.bundle_id));
    report.field("public_uri", publication.public_uri);
    report.field("manifest_signer", hex(&publication.manifest_signer));
    report.field("manifest_signature", hex(&publication.manifest_signature));
    report.field(
        "manifest_signature_count",
        publication.manifest_signature_count,
    );
    report.field(
        "independent_auditor_count",
        publication.independent_auditor_count,
    );
    Ok(report.finish())
}

pub(super) fn auditor_record_evidence_line(
    bundle_id: Hash,
    public_uri: &str,
    auditor_id: Address,
    audit_uri: &str,
    observed_at_unix_seconds: u64,
) -> Result<String> {
    let auditor = PublicEvidenceAuditorRecord::new(
        &bundle_id,
        public_uri,
        auditor_id,
        audit_uri.to_owned(),
        observed_at_unix_seconds,
    );
    if !auditor.has_external_auditor_proof(&bundle_id, public_uri) {
        return Err(TvmError::InvalidReceipt(
            "invalid public evidence auditor record",
        ));
    }
    let mut report = KeyValueReportWriter::new();
    report.field(
        "auditor",
        format!(
            "{},{},{},{}",
            hex(&auditor.auditor_id),
            auditor.audit_uri,
            auditor.observed_at_unix_seconds,
            hex(&auditor.auditor_signature)
        ),
    );
    Ok(report.finish())
}
