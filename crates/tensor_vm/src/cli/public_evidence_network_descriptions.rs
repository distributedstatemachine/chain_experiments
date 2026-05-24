use super::commands::EvidenceNetworkCommand;
use super::validation::path_argument;

pub(super) fn describe_public_evidence_network_command(command: &EvidenceNetworkCommand) -> String {
    match command {
        EvidenceNetworkCommand::Observation(args) => format!(
            "generate signed libp2p network observation peer_id={} listen_address={}",
            args.peer_id, args.listen_address
        ),
        EvidenceNetworkCommand::FromServiceLog(args) => format!(
            "generate signed libp2p network observation from service log service_log={} listen_address={}",
            path_argument(&args.service_log),
            args.listen_address
        ),
    }
}
