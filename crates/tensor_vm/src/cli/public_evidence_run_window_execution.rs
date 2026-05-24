use super::commands::EvidenceRunCommand;
use super::run_window_evidence::{run_window_evidence_line, run_window_evidence_line_from_file};
use super::validation::path_argument;
use crate::error::Result;

pub(super) fn execute_public_evidence_run_window_command(
    command: &EvidenceRunCommand,
) -> Result<String> {
    match command {
        EvidenceRunCommand::Window(args) => run_window_evidence_line(
            args.bundle_id.into_hash(),
            args.manifest_signer.into_address(),
            args.started_at,
            args.ended_at,
            args.observed_blocks,
        ),
        EvidenceRunCommand::WindowFile(args) => run_window_evidence_line_from_file(
            args.bundle_id.into_hash(),
            args.manifest_signer.into_address(),
            &path_argument(&args.block_observation_file),
        ),
    }
}
