use super::TvmdCommand;
use super::commands::EvidenceCommand;
use super::public_evidence_network_descriptions::describe_public_evidence_network_command;
use super::public_evidence_node_descriptions::describe_public_evidence_node_command;
use super::public_evidence_publication_descriptions::describe_public_evidence_publication_command;
use super::public_evidence_record_descriptions::describe_public_evidence_record_command;
use super::public_evidence_run_window_descriptions::describe_public_evidence_run_window_command;
use super::public_evidence_service_descriptions::describe_public_evidence_service_command;
use super::validation::path_argument;

pub(super) fn describe_public_evidence_command(command: &TvmdCommand) -> String {
    match command {
        TvmdCommand::Evidence(command) => describe_public_evidence_subcommand(command),
        _ => unreachable!("local commands are handled by cli::local_descriptions"),
    }
}

fn describe_public_evidence_subcommand(command: &EvidenceCommand) -> String {
    match command {
        EvidenceCommand::Validate(args) => {
            format!(
                "validate public evidence manifest {}",
                path_argument(&args.manifest)
            )
        }
        EvidenceCommand::Publish(_) | EvidenceCommand::Audit(_) => {
            describe_public_evidence_publication_command(command)
        }
        EvidenceCommand::Run(command) => describe_public_evidence_run_window_command(command),
        EvidenceCommand::Node(command) => describe_public_evidence_node_command(command),
        EvidenceCommand::Service(command) => describe_public_evidence_service_command(command),
        EvidenceCommand::Network(command) => describe_public_evidence_network_command(command),
        EvidenceCommand::Record(command) => describe_public_evidence_record_command(command),
    }
}
