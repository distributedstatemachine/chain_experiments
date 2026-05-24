use super::commands::{NodeCommand, NodePeerCommand};
use super::validation::{ensure_auth_token, ensure_data_dir, path_argument};
use crate::app::{KeyValueReportWriter, p2p_identity_report};
use crate::error::Result;
use crate::p2p::{Libp2pControlPlaneConfig, PeerRecord};

pub(super) fn execute_node_command(command: &NodeCommand) -> Result<String> {
    match command {
        NodeCommand::Init(args) => {
            ensure_data_dir(&args.data_dir)?;
            let data_dir = path_argument(&args.data_dir);
            let mut report = KeyValueReportWriter::new();
            report.field("command", "service_init");
            report.field("data_dir", data_dir);
            report.field("node_store_ready", true);
            Ok(report.finish())
        }
        NodeCommand::Peer(NodePeerCommand::Add(args)) => {
            ensure_data_dir(&args.data_dir)?;
            let peer_id = args.peer_id.to_string();
            let address = args.address.to_string();
            let record = PeerRecord::from_strings(&peer_id, &address)?;
            let peer_id = record.peer_id()?;
            let data_dir = path_argument(&args.data_dir);
            let mut report = KeyValueReportWriter::new();
            report.field("command", "service_peer_add");
            report.field("data_dir", data_dir);
            report.field("peer_id", peer_id);
            report.field("address", &args.address);
            report.field("peer_book_ready", true);
            Ok(report.finish())
        }
        NodeCommand::Check(args) => {
            ensure_data_dir(&args.data_dir)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = p2p_identity_report(args.identity_seed.map(|seed| seed.into_hash()));
            let data_dir = path_argument(&args.data_dir);
            let mut report = KeyValueReportWriter::new();
            report.field("command", "service_readiness");
            report.field("p2p_listen", &args.p2p_listen);
            report.field("p2p_runtime", "libp2p");
            report.field("p2p_gossipsub", "enabled");
            report.field("p2p_identify", "enabled");
            report.field("p2p_kademlia", "enabled");
            report.field("p2p_request_response", "enabled");
            report.append_report(&identity);
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
            report.field("data_dir", data_dir);
            report.field("node_store_required", true);
            report.field("libp2p_ready", true);
            Ok(report.finish())
        }
        NodeCommand::Serve(args) => {
            let runtime = &args.runtime;
            ensure_data_dir(&runtime.data_dir)?;
            ensure_auth_token(&runtime.auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = p2p_identity_report(runtime.identity_seed.map(|seed| seed.into_hash()));
            let data_dir = path_argument(&runtime.data_dir);
            let mut report = KeyValueReportWriter::new();
            report.field("command", "service_serve");
            report.field("listen", runtime.listen);
            report.field("p2p_listen", &runtime.p2p_listen);
            report.field("p2p_runtime", "libp2p");
            report.field("p2p_gossipsub", "enabled");
            report.field("p2p_identify", "enabled");
            report.field("p2p_kademlia", "enabled");
            report.field("p2p_request_response", "enabled");
            report.append_report(&identity);
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
            report.field("data_dir", data_dir);
            report.field("auth_enabled", true);
            report.field("max_requests", runtime.max_requests);
            report.field("rpc_routes", "enabled");
            report.field("explorer_routes", "enabled");
            report.field("faucet_routes", "enabled");
            report.field("telemetry_routes", "enabled");
            report.field("node_store_required", true);
            Ok(report.finish())
        }
        NodeCommand::Status(args) => {
            ensure_data_dir(&args.data_dir)?;
            let data_dir = path_argument(&args.data_dir);
            let mut report = KeyValueReportWriter::new();
            report.field("command", "service_status");
            report.field("data_dir", data_dir);
            report.field("status_source", "node_store");
            Ok(report.finish())
        }
        NodeCommand::Block(args) => {
            ensure_data_dir(&args.data_dir)?;
            let data_dir = path_argument(&args.data_dir);
            let mut report = KeyValueReportWriter::new();
            report.field("command", "service_block");
            report.field("data_dir", data_dir);
            report.field("height", args.height);
            report.field("status_source", "node_store");
            Ok(report.finish())
        }
    }
}
