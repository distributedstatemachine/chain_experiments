use super::CliCommand;
use super::parser_values::parse_hash_value;
use crate::types::{Address, Hash};
use clap::Args;

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct PublicationArgs {
    #[arg(long, value_parser = parse_hash_value)]
    bundle_id: Hash,
    #[arg(long)]
    public_uri: String,
    #[arg(long, value_parser = parse_hash_value)]
    manifest_signer: Address,
    #[arg(long)]
    manifest_signature_count: u64,
    #[arg(long)]
    independent_auditor_count: u64,
}

impl PublicationArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidencePublication {
            bundle_id: self.bundle_id,
            public_uri: self.public_uri,
            manifest_signer: self.manifest_signer,
            manifest_signature_count: self.manifest_signature_count,
            independent_auditor_count: self.independent_auditor_count,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(super) struct AuditorRecordArgs {
    #[arg(long, value_parser = parse_hash_value)]
    bundle_id: Hash,
    #[arg(long)]
    public_uri: String,
    #[arg(long, value_parser = parse_hash_value)]
    auditor_id: Hash,
    #[arg(long)]
    audit_uri: String,
    #[arg(long)]
    observed_at: u64,
}

impl AuditorRecordArgs {
    pub(super) fn into_command(self) -> CliCommand {
        CliCommand::PublicEvidenceAuditorRecord {
            bundle_id: self.bundle_id,
            public_uri: self.public_uri,
            auditor_id: self.auditor_id,
            audit_uri: self.audit_uri,
            observed_at_unix_seconds: self.observed_at,
        }
    }
}
