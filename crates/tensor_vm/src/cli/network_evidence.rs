use super::evidence_fields::{exact_comma_fields, parse_hash_field, parse_u64_field};
use crate::app::{KeyValueReport, KeyValueReportError, KeyValueReportWriter};
use crate::error::{Result, TvmError};
use crate::hash::hex;
use crate::testnet::public_network_runtime_multiaddr_is_external;
use crate::types::{Hash, hash_bytes};
use libp2p::{Multiaddr, PeerId};

pub(super) fn network_observation_root_from_record_line(record: &str) -> Result<Hash> {
    let fields = exact_comma_fields(record, 13, "invalid network observation record line")?;
    let operator_id = parse_hash_field(fields[0])?;
    if operator_id == [0; 32] {
        return Err(TvmError::InvalidReceipt("operator id argument is empty"));
    }
    let peer_id = fields[1]
        .parse::<PeerId>()
        .map_err(|_| TvmError::InvalidReceipt("invalid libp2p peer id"))?
        .to_string();
    let listen_address = fields[2]
        .parse::<Multiaddr>()
        .map_err(|_| TvmError::InvalidReceipt("invalid libp2p multiaddr"))?;
    if !public_network_runtime_multiaddr_is_external(&listen_address) {
        return Err(TvmError::InvalidReceipt(
            "network observation address is not public",
        ));
    }
    let listen_address = listen_address.to_string();
    let input = NetworkObservationEvidenceLine {
        operator_id,
        peer_id: &peer_id,
        listen_address: &listen_address,
        observed_at_unix_seconds: parse_u64_field(fields[3])?,
        gossip_topic_count: parse_u64_field(fields[4])?,
        request_response_protocol_count: parse_u64_field(fields[5])?,
        bootstrap_peer_count: parse_u64_field(fields[6])?,
        max_transmit_bytes: parse_u64_field(fields[7])?,
        request_timeout_seconds: parse_u64_field(fields[8])?,
        max_concurrent_streams: parse_u64_field(fields[9])?,
        idle_connection_timeout_seconds: parse_u64_field(fields[10])?,
    };
    if input.observed_at_unix_seconds == 0
        || input.gossip_topic_count == 0
        || input.request_response_protocol_count == 0
        || input.bootstrap_peer_count == 0
        || input.max_transmit_bytes == 0
        || input.request_timeout_seconds == 0
        || input.max_concurrent_streams == 0
        || input.idle_connection_timeout_seconds == 0
    {
        return Err(TvmError::InvalidReceipt(
            "invalid network observation record line",
        ));
    }
    let record_root = parse_record_file_root(fields[11])?;
    let record_signature = parse_record_file_root(fields[12])?;
    let expected_root = network_observation_root(&input, &peer_id, &listen_address);
    let expected_signature = hash_bytes(
        b"tensor-vm-network-runtime-observation-signature-v1",
        &[&operator_id, &expected_root],
    );
    if record_root != expected_root || record_signature != expected_signature {
        return Err(TvmError::InvalidReceipt(
            "invalid network observation record line",
        ));
    }
    Ok(record_root)
}

fn parse_record_file_root(root: &str) -> Result<Hash> {
    if root.trim() != root {
        return Err(TvmError::InvalidReceipt("invalid record root file line"));
    }
    parse_hash_field(root)
}

pub(super) struct NetworkObservationEvidenceLine<'a> {
    pub(super) operator_id: Hash,
    pub(super) peer_id: &'a str,
    pub(super) listen_address: &'a str,
    pub(super) observed_at_unix_seconds: u64,
    pub(super) gossip_topic_count: u64,
    pub(super) request_response_protocol_count: u64,
    pub(super) bootstrap_peer_count: u64,
    pub(super) max_transmit_bytes: u64,
    pub(super) request_timeout_seconds: u64,
    pub(super) max_concurrent_streams: u64,
    pub(super) idle_connection_timeout_seconds: u64,
}

