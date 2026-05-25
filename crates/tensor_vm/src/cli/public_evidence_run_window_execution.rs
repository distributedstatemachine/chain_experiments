use super::public_evidence_commands::EvidenceRunCommand;
use super::public_evidence_run_window_commands::RunWindowContextArgs;
use super::run_window_evidence::{run_window_evidence_line, run_window_evidence_line_from_file};
use super::validation::path_argument;
use crate::error::Result;
use crate::types::{Address, Hash};

pub(super) fn execute_public_evidence_run_window_command(
    command: &EvidenceRunCommand,
) -> Result<String> {
    match command {
        EvidenceRunCommand::Window(args) => {
            let context = run_window_context(&args.context);
            run_window_evidence_line(
                context.bundle_id,
                context.manifest_signer,
                args.started_at,
                args.ended_at,
                args.observed_blocks,
            )
        }
        EvidenceRunCommand::WindowFile(args) => {
            let context = run_window_context(&args.context);
            run_window_evidence_line_from_file(
                context.bundle_id,
                context.manifest_signer,
                &path_argument(&args.block_observation_file),
            )
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RunWindowContext {
    bundle_id: Hash,
    manifest_signer: Address,
}

fn run_window_context(args: &RunWindowContextArgs) -> RunWindowContext {
    RunWindowContext {
        bundle_id: args.bundle.bundle_id.into_hash(),
        manifest_signer: args.signer.manifest_signer.into_address(),
    }
}
