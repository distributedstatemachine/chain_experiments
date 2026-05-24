use super::public_evidence_parser::PublicEvidenceCommand;

pub(super) fn describe_public_evidence_network_command(
    command: &PublicEvidenceCommand,
) -> Option<String> {
    match command {
        PublicEvidenceCommand::NetworkObservation(args) => Some(format!(
            "generate signed libp2p network observation peer_id={} listen_address={}",
            args.peer_id, args.listen_address
        )),
        PublicEvidenceCommand::NetworkObservationFromServiceLog(args) => Some(format!(
            "generate signed libp2p network observation from service log service_log={} listen_address={}",
            args.service_log, args.listen_address
        )),
        _ => None,
    }
}
