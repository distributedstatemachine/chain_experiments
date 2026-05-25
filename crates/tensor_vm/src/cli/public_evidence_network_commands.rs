use super::public_evidence_observation_commands::ObservationTimestampArgs;
use super::public_evidence_operator_commands::OperatorIdArgs;
use crate::types::Hash;
use clap::{Args, Subcommand, ValueHint};
use libp2p::{Multiaddr, PeerId};
use std::path::{Path, PathBuf};

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
    target: NetworkObservationTargetArgs,
    #[arg(long, value_name = "PEER_ID", help = "Observed libp2p peer ID.")]
    peer_id: PeerId,
    #[command(flatten)]
    protocol_counts: NetworkObservationProtocolCountsArgs,
    #[command(flatten)]
    transport_limits: NetworkObservationTransportLimitsArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NetworkObservationProtocolCountsArgs {
    #[arg(
        long = "gossip-topics",
        value_name = "N",
        help = "Number of active gossipsub topics."
    )]
    gossip_topic_count: u64,
    #[arg(
        long = "request-response-protocols",
        value_name = "N",
        help = "Number of request-response protocols."
    )]
    request_response_protocol_count: u64,
    #[arg(
        long = "bootstrap-peers",
        value_name = "N",
        help = "Bootstrap peers configured by the node."
    )]
    bootstrap_peer_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NetworkObservationTransportLimitsArgs {
    #[arg(
        long,
        value_name = "BYTES",
        help = "Maximum request-response transmit size."
    )]
    max_transmit_bytes: u64,
    #[arg(long, value_name = "SECONDS", help = "Request-response timeout.")]
    request_timeout_seconds: u64,
    #[arg(long, value_name = "N", help = "Maximum concurrent libp2p streams.")]
    max_concurrent_streams: u64,
    #[arg(long, value_name = "SECONDS", help = "Idle connection timeout.")]
    idle_timeout_seconds: u64,
}

impl NetworkObservationArgs {
    #[cfg(test)]
    pub(crate) fn new(
        target: NetworkObservationTargetArgs,
        peer_id: PeerId,
        protocol_counts: NetworkObservationProtocolCountsArgs,
        transport_limits: NetworkObservationTransportLimitsArgs,
    ) -> Self {
        Self {
            target,
            peer_id,
            protocol_counts,
            transport_limits,
        }
    }

    pub fn operator_id(&self) -> Hash {
        self.target.operator_id()
    }

    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    pub fn listen_address(&self) -> &Multiaddr {
        self.target.listen_address()
    }

    pub fn observed_at(&self) -> u64 {
        self.target.observed_at()
    }

    pub fn gossip_topic_count(&self) -> u64 {
        self.protocol_counts.gossip_topic_count()
    }

    pub fn request_response_protocol_count(&self) -> u64 {
        self.protocol_counts.request_response_protocol_count()
    }

    pub fn bootstrap_peer_count(&self) -> u64 {
        self.protocol_counts.bootstrap_peer_count()
    }

    pub fn max_transmit_bytes(&self) -> u64 {
        self.transport_limits.max_transmit_bytes()
    }

    pub fn request_timeout_seconds(&self) -> u64 {
        self.transport_limits.request_timeout_seconds()
    }

    pub fn max_concurrent_streams(&self) -> u64 {
        self.transport_limits.max_concurrent_streams()
    }

    pub fn idle_timeout_seconds(&self) -> u64 {
        self.transport_limits.idle_timeout_seconds()
    }
}

impl NetworkObservationProtocolCountsArgs {
    #[cfg(test)]
    pub(crate) fn new(
        gossip_topic_count: u64,
        request_response_protocol_count: u64,
        bootstrap_peer_count: u64,
    ) -> Self {
        Self {
            gossip_topic_count,
            request_response_protocol_count,
            bootstrap_peer_count,
        }
    }

    pub fn gossip_topic_count(&self) -> u64 {
        self.gossip_topic_count
    }

    pub fn request_response_protocol_count(&self) -> u64 {
        self.request_response_protocol_count
    }

    pub fn bootstrap_peer_count(&self) -> u64 {
        self.bootstrap_peer_count
    }
}

impl NetworkObservationTransportLimitsArgs {
    #[cfg(test)]
    pub(crate) fn new(
        max_transmit_bytes: u64,
        request_timeout_seconds: u64,
        max_concurrent_streams: u64,
        idle_timeout_seconds: u64,
    ) -> Self {
        Self {
            max_transmit_bytes,
            request_timeout_seconds,
            max_concurrent_streams,
            idle_timeout_seconds,
        }
    }

    pub fn max_transmit_bytes(&self) -> u64 {
        self.max_transmit_bytes
    }

    pub fn request_timeout_seconds(&self) -> u64 {
        self.request_timeout_seconds
    }

    pub fn max_concurrent_streams(&self) -> u64 {
        self.max_concurrent_streams
    }

    pub fn idle_timeout_seconds(&self) -> u64 {
        self.idle_timeout_seconds
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NetworkObservationFromServiceLogArgs {
    #[command(flatten)]
    target: NetworkObservationTargetArgs,
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "Captured node service log."
    )]
    service_log: PathBuf,
}

impl NetworkObservationFromServiceLogArgs {
    #[cfg(test)]
    pub(crate) fn new(target: NetworkObservationTargetArgs, service_log: PathBuf) -> Self {
        Self {
            target,
            service_log,
        }
    }

    pub fn operator_id(&self) -> Hash {
        self.target.operator_id()
    }

    pub fn listen_address(&self) -> &Multiaddr {
        self.target.listen_address()
    }

    pub fn observed_at(&self) -> u64 {
        self.target.observed_at()
    }

    pub fn service_log(&self) -> &Path {
        &self.service_log
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NetworkObservationTargetArgs {
    #[command(flatten)]
    operator: OperatorIdArgs,
    #[arg(
        long,
        value_name = "MULTIADDR",
        help = "Public libp2p listen multiaddress."
    )]
    listen_address: Multiaddr,
    #[command(flatten)]
    observation: ObservationTimestampArgs,
}

impl NetworkObservationTargetArgs {
    #[cfg(test)]
    pub(crate) fn new(
        operator: OperatorIdArgs,
        listen_address: Multiaddr,
        observation: ObservationTimestampArgs,
    ) -> Self {
        Self {
            operator,
            listen_address,
            observation,
        }
    }

    pub fn operator_id(&self) -> Hash {
        self.operator.operator_id.into_hash()
    }

    pub fn listen_address(&self) -> &Multiaddr {
        &self.listen_address
    }

    pub fn observed_at(&self) -> u64 {
        self.observation.observed_at
    }
}
