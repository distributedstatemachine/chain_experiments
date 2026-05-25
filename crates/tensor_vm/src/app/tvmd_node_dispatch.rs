use crate::cli::{LocalnetCommand, NodeCommand, NodePeerCommand};

use super::operator_validation::{validate_data_dir, validate_service_runtime};
use super::tvmd_path::path_arg;
use super::{
    add_service_peer, check_service_readiness, init_service_store, seed_local_testnet,
    serve_service, service_block_status, service_status, verify_local_cpu_store,
};

pub(super) fn execute_node_command(command: &NodeCommand) -> std::result::Result<String, String> {
    match command {
        NodeCommand::Init(args) => {
            let data_dir = path_arg(args.path());
            validate_data_dir(&data_dir)?;
            init_service_store(&data_dir)
        }
        NodeCommand::Peer(NodePeerCommand::Add(args)) => {
            let data_dir = path_arg(args.data_dir.path());
            validate_data_dir(&data_dir)?;
            add_service_peer(
                &data_dir,
                &args.bootstrap_peer.peer_id().to_string(),
                &args.bootstrap_peer.address().to_string(),
            )
        }
        NodeCommand::Check(args) => {
            let data_dir = path_arg(args.data_dir.path());
            validate_data_dir(&data_dir)?;
            check_service_readiness(
                &args.p2p_listen.multiaddr().to_string(),
                &data_dir,
                args.identity_seed.hash(),
            )
        }
        NodeCommand::Serve(args) => {
            let runtime = &args.runtime;
            let listen = runtime.listen.to_string();
            let p2p_listen = runtime.p2p_listen.multiaddr().to_string();
            let data_dir = path_arg(runtime.data_dir.path());
            validate_service_runtime(&data_dir, &runtime.auth_token)?;
            serve_service(
                &listen,
                &p2p_listen,
                &data_dir,
                runtime.identity_seed.hash(),
                &runtime.auth_token,
                runtime.max_requests,
            )
        }
        NodeCommand::Status(args) => {
            let data_dir = path_arg(args.path());
            validate_data_dir(&data_dir)?;
            service_status(&data_dir)
        }
        NodeCommand::Block(args) => {
            let data_dir = path_arg(args.data_dir.path());
            validate_data_dir(&data_dir)?;
            service_block_status(&data_dir, args.height)
        }
    }
}

pub(super) fn execute_localnet_command(
    command: &LocalnetCommand,
) -> std::result::Result<String, String> {
    match command {
        LocalnetCommand::Seed(args) => {
            let data_dir = path_arg(args.path());
            validate_data_dir(&data_dir)?;
            seed_local_testnet(&data_dir)
        }
        LocalnetCommand::Verify(args) => {
            let data_dir = path_arg(args.data_dir.path());
            validate_data_dir(&data_dir)?;
            verify_local_cpu_store(&data_dir, args.json)
        }
    }
}
