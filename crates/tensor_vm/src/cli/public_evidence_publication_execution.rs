use super::public_evidence_publication_commands::{
    AuditorRecordArgs, PublicationArgs, PublicationBundleArgs,
};
use super::publication_evidence::{auditor_record_evidence_line, publication_evidence_lines};
use crate::error::Result;
use crate::types::Hash;

pub(super) fn execute_publication_evidence(args: &PublicationArgs) -> Result<String> {
    let bundle = publication_bundle(&args.bundle);
    publication_evidence_lines(
        bundle.bundle_id,
        bundle.public_uri,
        args.signer.manifest_signer.into_address(),
        args.manifest_signature_count,
        args.independent_auditor_count,
    )
}

pub(super) fn execute_auditor_record_evidence(args: &AuditorRecordArgs) -> Result<String> {
    let bundle = publication_bundle(&args.bundle);
    auditor_record_evidence_line(
        bundle.bundle_id,
        bundle.public_uri,
        args.auditor_id.into_address(),
        &args.audit_uri,
        args.observation.observed_at,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PublicationBundleContext<'a> {
    bundle_id: Hash,
    public_uri: &'a str,
}

fn publication_bundle(args: &PublicationBundleArgs) -> PublicationBundleContext<'_> {
    PublicationBundleContext {
        bundle_id: args.bundle.bundle_id.into_hash(),
        public_uri: &args.public_uri,
    }
}
