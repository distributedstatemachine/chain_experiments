use super::commands::{NodeCommand, NodePeerCommand};
use super::local_fixture_reports::{
    write_default_libp2p_limit_fields, write_libp2p_fixture_fields,
};
use super::validation::{ensure_auth_token, ensure_data_dir, path_argument};
use crate::app::{KeyValueReportWriter, p2p_identity_report};
use crate::error::Result;
use crate::p2p::PeerRecord;

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
            let identity = p2p_identity_report(args.identity_seed.map(|seed| seed.into_hash()));
            let data_dir = path_argument(&args.data_dir);
            let mut report = KeyValueReportWriter::new();
            report.field("command", "service_readiness");
            report.field("p2p_listen", &args.p2p_listen);
            write_libp2p_fixture_fields(&mut report);
            report.append_report(&identity);
            write_default_libp2p_limit_fields(&mut report);
            report.field("data_dir", data_dir);
            report.field("node_store_required", true);
            report.field("libp2p_ready", true);
            Ok(report.finish())
        }
        NodeCommand::Serve(args) => {
            let runtime = &args.runtime;
            ensure_data_dir(&runtime.data_dir)?;
            ensure_auth_token(&runtime.auth_token)?;
            let identity = p2p_identity_report(runtime.identity_seed.map(|seed| seed.into_hash()));
            let data_dir = path_argument(&runtime.data_dir);
            let mut report = KeyValueReportWriter::new();
            report.field("command", "service_serve");
            report.field("listen", runtime.listen);
            report.field("p2p_listen", &runtime.p2p_listen);
            write_libp2p_fixture_fields(&mut report);
            report.append_report(&identity);
            write_default_libp2p_limit_fields(&mut report);
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
