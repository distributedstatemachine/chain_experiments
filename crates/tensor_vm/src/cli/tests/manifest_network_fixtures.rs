use crate::hash::hex;
use crate::testnet::{
    PublicEvidenceRecordKind, PublicTestnetRunEvidence, aggregate_public_evidence_record_roots,
    public_network_runtime_observations_for_run,
};
use crate::types::Hash;

pub(super) fn network_runtime_root_for_run(run: &PublicTestnetRunEvidence) -> Hash {
    let record_roots = public_network_runtime_observations_for_run(run)
        .iter()
        .map(|observation| observation.record_root)
        .collect::<Vec<_>>();
    aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &record_roots,
    )
    .expect("generated network observation roots should aggregate")
}

pub(super) fn manifest_network_observation_lines_for_run(run: &PublicTestnetRunEvidence) -> String {
    public_network_runtime_observations_for_run(run)
        .iter()
        .map(|observation| {
            format!(
                "network_runtime_observation={},{},{},{},{},{},{},{},{},{},{},{},{}",
                hex(&observation.operator_id),
                observation.peer_id,
                observation.listen_address,
                observation.observed_at_unix_seconds,
                observation.gossip_topic_count,
                observation.request_response_protocol_count,
                observation.bootstrap_peer_count,
                observation.max_transmit_bytes,
                observation.request_timeout_seconds,
                observation.max_concurrent_streams,
                observation.idle_connection_timeout_seconds,
                hex(&observation.record_root),
                hex(&observation.observation_signature)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
