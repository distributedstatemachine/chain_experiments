use super::commands::EvidenceRecordCommand;
use super::record_evidence::{
    aggregate_public_evidence_record_roots, public_evidence_record_roots_from_file,
    record_artifact_evidence_line, record_summary_evidence_lines,
};
use super::validation::path_argument;
use crate::error::Result;
use crate::testnet::PublicEvidenceRecordKind;
use crate::types::{Address, Hash};

pub(super) fn execute_public_evidence_record_command(
    command: &EvidenceRecordCommand,
) -> Result<String> {
    match command {
        EvidenceRecordCommand::Summary(args) => record_summary_evidence_lines(
            args.kind.into(),
            args.bundle_id,
            args.manifest_signer,
            args.record_root,
            args.record_count,
        ),
        EvidenceRecordCommand::Artifact(args) => record_artifact_evidence_line(
            args.kind.into(),
            args.bundle_id,
            args.manifest_signer,
            &args.artifact_uri,
            args.record_root,
            args.record_count,
        ),
        EvidenceRecordCommand::ArtifactRoots(args) => record_artifact_from_roots(
            args.kind.into(),
            args.bundle_id,
            args.manifest_signer,
            &args.artifact_uri,
            &args.record_roots,
        ),
        EvidenceRecordCommand::ArtifactFile(args) => record_artifact_from_file(
            args.kind.into(),
            args.bundle_id,
            args.manifest_signer,
            &args.artifact_uri,
            &path_argument(&args.record_file),
        ),
        EvidenceRecordCommand::SummaryRoots(args) => record_summary_from_roots(
            args.kind.into(),
            args.bundle_id,
            args.manifest_signer,
            &args.record_roots,
        ),
        EvidenceRecordCommand::SummaryFile(args) => record_summary_from_file(
            args.kind.into(),
            args.bundle_id,
            args.manifest_signer,
            &path_argument(&args.record_file),
        ),
    }
}

fn record_artifact_from_roots(
    kind: PublicEvidenceRecordKind,
    bundle_id: Hash,
    manifest_signer: Address,
    artifact_uri: &str,
    record_roots: &[Hash],
) -> Result<String> {
    let record_root = aggregate_public_evidence_record_roots(kind, record_roots)?;
    record_artifact_evidence_line(
        kind,
        bundle_id,
        manifest_signer,
        artifact_uri,
        record_root,
        record_roots.len() as u64,
    )
}

fn record_artifact_from_file(
    kind: PublicEvidenceRecordKind,
    bundle_id: Hash,
    manifest_signer: Address,
    artifact_uri: &str,
    record_file: &str,
) -> Result<String> {
    let record_roots = public_evidence_record_roots_from_file(kind, record_file)?;
    record_artifact_from_roots(
        kind,
        bundle_id,
        manifest_signer,
        artifact_uri,
        &record_roots,
    )
}

fn record_summary_from_roots(
    kind: PublicEvidenceRecordKind,
    bundle_id: Hash,
    manifest_signer: Address,
    record_roots: &[Hash],
) -> Result<String> {
    let record_root = aggregate_public_evidence_record_roots(kind, record_roots)?;
    record_summary_evidence_lines(
        kind,
        bundle_id,
        manifest_signer,
        record_root,
        record_roots.len() as u64,
    )
}

fn record_summary_from_file(
    kind: PublicEvidenceRecordKind,
    bundle_id: Hash,
    manifest_signer: Address,
    record_file: &str,
) -> Result<String> {
    let record_roots = public_evidence_record_roots_from_file(kind, record_file)?;
    record_summary_from_roots(kind, bundle_id, manifest_signer, &record_roots)
}
