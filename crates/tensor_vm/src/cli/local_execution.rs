use super::CliCommand;
use super::local_execution_values::identity_report;
use super::local_parser::{
    LocalCpuCommand, LocalTestnetCommand, ServiceCommand, ServicePeerCommand,
};
use super::local_role_execution::{
    execute_miner_command, execute_proposer_command, execute_validator_command,
};
use super::validation::{
    ensure_auth_token, ensure_data_dir, ensure_libp2p_multiaddr, ensure_listen_addr, json_escape,
};
use crate::chain::ChainParams;
use crate::error::Result;
use crate::p2p::{Libp2pControlPlaneConfig, PeerRecord};

pub(super) fn execute_local_cli_command(command: &CliCommand) -> Result<String> {
    let params = ChainParams::default();
    match command {
        CliCommand::Miner { command } => execute_miner_command(command, &params),
        CliCommand::Validator { command } => execute_validator_command(command, &params),
        CliCommand::Proposer { command } => execute_proposer_command(command),
        CliCommand::Service { command } => execute_service_command(command),
        CliCommand::LocalTestnet { command } => execute_local_testnet_command(command),
        CliCommand::LocalCpu { command } => execute_local_cpu_command(command),
        _ => unreachable!("public evidence commands are handled by cli::execution"),
    }
}

fn execute_service_command(command: &ServiceCommand) -> Result<String> {
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

fn execute_local_testnet_command(command: &LocalTestnetCommand) -> Result<String> {
    match command {
        LocalTestnetCommand::Seed(args) => {
            ensure_data_dir(&args.data_dir)?;
            Ok(format!(
                "command=local_testnet_seed\ndata_dir={}\nlocal_cpu_seed_ready=true",
                args.data_dir
            ))
        }
    }
}

fn execute_local_cpu_command(command: &LocalCpuCommand) -> Result<String> {
    match command {
        LocalCpuCommand::Verify(args) => {
            ensure_data_dir(&args.data_dir)?;
            if args.json {
                Ok(format!(
                    "{{\"command\":\"local_cpu_verify\",\"data_dir\":\"{}\",\"structured_verifier_ready\":true}}",
                    json_escape(&args.data_dir)
                ))
            } else {
                Ok(format!(
                    "command=local_cpu_verify\ndata_dir={}\nstructured_verifier_ready=true",
                    args.data_dir
                ))
            }
        }
    }
}
