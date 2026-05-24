use super::commands::{MinerCommand, ProposerCommand, ValidatorCommand, ValidatorRunArgs};
use super::validation::{
    ensure_auth_token, ensure_data_dir, ensure_minimum_stake, miner_device_readiness,
    path_argument, wallet_address_hex,
};
use crate::app::p2p_identity_report;
use crate::chain::ChainParams;
use crate::error::Result;
use crate::p2p::Libp2pControlPlaneConfig;

pub(super) fn execute_miner_command(
    command: &MinerCommand,
    params: &ChainParams,
) -> Result<String> {
    match command {
        MinerCommand::Register(args) => {
            ensure_minimum_stake(args.stake, params.miner_min_stake)?;
            Ok(format!(
                "command=miner_register\nstake={}\nmin_stake={}\nstake_sufficient=true",
                args.stake, params.miner_min_stake
            ))
        }
        MinerCommand::Check(args) => {
            let address = wallet_address_hex(&args.wallet)?;
            let device_readiness = miner_device_readiness(&args.device)?;
            let wallet = path_argument(&args.wallet);
            Ok(format!(
                "command=miner_start\nwallet={}\naddress={address}\ndevice={}\nnode={}\n{}\nreference_backend_ready=true",
                wallet,
                args.device,
                args.node,
                device_readiness.report()
            ))
        }
        MinerCommand::Run(args) => {
            let runtime = &args.runtime;
            let node_runtime = &runtime.node_runtime;
            let address = wallet_address_hex(&args.wallet)?;
            let device_readiness = miner_device_readiness(&args.device)?;
            ensure_data_dir(&node_runtime.data_dir)?;
            ensure_auth_token(&node_runtime.auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity =
                p2p_identity_report(node_runtime.identity_seed.map(|seed| seed.into_hash()));
            let wallet = path_argument(&args.wallet);
            let data_dir = path_argument(&node_runtime.data_dir);
            Ok(format!(
                "command=miner_run\nrole=miner\nwallet={}\naddress={address}\ndevice={}\nnode={}\nlisten={}\np2p_listen={}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{}\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={}\nauth_enabled=true\nmax_requests={}\nrole_runtime_ready=true",
                wallet,
                args.device,
                runtime.node,
                node_runtime.listen,
                node_runtime.p2p_listen,
                device_readiness.report(),
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds,
                data_dir,
                node_runtime.max_requests
            ))
        }
        MinerCommand::Status => Ok(format!(
            "command=miner_status\nmin_stake={}\nreference_backend_ready=true\nstatus_source=rpc_or_node_store_required",
            params.miner_min_stake
        )),
    }
}

pub(super) fn execute_validator_command(
    command: &ValidatorCommand,
    params: &ChainParams,
) -> Result<String> {
    match command {
        ValidatorCommand::Register(args) => {
            ensure_minimum_stake(args.stake, params.validator_min_stake)?;
            Ok(format!(
                "command=validator_register\nstake={}\nmin_stake={}\nstake_sufficient=true",
                args.stake, params.validator_min_stake
            ))
        }
        ValidatorCommand::Check(args) => {
            let address = wallet_address_hex(&args.wallet)?;
            let wallet = path_argument(&args.wallet);
            Ok(format!(
                "command=validator_start\nwallet={}\naddress={address}\nnode={}\nreference_verifier_ready=true",
                wallet, args.node
            ))
        }
        ValidatorCommand::Run(args) => execute_validator_run("validator", args),
        ValidatorCommand::Status => Ok(format!(
            "command=validator_status\nmin_stake={}\nreference_verifier_ready=true\nstatus_source=rpc_or_node_store_required",
            params.validator_min_stake
        )),
    }
}

pub(super) fn execute_proposer_command(command: &ProposerCommand) -> Result<String> {
    match command {
        ProposerCommand::Run(args) => {
            let runtime = &args.runtime;
            let node_runtime = &runtime.node_runtime;
            let address = wallet_address_hex(&args.wallet)?;
            ensure_data_dir(&node_runtime.data_dir)?;
            ensure_auth_token(&node_runtime.auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity =
                p2p_identity_report(node_runtime.identity_seed.map(|seed| seed.into_hash()));
            let wallet = path_argument(&args.wallet);
            let data_dir = path_argument(&node_runtime.data_dir);
            Ok(format!(
                "command=proposer_run\nrole=proposer\nwallet={}\naddress={address}\nnode={}\nlisten={}\np2p_listen={}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={}\nauth_enabled=true\nmax_requests={}\nproposer_ready=true\nrole_runtime_ready=true",
                wallet,
                runtime.node,
                node_runtime.listen,
                node_runtime.p2p_listen,
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds,
                data_dir,
                node_runtime.max_requests
            ))
        }
    }
}

fn execute_validator_run(role: &str, args: &ValidatorRunArgs) -> Result<String> {
    let runtime = &args.runtime;
    let node_runtime = &runtime.node_runtime;
    let address = wallet_address_hex(&args.wallet)?;
    ensure_data_dir(&node_runtime.data_dir)?;
    ensure_auth_token(&node_runtime.auth_token)?;
    let p2p_config = Libp2pControlPlaneConfig::default();
    let identity = p2p_identity_report(node_runtime.identity_seed.map(|seed| seed.into_hash()));
    let wallet = path_argument(&args.wallet);
    let data_dir = path_argument(&node_runtime.data_dir);
    Ok(format!(
        "command={role}_run\nrole={role}\nwallet={}\naddress={address}\nnode={}\nlisten={}\np2p_listen={}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={}\nauth_enabled=true\nmax_requests={}\nreference_verifier_ready=true\nrole_runtime_ready=true",
        wallet,
        runtime.node,
        node_runtime.listen,
        node_runtime.p2p_listen,
        p2p_config.max_gossipsub_transmit_bytes,
        p2p_config.request_timeout_seconds,
        p2p_config.max_concurrent_request_streams,
        p2p_config.idle_connection_timeout_seconds,
        data_dir,
        node_runtime.max_requests
    ))
}
