use super::local_description_values::identity_description;
use super::local_service_parser::{ServiceCommand, ServicePeerCommand};
use crate::p2p::Libp2pControlPlaneConfig;

pub(super) fn describe_service_command(command: &ServiceCommand) -> String {
    match command {
        ServiceCommand::Init(args) => {
            format!("initialize service node store data_dir={}", args.data_dir)
        }
        ServiceCommand::Peer {
            command: ServicePeerCommand::Add(args),
        } => format!(
            "add libp2p bootstrap peer data_dir={} peer_id={} address={}",
            args.data_dir, args.peer_id, args.address
        ),
        ServiceCommand::Readiness(args) => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_description(args.identity_seed);
            format!(
                "check mandatory libp2p service readiness p2p_listen={} data_dir={}{} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                args.p2p_listen,
                args.data_dir,
                identity,
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
        }
        ServiceCommand::Serve(args) => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_description(args.identity_seed);
            format!(
                "serve RPC explorer faucet telemetry over mandatory libp2p listen={} p2p_listen={} data_dir={}{} max_requests={} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                args.listen,
                args.p2p_listen,
                args.data_dir,
                identity,
                args.max_requests,
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
        }
        ServiceCommand::Status(args) => {
            format!("show service node store status data_dir={}", args.data_dir)
        }
        ServiceCommand::Block(args) => format!(
            "show service node store block data_dir={} height={}",
            args.data_dir, args.height
        ),
    }
}
