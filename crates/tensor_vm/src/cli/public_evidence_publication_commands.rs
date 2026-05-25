use super::public_evidence_bundle_commands::EvidenceBundleIdArgs;
use super::public_evidence_observation_commands::ObservationTimestampArgs;
use super::public_evidence_signing_commands::ManifestSignerArgs;
use super::value_types::AddressArg;
use clap::{Args, ValueHint};

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct PublicationArgs {
    #[command(flatten)]
    pub(crate) bundle: PublicationBundleArgs,
    #[command(flatten)]
    pub(crate) signer: ManifestSignerArgs,
    #[arg(
        long,
        value_name = "N",
        help = "Number of manifest signatures included."
    )]
    pub(crate) manifest_signature_count: u64,
    #[arg(
        long,
        value_name = "N",
        help = "Number of independent auditor records included."
    )]
    pub(crate) independent_auditor_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct AuditorRecordArgs {
    #[command(flatten)]
    pub(crate) bundle: PublicationBundleArgs,
    #[arg(
        long,
        value_name = "HEX",
        help = "Address or identifier of the independent auditor."
    )]
    pub(crate) auditor_id: AddressArg,
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public URI for the auditor's review artifact."
    )]
    pub(crate) audit_uri: String,
    #[command(flatten)]
    pub(crate) observation: ObservationTimestampArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct PublicationBundleArgs {
    #[command(flatten)]
    pub(crate) bundle: EvidenceBundleIdArgs,
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public URI where the evidence bundle is published."
    )]
    pub(crate) public_uri: String,
}
