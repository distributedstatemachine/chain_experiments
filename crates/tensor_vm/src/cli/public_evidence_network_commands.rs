use super::public_evidence_observation_commands::ObservationTimestampArgs;
use super::public_evidence_operator_commands::OperatorIdArgs;
use clap::{Args, Subcommand, ValueHint};
use libp2p::{Multiaddr, PeerId};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub enum EvidenceNetworkCommand {
    #[command(about = "Generate public libp2p network runtime evidence.")]
    Observation(NetworkObservationArgs),
    #[command(about = "Generate public libp2p network runtime evidence from a service log.")]
    FromServiceLog(NetworkObservationFromServiceLogArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NetworkObservationArgs {
    #[command(flatten)]
    pub target: NetworkObservationTargetArgs,
    #[arg(long, value_name = "PEER_ID", help = "Observed libp2p peer ID.")]
    pub peer_id: PeerId,
    #[arg(long, value_name = "N", help = "Number of active gossipsub topics.")]
    pub gossip_topics: u64,
    #[arg(long, value_name = "N", help = "Number of request-response protocols.")]
    pub request_response_protocols: u64,
    #[arg(
        long,
        value_name = "N",
        help = "Bootstrap peers configured by the node."
    )]
    pub bootstrap_peers: u64,
    #[arg(
        long,
        value_name = "BYTES",
        help = "Maximum request-response transmit size."
    )]
    pub max_transmit_bytes: u64,
    #[arg(long, value_name = "SECONDS", help = "Request-response timeout.")]
    pub request_timeout_seconds: u64,
    #[arg(long, value_name = "N", help = "Maximum concurrent libp2p streams.")]
    pub max_concurrent_streams: u64,
    #[arg(long, value_name = "SECONDS", help = "Idle connection timeout.")]
    pub idle_timeout_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NetworkObservationFromServiceLogArgs {
    #[command(flatten)]
    pub target: NetworkObservationTargetArgs,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "Captured node service log."
    )]
    pub service_log: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct NetworkObservationTargetArgs {
    #[command(flatten)]
    pub operator: OperatorIdArgs,
    #[arg(
        long,
        value_name = "MULTIADDR",
        help = "Public libp2p listen multiaddress."
    )]
    pub listen_address: Multiaddr,
    #[command(flatten)]
    pub observation: ObservationTimestampArgs,
}

impl NetworkObservationTargetArgs {
    pub fn operator_id(&self) -> crate::types::Hash {
        self.operator.id()
    }

    pub fn listen_address(&self) -> &Multiaddr {
        &self.listen_address
    }
}
