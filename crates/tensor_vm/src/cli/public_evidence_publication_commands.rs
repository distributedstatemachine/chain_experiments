use super::value_types::{AddressArg, HashArg};
use clap::{Args, ValueHint};

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct PublicationArgs {
    #[arg(long, value_name = "HEX", help = "Public evidence bundle identifier.")]
    pub bundle_id: HashArg,
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public URI where the evidence bundle is published."
    )]
    pub public_uri: String,
    #[arg(
        long,
        value_name = "HEX",
        help = "Address signing the evidence manifest."
    )]
    pub manifest_signer: AddressArg,
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
    #[arg(long, value_name = "HEX", help = "Public evidence bundle identifier.")]
    pub bundle_id: HashArg,
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Url,
        help = "Public URI where the evidence bundle is published."
    )]
    pub public_uri: String,
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
    #[arg(
        long,
        value_name = "UNIX_SECONDS",
        help = "Unix timestamp for the audit observation."
    )]
    pub observed_at: u64,
}
