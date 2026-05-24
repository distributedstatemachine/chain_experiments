use crate::app::KeyValueReportWriter;
use crate::p2p::Libp2pControlPlaneConfig;

pub(super) fn write_libp2p_fixture_fields(report: &mut KeyValueReportWriter) {
    report.field("p2p_runtime", "libp2p");
    report.field("p2p_gossipsub", "enabled");
    report.field("p2p_identify", "enabled");
    report.field("p2p_kademlia", "enabled");
    report.field("p2p_request_response", "enabled");
}

pub(super) fn write_default_libp2p_limit_fields(report: &mut KeyValueReportWriter) {
    let p2p_config = Libp2pControlPlaneConfig::default();
    report.field(
        "p2p_max_transmit_bytes",
        p2p_config.max_gossipsub_transmit_bytes,
    );
    report.field(
        "p2p_request_timeout_seconds",
        p2p_config.request_timeout_seconds,
    );
    report.field(
        "p2p_max_concurrent_streams",
        p2p_config.max_concurrent_request_streams,
    );
    report.field(
        "p2p_idle_timeout_seconds",
        p2p_config.idle_connection_timeout_seconds,
    );
}
