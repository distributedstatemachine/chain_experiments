use super::CliCommand;
use crate::hash::hex;
use crate::p2p::Libp2pControlPlaneConfig;
use crate::types::Hash;

pub(super) fn describe_local_command(command: &CliCommand) -> String {
    match command {
        CliCommand::MinerRegister { stake } => format!("register miner with stake {stake}"),
        CliCommand::MinerStart {
            wallet,
            device,
            node,
        } => format!("start miner wallet={wallet} device={device} node={node}"),
        CliCommand::MinerRun {
            wallet,
            device,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token: _,
            max_requests,
        } => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_description(*identity_seed);
            format!(
                "run miner role wallet={wallet} device={device} node={node} listen={listen} p2p_listen={p2p_listen} data_dir={data_dir}{identity} max_requests={max_requests} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
        }
        CliCommand::MinerStatus => "show miner status".to_owned(),
        CliCommand::ValidatorRegister { stake } => format!("register validator with stake {stake}"),
        CliCommand::ValidatorStart { wallet, node } => {
            format!("start validator wallet={wallet} node={node}")
        }
        CliCommand::ValidatorRun {
            wallet,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token: _,
            max_requests,
        } => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_description(*identity_seed);
            format!(
                "run validator role wallet={wallet} node={node} listen={listen} p2p_listen={p2p_listen} data_dir={data_dir}{identity} max_requests={max_requests} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
        }
        CliCommand::ValidatorStatus => "show validator status".to_owned(),
        CliCommand::ProposerRun {
            wallet,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token: _,
            max_requests,
        } => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_description(*identity_seed);
            format!(
                "run proposer role wallet={wallet} node={node} listen={listen} p2p_listen={p2p_listen} data_dir={data_dir}{identity} max_requests={max_requests} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
        }
        CliCommand::ServiceInit { data_dir } => {
            format!("initialize service node store data_dir={data_dir}")
        }
        CliCommand::ServicePeerAdd {
            data_dir,
            peer_id,
            address,
        } => {
            format!(
                "add libp2p bootstrap peer data_dir={data_dir} peer_id={peer_id} address={address}"
            )
        }
        CliCommand::ServiceReadiness {
            p2p_listen,
            data_dir,
            identity_seed,
        } => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_description(*identity_seed);
            format!(
                "check mandatory libp2p service readiness p2p_listen={p2p_listen} data_dir={data_dir}{identity} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
        }
        CliCommand::ServiceServe {
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token: _,
            max_requests,
        } => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_description(*identity_seed);
            format!(
                "serve RPC explorer faucet telemetry over mandatory libp2p listen={listen} p2p_listen={p2p_listen} data_dir={data_dir}{identity} max_requests={max_requests} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
        }
        CliCommand::ServiceStatus { data_dir } => {
            format!("show service node store status data_dir={data_dir}")
        }
        CliCommand::ServiceBlock { data_dir, height } => {
            format!("show service node store block data_dir={data_dir} height={height}")
        }
        CliCommand::LocalTestnetSeed { data_dir } => {
            format!("seed local CPU testnet data_dir={data_dir}")
        }
        CliCommand::LocalCpuVerify { data_dir, json } => {
            format!("verify local CPU node evidence data_dir={data_dir} json={json}")
        }
        _ => unreachable!(
            "public evidence commands are handled by cli::public_evidence_descriptions"
        ),
    }
}

fn identity_description(identity_seed: Option<Hash>) -> String {
    identity_seed
        .map(|seed| format!(" identity_seed={}", hex(&seed)))
        .unwrap_or_default()
}
