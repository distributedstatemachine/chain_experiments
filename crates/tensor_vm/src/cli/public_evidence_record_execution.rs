use super::CliCommand;
use super::record_evidence::{
    aggregate_public_evidence_record_roots, public_evidence_record_roots_from_file,
    record_artifact_evidence_line, record_summary_evidence_lines,
};
use crate::error::Result;
use crate::testnet::PublicEvidenceRecordKind;
use crate::types::{Address, Hash};

pub(super) fn execute_public_evidence_record_command(
    command: &CliCommand,
) -> Option<Result<String>> {
    match command {
        CliCommand::PublicEvidenceRecordSummary {
            kind,
            bundle_id,
            manifest_signer,
            record_root,
            record_count,
        } => Some(record_summary_evidence_lines(
            *kind,
            *bundle_id,
            *manifest_signer,
            *record_root,
            *record_count,
        )),
        CliCommand::PublicEvidenceRecordArtifact {
            kind,
            bundle_id,
            manifest_signer,
            artifact_uri,
            record_root,
            record_count,
        } => Some(record_artifact_evidence_line(
            *kind,
            *bundle_id,
            *manifest_signer,
            artifact_uri,
            *record_root,
            *record_count,
        )),
        CliCommand::PublicEvidenceRecordArtifactFromRoots {
            kind,
            bundle_id,
            manifest_signer,
            artifact_uri,
            record_roots,
        } => Some(record_artifact_from_roots(
            *kind,
            *bundle_id,
            *manifest_signer,
            artifact_uri,
            record_roots,
        )),
        CliCommand::PublicEvidenceRecordArtifactFromFile {
            kind,
            bundle_id,
            manifest_signer,
            artifact_uri,
            record_file,
        } => Some(record_artifact_from_file(
            *kind,
            *bundle_id,
            *manifest_signer,
            artifact_uri,
            record_file,
        )),
        CliCommand::PublicEvidenceRecordSummaryFromRoots {
            kind,
            bundle_id,
            manifest_signer,
            record_roots,
        } => Some(record_summary_from_roots(
            *kind,
            *bundle_id,
            *manifest_signer,
            record_roots,
        )),
        CliCommand::PublicEvidenceRecordSummaryFromFile {
            kind,
            bundle_id,
            manifest_signer,
            record_file,
        } => Some(record_summary_from_file(
            *kind,
            *bundle_id,
            *manifest_signer,
            record_file,
        )),
        _ => None,
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
