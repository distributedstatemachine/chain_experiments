use super::CliCommand;
use super::validation::{
    ensure_auth_token, ensure_data_dir, ensure_libp2p_multiaddr, ensure_listen_addr,
    ensure_minimum_stake, ensure_node_endpoint, json_escape, miner_device_readiness,
    wallet_address_hex,
};
use crate::chain::ChainParams;
use crate::error::Result;
use crate::hash::hex;
use crate::p2p::{Libp2pControlPlaneConfig, PeerRecord};
use crate::types::Hash;

fn identity_report(identity_seed: Option<Hash>) -> String {
    match identity_seed {
        Some(seed) => format!("p2p_identity_seeded=true\np2p_identity_seed={}", hex(&seed)),
        None => "p2p_identity_seeded=false".to_owned(),
    }
}

pub(super) fn execute_local_cli_command(command: &CliCommand) -> Result<String> {
    let params = ChainParams::default();
    match command {
        CliCommand::MinerRegister { stake } => {
            ensure_minimum_stake(*stake, params.miner_min_stake)?;
            Ok(format!(
                "command=miner_register\nstake={stake}\nmin_stake={}\nstake_sufficient=true",
                params.miner_min_stake
            ))
        }
        CliCommand::MinerStart {
            wallet,
            device,
            node,
        } => {
            let address = wallet_address_hex(wallet)?;
            let device_readiness = miner_device_readiness(device)?;
            ensure_node_endpoint(node)?;
            Ok(format!(
                "command=miner_start\nwallet={wallet}\naddress={address}\ndevice={device}\nnode={node}\n{}\nreference_backend_ready=true",
                device_readiness.report()
            ))
        }
        CliCommand::MinerRun {
            wallet,
            device,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            let address = wallet_address_hex(wallet)?;
            let device_readiness = miner_device_readiness(device)?;
            ensure_node_endpoint(node)?;
            ensure_listen_addr(listen)?;
            ensure_libp2p_multiaddr(p2p_listen)?;
            ensure_data_dir(data_dir)?;
            ensure_auth_token(auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_report(*identity_seed);
            Ok(format!(
                "command=miner_run\nrole=miner\nwallet={wallet}\naddress={address}\ndevice={device}\nnode={node}\nlisten={listen}\np2p_listen={p2p_listen}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{}\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={data_dir}\nauth_enabled=true\nmax_requests={max_requests}\nrole_runtime_ready=true",
                device_readiness.report(),
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            ))
        }
        CliCommand::MinerStatus => Ok(format!(
            "command=miner_status\nmin_stake={}\nreference_backend_ready=true\nstatus_source=rpc_or_node_store_required",
            params.miner_min_stake
        )),
        CliCommand::ValidatorRegister { stake } => {
            ensure_minimum_stake(*stake, params.validator_min_stake)?;
            Ok(format!(
                "command=validator_register\nstake={stake}\nmin_stake={}\nstake_sufficient=true",
                params.validator_min_stake
            ))
        }
        CliCommand::ValidatorStart { wallet, node } => {
            let address = wallet_address_hex(wallet)?;
            ensure_node_endpoint(node)?;
            Ok(format!(
                "command=validator_start\nwallet={wallet}\naddress={address}\nnode={node}\nreference_verifier_ready=true"
            ))
        }
        CliCommand::ValidatorRun {
            wallet,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            let address = wallet_address_hex(wallet)?;
            ensure_node_endpoint(node)?;
            ensure_listen_addr(listen)?;
            ensure_libp2p_multiaddr(p2p_listen)?;
            ensure_data_dir(data_dir)?;
            ensure_auth_token(auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_report(*identity_seed);
            Ok(format!(
                "command=validator_run\nrole=validator\nwallet={wallet}\naddress={address}\nnode={node}\nlisten={listen}\np2p_listen={p2p_listen}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={data_dir}\nauth_enabled=true\nmax_requests={max_requests}\nreference_verifier_ready=true\nrole_runtime_ready=true",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            ))
        }
        CliCommand::ValidatorStatus => Ok(format!(
            "command=validator_status\nmin_stake={}\nreference_verifier_ready=true\nstatus_source=rpc_or_node_store_required",
            params.validator_min_stake
        )),
        CliCommand::ProposerRun {
            wallet,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            let address = wallet_address_hex(wallet)?;
            ensure_node_endpoint(node)?;
            ensure_listen_addr(listen)?;
            ensure_libp2p_multiaddr(p2p_listen)?;
            ensure_data_dir(data_dir)?;
            ensure_auth_token(auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_report(*identity_seed);
            Ok(format!(
                "command=proposer_run\nrole=proposer\nwallet={wallet}\naddress={address}\nnode={node}\nlisten={listen}\np2p_listen={p2p_listen}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={data_dir}\nauth_enabled=true\nmax_requests={max_requests}\nproposer_ready=true\nrole_runtime_ready=true",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            ))
        }
        CliCommand::ServiceInit { data_dir } => {
            ensure_data_dir(data_dir)?;
            Ok(format!(
                "command=service_init\ndata_dir={data_dir}\nnode_store_ready=true"
            ))
        }
        CliCommand::ServicePeerAdd {
            data_dir,
            peer_id,
            address,
        } => {
            ensure_data_dir(data_dir)?;
            let record = PeerRecord::from_strings(peer_id, address)?;
            let peer_id = record.peer_id()?;
            Ok(format!(
                "command=service_peer_add\ndata_dir={data_dir}\npeer_id={peer_id}\naddress={address}\npeer_book_ready=true"
            ))
        }
        CliCommand::ServiceReadiness {
            p2p_listen,
            data_dir,
            identity_seed,
        } => {
            ensure_libp2p_multiaddr(p2p_listen)?;
            ensure_data_dir(data_dir)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_report(*identity_seed);
            Ok(format!(
                "command=service_readiness\np2p_listen={p2p_listen}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={data_dir}\nnode_store_required=true\nlibp2p_ready=true",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            ))
        }
        CliCommand::ServiceServe {
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            ensure_listen_addr(listen)?;
            ensure_libp2p_multiaddr(p2p_listen)?;
            ensure_data_dir(data_dir)?;
            ensure_auth_token(auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_report(*identity_seed);
            Ok(format!(
                "command=service_serve\nlisten={listen}\np2p_listen={p2p_listen}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={data_dir}\nauth_enabled=true\nmax_requests={max_requests}\nrpc_routes=enabled\nexplorer_routes=enabled\nfaucet_routes=enabled\ntelemetry_routes=enabled\nnode_store_required=true",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            ))
        }
        CliCommand::ServiceStatus { data_dir } => {
            ensure_data_dir(data_dir)?;
            Ok(format!(
                "command=service_status\ndata_dir={data_dir}\nstatus_source=node_store"
            ))
        }
        CliCommand::ServiceBlock { data_dir, height } => {
            ensure_data_dir(data_dir)?;
            Ok(format!(
                "command=service_block\ndata_dir={data_dir}\nheight={height}\nstatus_source=node_store"
            ))
        }
        CliCommand::LocalTestnetSeed { data_dir } => {
            ensure_data_dir(data_dir)?;
            Ok(format!(
                "command=local_testnet_seed\ndata_dir={data_dir}\nlocal_cpu_seed_ready=true"
            ))
        }
        CliCommand::LocalCpuVerify { data_dir, json } => {
            ensure_data_dir(data_dir)?;
            if *json {
                Ok(format!(
                    "{{\"command\":\"local_cpu_verify\",\"data_dir\":\"{}\",\"structured_verifier_ready\":true}}",
                    json_escape(data_dir)
                ))
            } else {
                Ok(format!(
                    "command=local_cpu_verify\ndata_dir={data_dir}\nstructured_verifier_ready=true"
                ))
            }
        }
        _ => unreachable!("public evidence commands are handled by cli::execution"),
    }
}
