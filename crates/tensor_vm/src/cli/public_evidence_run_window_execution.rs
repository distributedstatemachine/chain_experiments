use super::CliCommand;
use super::run_window_evidence::{run_window_evidence_line, run_window_evidence_line_from_file};
use crate::error::Result;

pub(super) fn execute_public_evidence_run_window_command(
    command: &CliCommand,
) -> Option<Result<String>> {
    match command {
        CliCommand::PublicEvidenceRunWindow {
            bundle_id,
            manifest_signer,
            run_started_at_unix_seconds,
            run_ended_at_unix_seconds,
            observed_blocks,
        } => Some(run_window_evidence_line(
            *bundle_id,
            *manifest_signer,
            *run_started_at_unix_seconds,
            *run_ended_at_unix_seconds,
            *observed_blocks,
        )),
        CliCommand::PublicEvidenceRunWindowFromFile {
            bundle_id,
            manifest_signer,
            block_observation_file,
        } => Some(run_window_evidence_line_from_file(
            *bundle_id,
            *manifest_signer,
            block_observation_file,
        )),
        _ => None,
    }
}
