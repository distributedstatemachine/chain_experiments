use super::public_evidence_commands::EvidenceRecordCommand;
use super::public_evidence_record_commands::{PublicEvidenceRecordContextArgs, RecordRootsArgs};
use super::record_evidence::{record_artifact_evidence_line, record_summary_evidence_lines};
use super::record_evidence_roots::{
    aggregate_public_evidence_record_roots, public_evidence_record_roots_from_file,
};
use super::validation::path_argument;
use crate::error::Result;
use crate::testnet::PublicEvidenceRecordKind;
use crate::types::{Address, Hash};

pub(super) fn execute_public_evidence_record_command(
    command: &EvidenceRecordCommand,
) -> Result<String> {
    match command {
        EvidenceRecordCommand::Summary(args) => record_summary_from_root(
            record_context(&args.context),
            args.root.record_root.into_hash(),
            args.root.record_count,
        ),
        EvidenceRecordCommand::Artifact(args) => record_artifact_from_root(
            record_context(&args.context),
            &args.artifact.artifact_uri,
            args.root.record_root.into_hash(),
            args.root.record_count,
        ),
        EvidenceRecordCommand::ArtifactRoots(args) => record_artifact_from_roots(
            record_context(&args.context),
            &args.artifact.artifact_uri,
            &record_roots(&args.roots),
        ),
        EvidenceRecordCommand::ArtifactFile(args) => record_artifact_from_file(
            record_context(&args.context),
            &args.artifact.artifact_uri,
            &path_argument(&args.file.record_file),
        ),
        EvidenceRecordCommand::SummaryRoots(args) => {
            record_summary_from_roots(record_context(&args.context), &record_roots(&args.roots))
        }
        EvidenceRecordCommand::SummaryFile(args) => record_summary_from_file(
            record_context(&args.context),
            &path_argument(&args.file.record_file),
        ),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RecordEvidenceContext {
    kind: PublicEvidenceRecordKind,
    bundle_id: Hash,
    manifest_signer: Address,
}

fn record_context(args: &PublicEvidenceRecordContextArgs) -> RecordEvidenceContext {
    RecordEvidenceContext {
        kind: args.kind.into(),
        bundle_id: args.bundle.bundle_id.into_hash(),
        manifest_signer: args.signer.manifest_signer.into_address(),
    }
}

fn record_roots(args: &RecordRootsArgs) -> Vec<Hash> {
    args.record_roots
        .iter()
        .copied()
        .map(|root| root.into_hash())
        .collect()
}

fn record_summary_from_root(
    context: RecordEvidenceContext,
    record_root: Hash,
    record_count: u64,
) -> Result<String> {
    record_summary_evidence_lines(
        context.kind,
        context.bundle_id,
        context.manifest_signer,
        record_root,
        record_count,
    )
}

fn record_artifact_from_root(
    context: RecordEvidenceContext,
    artifact_uri: &str,
    record_root: Hash,
    record_count: u64,
) -> Result<String> {
    record_artifact_evidence_line(
        context.kind,
        context.bundle_id,
        context.manifest_signer,
        artifact_uri,
        record_root,
        record_count,
    )
}

fn record_artifact_from_roots(
    context: RecordEvidenceContext,
    artifact_uri: &str,
    record_roots: &[Hash],
) -> Result<String> {
    let record_root = aggregate_public_evidence_record_roots(context.kind, record_roots)?;
    record_artifact_from_root(
        context,
        artifact_uri,
        record_root,
        record_roots.len() as u64,
    )
}

fn record_artifact_from_file(
    context: RecordEvidenceContext,
    artifact_uri: &str,
    record_file: &str,
) -> Result<String> {
    let record_roots = public_evidence_record_roots_from_file(context.kind, record_file)?;
    record_artifact_from_roots(context, artifact_uri, &record_roots)
}

fn record_summary_from_roots(
    context: RecordEvidenceContext,
    record_roots: &[Hash],
) -> Result<String> {
    let record_root = aggregate_public_evidence_record_roots(context.kind, record_roots)?;
    record_summary_from_root(context, record_root, record_roots.len() as u64)
}

fn record_summary_from_file(context: RecordEvidenceContext, record_file: &str) -> Result<String> {
    let record_roots = public_evidence_record_roots_from_file(context.kind, record_file)?;
    record_summary_from_roots(context, &record_roots)
}
