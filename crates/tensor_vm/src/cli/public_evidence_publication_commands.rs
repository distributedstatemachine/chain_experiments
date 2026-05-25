use super::public_evidence_bundle_commands::EvidenceBundleIdArgs;
use super::public_evidence_observation_commands::ObservationTimestampArgs;
use super::public_evidence_signing_commands::ManifestSignerArgs;
use super::value_types::AddressArg;
use crate::types::{Address, Hash};
use clap::{Args, ValueHint};

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct PublicationArgs {
    #[command(flatten)]
    bundle: PublicationBundleArgs,
    #[command(flatten)]
    signer: ManifestSignerArgs,
    #[arg(
        long,
        value_name = "N",
        help = "Number of manifest signatures included."
    )]
    manifest_signature_count: u64,
    #[arg(
        long,
        value_name = "N",
        help = "Number of independent auditor records included."
    )]
    independent_auditor_count: u64,
}

impl PublicationArgs {
    #[cfg(test)]
    pub(crate) fn new(
        bundle: PublicationBundleArgs,
        signer: ManifestSignerArgs,
        manifest_signature_count: u64,
        independent_auditor_count: u64,
    ) -> Self {
        Self {
            bundle,
            signer,
            manifest_signature_count,
            independent_auditor_count,
        }
    }

    pub fn bundle_id(&self) -> Hash {
        self.bundle.bundle_id()
    }

    pub fn public_uri(&self) -> &str {
        self.bundle.public_uri()
    }

    pub fn manifest_signer(&self) -> Address {
        self.signer.signer()
    }

    pub fn manifest_signature_count(&self) -> u64 {
        self.manifest_signature_count
    }

    pub fn independent_auditor_count(&self) -> u64 {
        self.independent_auditor_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct AuditorRecordArgs {
    #[command(flatten)]
    bundle: PublicationBundleArgs,
    #[arg(
        long,
        value_name = "HEX",
        help = "Address or identifier of the independent auditor."
    )]
    auditor_id: AddressArg,
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public URI for the auditor's review artifact."
    )]
    audit_uri: String,
    #[command(flatten)]
    observation: ObservationTimestampArgs,
}

impl AuditorRecordArgs {
    #[cfg(test)]
    pub(crate) fn new(
        bundle: PublicationBundleArgs,
        auditor_id: AddressArg,
        audit_uri: impl Into<String>,
        observation: ObservationTimestampArgs,
    ) -> Self {
        Self {
            bundle,
            auditor_id,
            audit_uri: audit_uri.into(),
            observation,
        }
    }

    pub fn bundle_id(&self) -> Hash {
        self.bundle.bundle_id()
    }

    pub fn public_uri(&self) -> &str {
        self.bundle.public_uri()
    }

    pub fn auditor_id(&self) -> Address {
        self.auditor_id.into_address()
    }

    pub fn audit_uri(&self) -> &str {
        &self.audit_uri
    }

    pub fn observed_at(&self) -> u64 {
        self.observation.observed_at()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct PublicationBundleArgs {
    #[command(flatten)]
    bundle: EvidenceBundleIdArgs,
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public URI where the evidence bundle is published."
    )]
    public_uri: String,
}

impl PublicationBundleArgs {
    #[cfg(test)]
    pub(crate) fn new(bundle: EvidenceBundleIdArgs, public_uri: impl Into<String>) -> Self {
        Self {
            bundle,
            public_uri: public_uri.into(),
        }
    }

    pub fn bundle_id(&self) -> Hash {
        self.bundle.id()
    }

    pub fn public_uri(&self) -> &str {
        &self.public_uri
    }
}
