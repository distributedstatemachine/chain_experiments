use super::public_evidence_parser::PublicEvidenceCommand;

pub(super) fn describe_public_evidence_run_window_command(
    command: &PublicEvidenceCommand,
) -> Option<String> {
    match command {
        PublicEvidenceCommand::RunWindow(args) => Some(format!(
            "generate public evidence run window started={} ended={} observed_blocks={}",
            args.started_at, args.ended_at, args.observed_blocks
        )),
        PublicEvidenceCommand::RunWindowFromFile(args) => Some(format!(
            "generate public evidence run window from captured block observations block_observation_file={}",
            args.block_observation_file
        )),
        _ => None,
    }
}
