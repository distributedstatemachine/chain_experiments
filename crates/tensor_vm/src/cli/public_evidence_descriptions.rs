use super::TvmdCommand;
use super::commands::PublicEvidenceCommand;
use super::commands::PublicTestnetCommand;
use super::public_evidence_network_descriptions::describe_public_evidence_network_command;
use super::public_evidence_node_descriptions::describe_public_evidence_node_command;
use super::public_evidence_publication_descriptions::describe_public_evidence_publication_command;
use super::public_evidence_record_descriptions::describe_public_evidence_record_command;
use super::public_evidence_run_window_descriptions::describe_public_evidence_run_window_command;
use super::public_evidence_service_descriptions::describe_public_evidence_service_command;

pub(super) fn describe_public_evidence_command(command: &TvmdCommand) -> String {
    match command {
        TvmdCommand::PublicEvidence { command } => describe_public_evidence_subcommand(command),
        TvmdCommand::PublicTestnet {
            command: PublicTestnetCommand::Preflight(args),
        } => format!("run public testnet preflight manifest {}", args.manifest),
        _ => unreachable!("local commands are handled by cli::local_descriptions"),
    }
}

fn describe_public_evidence_subcommand(command: &PublicEvidenceCommand) -> String {
    if let Some(description) = describe_public_evidence_service_command(command) {
        return description;
    }
    if let Some(description) = describe_public_evidence_record_command(command) {
        return description;
    }
    if let Some(description) = describe_public_evidence_network_command(command) {
        return description;
    }
    if let Some(description) = describe_public_evidence_publication_command(command) {
        return description;
    }
    if let Some(description) = describe_public_evidence_run_window_command(command) {
        return description;
    }
    if let Some(description) = describe_public_evidence_node_command(command) {
        return description;
    }

    match command {
        PublicEvidenceCommand::Validate(args) => {
            format!("validate public evidence manifest {}", args.manifest)
        }
        _ => unreachable!("handled by public evidence family description modules"),
    }
}
