use crate::error::{Result as TvmResult, TvmError};
use libp2p::{PeerId, Swarm};
use std::time::Duration;

use super::behaviour::{TensorVmNetworkBehaviour, build_libp2p_behaviour};
use super::peer_book::{bootstrap_peer_address, parse_multiaddr};
use super::{LIBP2P_PROTOCOL_PREFIX, Libp2pControlPlaneConfig};

pub struct TensorVmLibp2pNode {
    pub peer_id: PeerId,
    pub swarm: Swarm<TensorVmNetworkBehaviour>,
    pub identify_protocol: String,
    pub subscribed_topics: Vec<String>,
    pub request_response_protocols: Vec<String>,
}

pub fn build_libp2p_node(config: &Libp2pControlPlaneConfig) -> TvmResult<TensorVmLibp2pNode> {
    let keypair = match config.identity_seed {
        Some(seed) => libp2p::identity::Keypair::ed25519_from_bytes(seed)
            .map_err(|_| TvmError::InvalidReceipt("libp2p identity seed rejected"))?,
        None => libp2p::identity::Keypair::generate_ed25519(),
    };
    let peer_id = PeerId::from(keypair.public());
    let behaviour = build_libp2p_behaviour(config, &keypair)?;
    let identify_protocol = format!("{LIBP2P_PROTOCOL_PREFIX}/identify");
    let subscribed_topics = config
        .gossipsub_topics
        .iter()
        .map(|topic| topic.as_str().to_owned())
        .collect();
    let request_response_protocols = config
        .request_response_protocols
        .iter()
        .map(|protocol| protocol.as_str().to_owned())
        .collect();
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::tls::Config::new,
            libp2p::yamux::Config::default,
        )
        .map_err(|_| TvmError::InvalidReceipt("libp2p transport build failed"))?
        .with_dns()
        .map_err(|_| TvmError::InvalidReceipt("libp2p dns transport build failed"))?
        .with_behaviour(|_| behaviour)
        .map_err(|_| TvmError::InvalidReceipt("libp2p behaviour build failed"))?
        .with_swarm_config(|swarm_config| {
            swarm_config.with_idle_connection_timeout(Duration::from_secs(
                config.idle_connection_timeout_seconds,
            ))
        })
        .build();

    for address in &config.listen_addresses {
        swarm
            .listen_on(parse_multiaddr(address)?)
            .map_err(|_| TvmError::InvalidReceipt("libp2p listen address rejected"))?;
    }
    for address in &config.bootstrap_addresses {
        let multiaddr = parse_multiaddr(address)?;
        if let Some((peer_id, peer_address)) = bootstrap_peer_address(&multiaddr) {
            swarm
                .behaviour_mut()
                .kademlia
                .add_address(&peer_id, peer_address);
        }
        swarm
            .dial(multiaddr)
            .map_err(|_| TvmError::InvalidReceipt("libp2p bootstrap address rejected"))?;
    }

    Ok(TensorVmLibp2pNode {
        peer_id,
        swarm,
        identify_protocol,
        subscribed_topics,
        request_response_protocols,
    })
}