pub(super) fn network_observation_evidence_line(
    input: NetworkObservationEvidenceLine<'_>,
) -> Result<String> {
    if input.operator_id == [0; 32] {
        return Err(TvmError::InvalidReceipt("operator id argument is empty"));
    }
    if input.observed_at_unix_seconds == 0 {
        return Err(TvmError::InvalidReceipt("observed-at argument is empty"));
    }
    if input.gossip_topic_count == 0 {
        return Err(TvmError::InvalidReceipt("gossip topics argument is empty"));
    }
    if input.request_response_protocol_count == 0 {
        return Err(TvmError::InvalidReceipt(
            "request-response protocols argument is empty",
        ));
    }
    if input.bootstrap_peer_count == 0 {
        return Err(TvmError::InvalidReceipt(
            "bootstrap peers argument is empty",
        ));
    }
    if input.max_transmit_bytes == 0
        || input.request_timeout_seconds == 0
        || input.max_concurrent_streams == 0
        || input.idle_connection_timeout_seconds == 0
    {
        return Err(TvmError::InvalidReceipt(
            "network runtime control arguments must be positive",
        ));
    }

    let peer_id = input
        .peer_id
        .parse::<PeerId>()
        .map_err(|_| TvmError::InvalidReceipt("invalid libp2p peer id"))?;
    let listen_address = input
        .listen_address
        .parse::<Multiaddr>()
        .map_err(|_| TvmError::InvalidReceipt("invalid libp2p multiaddr"))?;
    if !public_network_runtime_multiaddr_is_external(&listen_address) {
        return Err(TvmError::InvalidReceipt(
            "network observation address is not public",
        ));
    }

    let peer_id = peer_id.to_string();
    let listen_address = listen_address.to_string();
    let root = network_observation_root(&input, &peer_id, &listen_address);
    let signature = hash_bytes(
        b"tensor-vm-network-runtime-observation-signature-v1",
        &[&input.operator_id, &root],
    );
    let mut report = KeyValueReportWriter::new();
    report.field(
        "network_runtime_observation",
        format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            hex(&input.operator_id),
            peer_id,
            listen_address,
            input.observed_at_unix_seconds,
            input.gossip_topic_count,
            input.request_response_protocol_count,
            input.bootstrap_peer_count,
            input.max_transmit_bytes,
            input.request_timeout_seconds,
            input.max_concurrent_streams,
            input.idle_connection_timeout_seconds,
            hex(&root),
            hex(&signature)
        ),
    );
    Ok(report.finish())
}

pub(super) fn network_observation_evidence_line_from_service_log(
    operator_id: Hash,
    listen_address: &str,
    observed_at_unix_seconds: u64,
    service_log: &str,
) -> Result<String> {
    let service_log = ServiceLogFields::parse(service_log)?;
    if service_log.field("command")? != "service_serve" {
        return Err(TvmError::InvalidReceipt("service log is not service_serve"));
    }
    if service_log.field("p2p_runtime")? != "libp2p" {
        return Err(TvmError::InvalidReceipt(
            "service log does not prove libp2p runtime",
        ));
    }
    network_observation_evidence_line(NetworkObservationEvidenceLine {
        operator_id,
        peer_id: service_log.field("p2p_peer_id")?,
        listen_address,
        observed_at_unix_seconds,
        gossip_topic_count: parse_u64_field(service_log.field("p2p_gossipsub_topics")?)?,
        request_response_protocol_count: parse_u64_field(
            service_log.field("p2p_request_response_protocols")?,
        )?,
        bootstrap_peer_count: parse_u64_field(service_log.field("p2p_bootstrap_peers")?)?,
        max_transmit_bytes: parse_u64_field(service_log.field("p2p_max_transmit_bytes")?)?,
        request_timeout_seconds: parse_u64_field(
            service_log.field("p2p_request_timeout_seconds")?,
        )?,
        max_concurrent_streams: parse_u64_field(service_log.field("p2p_max_concurrent_streams")?)?,
        idle_connection_timeout_seconds: parse_u64_field(
            service_log.field("p2p_idle_timeout_seconds")?,
        )?,
    })
}

#[cfg(test)]
pub(super) fn service_log_field<'a>(service_log: &'a str, key: &str) -> Result<&'a str> {
    ServiceLogFields::parse(service_log)?.field(key)
}

struct ServiceLogFields<'a> {
    report: KeyValueReport<'a>,
}

impl<'a> ServiceLogFields<'a> {
    fn parse(service_log: &'a str) -> Result<Self> {
        let report = KeyValueReport::parse_strict(service_log).map_err(service_log_parse_error)?;
        Ok(Self { report })
    }

    fn field(&self, key: &str) -> Result<&'a str> {
        self.report
            .value(key)
            .ok_or(TvmError::InvalidReceipt("missing service log field"))
    }
}

fn service_log_parse_error(error: KeyValueReportError) -> TvmError {
    match error {
        KeyValueReportError::DuplicateField => {
            TvmError::InvalidReceipt("duplicate service log field")
        }
        KeyValueReportError::InvalidField => TvmError::InvalidReceipt("invalid service log field"),
    }
}

pub(super) fn network_observation_root(
    input: &NetworkObservationEvidenceLine<'_>,
    peer_id: &str,
    listen_address: &str,
) -> Hash {
    let observed_at = input.observed_at_unix_seconds.to_le_bytes();
    let gossip_topics = input.gossip_topic_count.to_le_bytes();
    let request_response_protocols = input.request_response_protocol_count.to_le_bytes();
    let bootstrap_peers = input.bootstrap_peer_count.to_le_bytes();
    let max_transmit_bytes = input.max_transmit_bytes.to_le_bytes();
    let request_timeout = input.request_timeout_seconds.to_le_bytes();
    let max_streams = input.max_concurrent_streams.to_le_bytes();
    let idle_timeout = input.idle_connection_timeout_seconds.to_le_bytes();
    hash_bytes(
        b"tensor-vm-network-runtime-observation-v1",
        &[
            &input.operator_id,
            peer_id.as_bytes(),
            listen_address.as_bytes(),
            &observed_at,
            &gossip_topics,
            &request_response_protocols,
            &bootstrap_peers,
            &max_transmit_bytes,
            &request_timeout,
            &max_streams,
            &idle_timeout,
        ],
    )
}
