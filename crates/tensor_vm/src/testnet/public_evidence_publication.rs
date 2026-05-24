use super::public_evidence_crypto::{
    PublicEvidenceRecordKind, public_evidence_artifact_message, public_evidence_auditor_message,
    public_evidence_manifest_message,
};
use super::public_urls::public_evidence_uri_is_external;
use crate::types::{Address, Hash, Signature, sign, verify_signature};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicEvidencePublication {
    pub bundle_id: Hash,
    pub public_uri: String,
    pub manifest_signer: Address,
    pub manifest_signature: Signature,
    pub manifest_signature_count: u64,
    pub independent_auditor_count: u64,
}

impl PublicEvidencePublication {
    pub fn new(
        bundle_id: Hash,
        public_uri: String,
        manifest_signer: Address,
        manifest_signature_count: u64,
        independent_auditor_count: u64,
    ) -> Self {
        let message = public_evidence_manifest_message(
            &bundle_id,
            &public_uri,
            manifest_signature_count,
            independent_auditor_count,
        );
        Self {
            bundle_id,
            public_uri,
            manifest_signer,
            manifest_signature: sign(&manifest_signer, &message),
            manifest_signature_count,
            independent_auditor_count,
        }
    }

    pub fn manifest_signature_valid(&self) -> bool {
        verify_signature(
            &self.manifest_signer,
            &public_evidence_manifest_message(
                &self.bundle_id,
                &self.public_uri,
                self.manifest_signature_count,
                self.independent_auditor_count,
            ),
            &self.manifest_signature,
        )
    }

    pub fn is_published_and_independently_checkable(&self) -> bool {
        self.bundle_id != [0; 32]
            && public_evidence_uri_is_external(&self.public_uri)
            && self.manifest_signer != [0; 32]
            && self.manifest_signature_count == 1
            && self.manifest_signature_valid()
            && self.independent_auditor_count > 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicEvidenceAuditorRecord {
    pub auditor_id: Address,
    pub audit_uri: String,
    pub observed_at_unix_seconds: u64,
    pub auditor_signature: Signature,
}

impl PublicEvidenceAuditorRecord {
    pub fn new(
        bundle_id: &Hash,
        public_uri: &str,
        auditor_id: Address,
        audit_uri: impl Into<String>,
        observed_at_unix_seconds: u64,
    ) -> Self {
        let audit_uri = audit_uri.into();
        let message = public_evidence_auditor_message(
            bundle_id,
            public_uri,
            &auditor_id,
            &audit_uri,
            observed_at_unix_seconds,
        );
        Self {
            auditor_id,
            audit_uri,
            observed_at_unix_seconds,
            auditor_signature: sign(&auditor_id, &message),
        }
    }

    pub fn auditor_signature_valid(&self, bundle_id: &Hash, public_uri: &str) -> bool {
        verify_signature(
            &self.auditor_id,
            &public_evidence_auditor_message(
                bundle_id,
                public_uri,
                &self.auditor_id,
                &self.audit_uri,
                self.observed_at_unix_seconds,
            ),
            &self.auditor_signature,
        )
    }

    pub fn has_external_auditor_proof(&self, bundle_id: &Hash, public_uri: &str) -> bool {
        self.auditor_id != [0; 32]
            && *bundle_id != [0; 32]
            && self.observed_at_unix_seconds > 0
            && public_evidence_uri_is_external(public_uri)
            && public_evidence_uri_is_external(&self.audit_uri)
            && self.auditor_signature_valid(bundle_id, public_uri)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicEvidenceSupportingArtifact {
    pub kind: PublicEvidenceRecordKind,
    pub artifact_uri: String,
    pub record_root: Hash,
    pub record_count: u64,
    pub artifact_signature: Signature,
}

impl PublicEvidenceSupportingArtifact {
    pub fn new(
        bundle_id: &Hash,
        manifest_signer: &Address,
        kind: PublicEvidenceRecordKind,
        artifact_uri: impl Into<String>,
        record_root: Hash,
        record_count: u64,
    ) -> Self {
        let artifact_uri = artifact_uri.into();
        let message = public_evidence_artifact_message(
            bundle_id,
            kind,
            &artifact_uri,
            &record_root,
            record_count,
        );
        Self {
            kind,
            artifact_uri,
            record_root,
            record_count,
            artifact_signature: sign(manifest_signer, &message),
        }
    }

    pub fn artifact_signature_valid(&self, bundle_id: &Hash, manifest_signer: &Address) -> bool {
        verify_signature(
            manifest_signer,
            &public_evidence_artifact_message(
                bundle_id,
                self.kind,
                &self.artifact_uri,
                &self.record_root,
                self.record_count,
            ),
            &self.artifact_signature,
        )
    }

    pub fn is_public_and_signed(&self, bundle_id: &Hash, manifest_signer: &Address) -> bool {
        self.record_root != [0; 32]
            && self.record_count > 0
            && public_evidence_uri_is_external(&self.artifact_uri)
            && self.artifact_signature_valid(bundle_id, manifest_signer)
    }
}
