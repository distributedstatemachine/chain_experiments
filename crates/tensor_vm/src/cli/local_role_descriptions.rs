use super::local_description_values::identity_description;
use super::local_role_parser::{MinerCommand, ProposerCommand, ValidatorCommand};
use crate::p2p::Libp2pControlPlaneConfig;

pub(super) fn describe_miner_command(command: &MinerCommand) -> String {
    match command {
        MinerCommand::Register(args) => format!("register miner with stake {}", args.stake),
        MinerCommand::Start(args) => format!(
            "start miner wallet={} device={} node={}",
            args.wallet, args.device, args.node
        ),
        MinerCommand::Run(args) => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let runtime = &args.runtime;
            let service = &runtime.service;
            let identity = identity_description(service.identity_seed);
            format!(
                "run miner role wallet={} device={} node={} listen={} p2p_listen={} data_dir={}{} max_requests={} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                args.wallet,
                args.device,
                runtime.node,
                service.listen,
                service.p2p_listen,
                service.data_dir,
                identity,
                service.max_requests,
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
        }
        MinerCommand::Status => "show miner status".to_owned(),
    }
}

pub(super) fn describe_validator_command(command: &ValidatorCommand) -> String {
    match command {
        ValidatorCommand::Register(args) => {
            format!("register validator with stake {}", args.stake)
        }
        ValidatorCommand::Start(args) => {
            format!("start validator wallet={} node={}", args.wallet, args.node)
        }
        ValidatorCommand::Run(args) => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let runtime = &args.runtime;
            let service = &runtime.service;
            let identity = identity_description(service.identity_seed);
            format!(
                "run validator role wallet={} node={} listen={} p2p_listen={} data_dir={}{} max_requests={} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                args.wallet,
                runtime.node,
                service.listen,
                service.p2p_listen,
                service.data_dir,
                identity,
                service.max_requests,
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
        }
        ValidatorCommand::Status => "show validator status".to_owned(),
    }
}

pub(super) fn describe_proposer_command(command: &ProposerCommand) -> String {
    match command {
        ProposerCommand::Run(args) => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let runtime = &args.runtime;
            let service = &runtime.service;
            let identity = identity_description(service.identity_seed);
            format!(
                "run proposer role wallet={} node={} listen={} p2p_listen={} data_dir={}{} max_requests={} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                args.wallet,
                runtime.node,
                service.listen,
                service.p2p_listen,
                service.data_dir,
                identity,
                service.max_requests,
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
        }
    }
}
