use super::public_evidence_commands::EvidenceRecordCommand;
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
        EvidenceRecordCommand::Summary(args) => record_summary_evidence_lines(
            args.kind(),
            args.bundle_id(),
            args.manifest_signer(),
            args.root(),
            args.count(),
        ),
        EvidenceRecordCommand::Artifact(args) => record_artifact_evidence_line(
            args.kind(),
            args.bundle_id(),
            args.manifest_signer(),
            args.artifact_uri(),
            args.root(),
            args.count(),
        ),
        EvidenceRecordCommand::ArtifactRoots(args) => {
            let roots = args.roots();
            record_artifact_from_roots(
                args.kind(),
                args.bundle_id(),
                args.manifest_signer(),
                args.artifact_uri(),
                &roots,
            )
        }
        EvidenceRecordCommand::ArtifactFile(args) => record_artifact_from_file(
            args.kind(),
            args.bundle_id(),
            args.manifest_signer(),
            args.artifact_uri(),
            &path_argument(args.file_path()),
        ),
        EvidenceRecordCommand::SummaryRoots(args) => {
            let roots = args.roots();
            record_summary_from_roots(
                args.kind(),
                args.bundle_id(),
                args.manifest_signer(),
                &roots,
            )
        }
        EvidenceRecordCommand::SummaryFile(args) => record_summary_from_file(
            args.kind(),
            args.bundle_id(),
            args.manifest_signer(),
            &path_argument(args.file_path()),
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
