use super::commands::{NodeCommand, NodePeerCommand};
use super::validation::{ensure_auth_token, ensure_data_dir, path_argument};
use crate::app::p2p_identity_report;
use crate::error::Result;
use crate::p2p::{Libp2pControlPlaneConfig, PeerRecord};

pub(super) fn execute_node_command(command: &NodeCommand) -> Result<String> {
    match command {
        NodeCommand::Init(args) => {
            ensure_data_dir(&args.data_dir)?;
            let data_dir = path_argument(&args.data_dir);
            Ok(format!(
                "command=service_init\ndata_dir={}\nnode_store_ready=true",
                data_dir
            ))
        }
        NodeCommand::Peer(NodePeerCommand::Add(args)) => {
            ensure_data_dir(&args.data_dir)?;
            let peer_id = args.peer_id.to_string();
            let address = args.address.to_string();
            let record = PeerRecord::from_strings(&peer_id, &address)?;
            let peer_id = record.peer_id()?;
            let data_dir = path_argument(&args.data_dir);
            Ok(format!(
                "command=service_peer_add\ndata_dir={}\npeer_id={peer_id}\naddress={}\npeer_book_ready=true",
                data_dir, args.address
            ))
        }
        NodeCommand::Check(args) => {
            ensure_data_dir(&args.data_dir)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = p2p_identity_report(args.identity_seed.map(|seed| seed.into_hash()));
            let data_dir = path_argument(&args.data_dir);
            Ok(format!(
                "command=service_readiness\np2p_listen={}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={}\nnode_store_required=true\nlibp2p_ready=true",
                args.p2p_listen,
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds,
                data_dir
            ))
        }
        NodeCommand::Serve(args) => {
            let runtime = &args.runtime;
            ensure_data_dir(&runtime.data_dir)?;
            ensure_auth_token(&runtime.auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = p2p_identity_report(runtime.identity_seed.map(|seed| seed.into_hash()));
            let data_dir = path_argument(&runtime.data_dir);
            Ok(format!(
                "command=service_serve\nlisten={}\np2p_listen={}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={}\nauth_enabled=true\nmax_requests={}\nrpc_routes=enabled\nexplorer_routes=enabled\nfaucet_routes=enabled\ntelemetry_routes=enabled\nnode_store_required=true",
                runtime.listen,
                runtime.p2p_listen,
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds,
                data_dir,
                runtime.max_requests
            ))
        }
        NodeCommand::Status(args) => {
            ensure_data_dir(&args.data_dir)?;
            let data_dir = path_argument(&args.data_dir);
            Ok(format!(
                "command=service_status\ndata_dir={}\nstatus_source=node_store",
                data_dir
            ))
        }
        NodeCommand::Block(args) => {
            ensure_data_dir(&args.data_dir)?;
            let data_dir = path_argument(&args.data_dir);
            Ok(format!(
                "command=service_block\ndata_dir={}\nheight={}\nstatus_source=node_store",
                data_dir, args.height
            ))
        }
    }
}
