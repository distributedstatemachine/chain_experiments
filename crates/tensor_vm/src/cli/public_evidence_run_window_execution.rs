use super::commands::PublicEvidenceCommand;
use super::run_window_evidence::{run_window_evidence_line, run_window_evidence_line_from_file};
use super::validation::path_argument;
use crate::error::Result;

pub(super) fn execute_public_evidence_run_window_command(
    command: &PublicEvidenceCommand,
) -> Option<Result<String>> {
    match command {
        PublicEvidenceCommand::RunWindow(args) => Some(run_window_evidence_line(
            args.bundle_id,
            args.manifest_signer,
            args.started_at,
            args.ended_at,
            args.observed_blocks,
        )),
        PublicEvidenceCommand::RunWindowFromFile(args) => Some(run_window_evidence_line_from_file(
            args.bundle_id,
            args.manifest_signer,
            &path_argument(&args.block_observation_file),
        )),
        _ => None,
    }
}
