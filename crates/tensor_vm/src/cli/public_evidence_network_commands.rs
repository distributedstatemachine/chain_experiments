use super::public_evidence_observation_commands::ObservationTimestampArgs;
use super::public_evidence_operator_commands::OperatorIdArgs;
use clap::{Args, Subcommand, ValueHint};
use libp2p::{Multiaddr, PeerId};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub(crate) enum EvidenceNetworkCommand {
    #[command(about = "Generate public libp2p network runtime evidence.")]
    Observation(NetworkObservationArgs),
    #[command(about = "Generate public libp2p network runtime evidence from a service log.")]
    FromServiceLog(NetworkObservationFromServiceLogArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NetworkObservationArgs {
    #[command(flatten)]
    pub(crate) target: NetworkObservationTargetArgs,
    #[arg(long, value_name = "PEER_ID", help = "Observed libp2p peer ID.")]
    pub(crate) peer_id: PeerId,
    #[command(flatten)]
    pub(crate) protocol_counts: NetworkObservationProtocolCountsArgs,
    #[command(flatten)]
    pub(crate) transport_limits: NetworkObservationTransportLimitsArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NetworkObservationProtocolCountsArgs {
    #[arg(
        long = "gossip-topics",
        value_name = "N",
        help = "Number of active gossipsub topics."
    )]
    pub(crate) gossip_topic_count: u64,
    #[arg(
        long = "request-response-protocols",
        value_name = "N",
        help = "Number of request-response protocols."
    )]
    pub(crate) request_response_protocol_count: u64,
    #[arg(
        long = "bootstrap-peers",
        value_name = "N",
        help = "Bootstrap peers configured by the node."
    )]
    pub(crate) bootstrap_peer_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NetworkObservationTransportLimitsArgs {
    #[arg(
        long,
        value_name = "BYTES",
        help = "Maximum request-response transmit size."
    )]
    pub(crate) max_transmit_bytes: u64,
    #[arg(long, value_name = "SECONDS", help = "Request-response timeout.")]
    pub(crate) request_timeout_seconds: u64,
    #[arg(long, value_name = "N", help = "Maximum concurrent libp2p streams.")]
    pub(crate) max_concurrent_streams: u64,
    #[arg(long, value_name = "SECONDS", help = "Idle connection timeout.")]
    pub(crate) idle_timeout_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NetworkObservationFromServiceLogArgs {
    #[command(flatten)]
    pub(crate) target: NetworkObservationTargetArgs,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "Captured node service log."
    )]
    pub(crate) service_log: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NetworkObservationTargetArgs {
    #[command(flatten)]
    pub(crate) operator: OperatorIdArgs,
    #[arg(
        long,
        value_name = "MULTIADDR",
        help = "Public libp2p listen multiaddress."
    )]
    pub(crate) listen_address: Multiaddr,
    #[command(flatten)]
    pub(crate) observation: ObservationTimestampArgs,
}
