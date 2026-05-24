use super::local_execution_values::identity_report;
use super::local_parser::{ServiceCommand, ServicePeerCommand};
use super::validation::{
    ensure_auth_token, ensure_data_dir, ensure_libp2p_multiaddr, ensure_listen_addr,
};
use crate::error::Result;
use crate::p2p::{Libp2pControlPlaneConfig, PeerRecord};

pub(super) fn execute_service_command(command: &ServiceCommand) -> Result<String> {
    match command {
        ServiceCommand::Init(args) => {
            ensure_data_dir(&args.data_dir)?;
            Ok(format!(
                "command=service_init\ndata_dir={}\nnode_store_ready=true",
                args.data_dir
            ))
        }
        ServiceCommand::Peer {
            command: ServicePeerCommand::Add(args),
        } => {
            ensure_data_dir(&args.data_dir)?;
            let record = PeerRecord::from_strings(&args.peer_id, &args.address)?;
            let peer_id = record.peer_id()?;
            Ok(format!(
                "command=service_peer_add\ndata_dir={}\npeer_id={peer_id}\naddress={}\npeer_book_ready=true",
                args.data_dir, args.address
            ))
        }
        ServiceCommand::Readiness(args) => {
            ensure_libp2p_multiaddr(&args.p2p_listen)?;
            ensure_data_dir(&args.data_dir)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_report(args.identity_seed);
            Ok(format!(
                "command=service_readiness\np2p_listen={}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={}\nnode_store_required=true\nlibp2p_ready=true",
                args.p2p_listen,
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds,
                args.data_dir
            ))
        }
        ServiceCommand::Serve(args) => {
            ensure_listen_addr(&args.listen)?;
            ensure_libp2p_multiaddr(&args.p2p_listen)?;
            ensure_data_dir(&args.data_dir)?;
            ensure_auth_token(&args.auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_report(args.identity_seed);
            Ok(format!(
                "command=service_serve\nlisten={}\np2p_listen={}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={}\nauth_enabled=true\nmax_requests={}\nrpc_routes=enabled\nexplorer_routes=enabled\nfaucet_routes=enabled\ntelemetry_routes=enabled\nnode_store_required=true",
                args.listen,
                args.p2p_listen,
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds,
                args.data_dir,
                args.max_requests
            ))
        }
        ServiceCommand::Status(args) => {
            ensure_data_dir(&args.data_dir)?;
            Ok(format!(
                "command=service_status\ndata_dir={}\nstatus_source=node_store",
                args.data_dir
            ))
        }
        ServiceCommand::Block(args) => {
            ensure_data_dir(&args.data_dir)?;
            Ok(format!(
                "command=service_block\ndata_dir={}\nheight={}\nstatus_source=node_store",
                args.data_dir, args.height
            ))
        }
    }
}
