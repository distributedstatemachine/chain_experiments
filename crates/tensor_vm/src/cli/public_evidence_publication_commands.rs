use super::public_evidence_observation_commands::ObservationTimestampArgs;
use super::public_evidence_signing_commands::ManifestSignerArgs;
use super::value_types::{AddressArg, HashArg};
use crate::types::Hash;
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

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicationBundleArgs {
    #[arg(long, value_name = "HEX", help = "Public evidence bundle identifier.")]
    pub bundle_id: HashArg,
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
        self.bundle_id.into_hash()
    }

    pub fn public_uri(&self) -> &str {
        &self.public_uri
    }
}
