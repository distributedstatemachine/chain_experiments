use super::CliCommand;
use super::local_parser::{
    LocalCpuCommand, LocalTestnetCommand, MinerCommand, ProposerCommand, ServiceCommand,
    ServicePeerCommand, ValidatorCommand,
};
use crate::hash::hex;
use crate::p2p::Libp2pControlPlaneConfig;
use crate::types::Hash;

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

fn describe_miner_command(command: &MinerCommand) -> String {
    match command {
        MinerCommand::Register(args) => format!("register miner with stake {}", args.stake),
        MinerCommand::Start(args) => format!(
            "start miner wallet={} device={} node={}",
            args.wallet, args.device, args.node
        ),
        MinerCommand::Run(args) => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_description(args.identity_seed);
            format!(
                "run miner role wallet={} device={} node={} listen={} p2p_listen={} data_dir={}{} max_requests={} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                args.wallet,
                args.device,
                args.node,
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
        MinerCommand::Status => "show miner status".to_owned(),
    }
}

fn describe_validator_command(command: &ValidatorCommand) -> String {
    match command {
        ValidatorCommand::Register(args) => {
            format!("register validator with stake {}", args.stake)
        }
        ValidatorCommand::Start(args) => {
            format!("start validator wallet={} node={}", args.wallet, args.node)
        }
        ValidatorCommand::Run(args) => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_description(args.identity_seed);
            format!(
                "run validator role wallet={} node={} listen={} p2p_listen={} data_dir={}{} max_requests={} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                args.wallet,
                args.node,
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
        ValidatorCommand::Status => "show validator status".to_owned(),
    }
}

fn describe_proposer_command(command: &ProposerCommand) -> String {
    match command {
        ProposerCommand::Run(args) => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_description(args.identity_seed);
            format!(
                "run proposer role wallet={} node={} listen={} p2p_listen={} data_dir={}{} max_requests={} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                args.wallet,
                args.node,
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

fn identity_description(identity_seed: Option<Hash>) -> String {
    identity_seed
        .map(|seed| format!(" identity_seed={}", hex(&seed)))
        .unwrap_or_default()
}
