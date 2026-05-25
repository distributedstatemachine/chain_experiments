use super::public_evidence_bundle_commands::EvidenceBundleIdArgs;
use super::public_evidence_observation_commands::ObservationTimestampArgs;
use super::public_evidence_signing_commands::ManifestSignerArgs;
use super::value_types::AddressArg;
use crate::types::{Address, Hash};
use clap::{Args, ValueHint};

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicationArgs {
    #[command(flatten)]
    pub bundle: PublicationBundleArgs,
    #[command(flatten)]
    pub signer: ManifestSignerArgs,
    #[arg(
        long,
        value_name = "N",
        help = "Number of manifest signatures included."
    )]
    pub manifest_signature_count: u64,
    #[arg(
        long,
        value_name = "N",
        help = "Number of independent auditor records included."
    )]
    pub independent_auditor_count: u64,
}

impl PublicationArgs {
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
pub struct AuditorRecordArgs {
    #[command(flatten)]
    pub bundle: PublicationBundleArgs,
    #[arg(
        long,
        value_name = "HEX",
        help = "Address or identifier of the independent auditor."
    )]
    pub auditor_id: AddressArg,
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public URI for the auditor's review artifact."
    )]
    pub audit_uri: String,
    #[command(flatten)]
    pub observation: ObservationTimestampArgs,
}

impl AuditorRecordArgs {
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
pub struct PublicationBundleArgs {
    #[command(flatten)]
    pub bundle: EvidenceBundleIdArgs,
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public URI where the evidence bundle is published."
    )]
    pub public_uri: String,
}

impl PublicationBundleArgs {
    pub fn bundle_id(&self) -> Hash {
        self.bundle.id()
    }

    pub fn public_uri(&self) -> &str {
        &self.public_uri
    }
}
