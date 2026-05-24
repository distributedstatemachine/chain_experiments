use super::CliCommand;
use super::local_description_values::identity_description;
use super::local_parser::{
    LocalCpuCommand, LocalTestnetCommand, ServiceCommand, ServicePeerCommand,
};
use super::local_role_descriptions::{
    describe_miner_command, describe_proposer_command, describe_validator_command,
};
use crate::p2p::Libp2pControlPlaneConfig;

pub(super) fn describe_local_command(command: &CliCommand) -> String {
    match command {
        CliCommand::Miner { command } => describe_miner_command(command),
        CliCommand::Validator { command } => describe_validator_command(command),
        CliCommand::Proposer { command } => describe_proposer_command(command),
        CliCommand::Service { command } => describe_service_command(command),
        CliCommand::LocalTestnet { command } => describe_local_testnet_command(command),
        CliCommand::LocalCpu { command } => describe_local_cpu_command(command),
        _ => unreachable!(
            "public evidence commands are handled by cli::public_evidence_descriptions"
        ),
    }
}

fn describe_service_command(command: &ServiceCommand) -> String {
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

fn describe_local_testnet_command(command: &LocalTestnetCommand) -> String {
    match command {
        LocalTestnetCommand::Seed(args) => {
            format!("seed local CPU testnet data_dir={}", args.data_dir)
        }
    }
}

fn describe_local_cpu_command(command: &LocalCpuCommand) -> String {
    match command {
        LocalCpuCommand::Verify(args) => format!(
            "verify local CPU node evidence data_dir={} json={}",
            args.data_dir, args.json
        ),
    }
}
