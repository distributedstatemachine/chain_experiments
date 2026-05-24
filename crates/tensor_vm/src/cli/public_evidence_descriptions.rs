use super::CliCommand;
use super::arguments::public_node_role_tag;
use super::commands::PublicTestnetCommand;
use super::public_evidence_parser::PublicEvidenceCommand;
use super::public_evidence_record_descriptions::describe_public_evidence_record_command;
use super::public_evidence_service_descriptions::describe_public_evidence_service_command;
use crate::hash::hex;

pub(super) fn describe_public_evidence_command(command: &CliCommand) -> String {
    match command {
        CliCommand::PublicEvidence { command } => describe_public_evidence_subcommand(command),
        CliCommand::PublicTestnet {
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

    match command {
        PublicEvidenceCommand::Validate(args) => {
            format!("validate public evidence manifest {}", args.manifest)
        }
        PublicEvidenceCommand::NetworkObservation(args) => {
            format!(
                "generate signed libp2p network observation peer_id={} listen_address={}",
                args.peer_id, args.listen_address
            )
        }
        PublicEvidenceCommand::NetworkObservationFromServiceLog(args) => {
            format!(
                "generate signed libp2p network observation from service log service_log={} listen_address={}",
                args.service_log, args.listen_address
            )
        }
        PublicEvidenceCommand::Publication(args) => {
            format!(
                "generate public evidence publication signature public_uri={}",
                args.public_uri
            )
        }
        PublicEvidenceCommand::AuditorRecord(args) => {
            format!(
                "generate public evidence auditor record auditor_id={} audit_uri={}",
                hex(&args.auditor_id),
                args.audit_uri
            )
        }
        PublicEvidenceCommand::RunWindow(args) => {
            format!(
                "generate public evidence run window started={} ended={} observed_blocks={}",
                args.started_at, args.ended_at, args.observed_blocks
            )
        }
        PublicEvidenceCommand::RunWindowFromFile(args) => {
            format!(
                "generate public evidence run window from captured block observations block_observation_file={}",
                args.block_observation_file
            )
        }
        PublicEvidenceCommand::NodeHeartbeat(args) => {
            format!(
                "generate {} node heartbeat evidence address={}",
                public_node_role_tag(args.role.into()),
                hex(&args.address)
            )
        }
        PublicEvidenceCommand::NodeHeartbeatFromFile(args) => {
            format!(
                "generate {} node heartbeat evidence from captured observations heartbeat_file={} address={}",
                public_node_role_tag(args.role.into()),
                args.heartbeat_file,
                hex(&args.address)
            )
        }
        PublicEvidenceCommand::OperatorAttestation(args) => {
            format!(
                "generate {} operator identity attestation address={} identity_uri={}",
                public_node_role_tag(args.role.into()),
                hex(&args.address),
                args.identity_uri
            )
        }
        _ => unreachable!("handled by public evidence family description modules"),
    }
}
