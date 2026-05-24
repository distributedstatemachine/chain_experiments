use super::commands::EvidenceRunCommand;
use super::validation::path_argument;

pub(super) fn describe_public_evidence_run_window_command(command: &EvidenceRunCommand) -> String {
    match command {
        EvidenceRunCommand::Window(args) => format!(
            "generate public evidence run window started={} ended={} observed_blocks={}",
            args.started_at, args.ended_at, args.observed_blocks
        ),
        EvidenceRunCommand::WindowFile(args) => format!(
            "generate public evidence run window from captured block observations block_observation_file={}",
            path_argument(&args.block_observation_file)
        ),
    }
}
