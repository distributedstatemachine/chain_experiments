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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::hash_bytes;
    use libp2p::Multiaddr;

    #[test]
    fn libp2p_node_builds_real_swarm_and_protocol_behaviour() {
        let config = Libp2pControlPlaneConfig::default();
        let node = build_libp2p_node(&config).unwrap();
        assert!(!node.peer_id.to_string().is_empty());
        assert_eq!(node.subscribed_topics.len(), 5);
        assert!(
            node.subscribed_topics
                .contains(&"/tensorchain/1/blocks".to_owned())
        );
        assert_eq!(node.request_response_protocols.len(), 4);
        assert!(
            node.request_response_protocols
                .contains(&"/tensorchain/1/tensor/chunk".to_owned())
        );
        assert!(
            node.request_response_protocols
                .contains(&"/tensorchain/1/tensor/by-root".to_owned())
        );
        assert_eq!(node.identify_protocol, "/tensorchain/1/identify");
    }

    #[test]
    fn libp2p_node_uses_configured_identity_seed() {
        let seed = hash_bytes(b"test", &[b"libp2p-identity-seed"]);
        let peer_a = build_libp2p_node(&Libp2pControlPlaneConfig {
            identity_seed: Some(seed),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap()
        .peer_id;
        let peer_b = build_libp2p_node(&Libp2pControlPlaneConfig {
            identity_seed: Some(seed),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap()
        .peer_id;
        let peer_c = build_libp2p_node(&Libp2pControlPlaneConfig {
            identity_seed: Some(hash_bytes(b"test", &[b"other-libp2p-identity-seed"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap()
        .peer_id;

        assert_eq!(peer_a, peer_b);
        assert_ne!(peer_a, peer_c);
    }

    #[test]
    fn libp2p_node_accepts_listen_and_bootstrap_multiaddrs() {
        let bootstrap_peer = PeerId::random();
        let bootstrap_address = format!("/ip4/127.0.0.1/tcp/4001/p2p/{bootstrap_peer}");
        let (discovered_peer, discovered_address) =
            bootstrap_peer_address(&bootstrap_address.parse().unwrap()).unwrap();
        assert_eq!(discovered_peer, bootstrap_peer);
        assert_eq!(discovered_address.to_string(), "/ip4/127.0.0.1/tcp/4001");
        let plain_address: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        assert_eq!(bootstrap_peer_address(&plain_address), None);
        let config = Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            bootstrap_addresses: vec![bootstrap_address],
            ..Libp2pControlPlaneConfig::default()
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap();
        runtime.block_on(async {
            let node = build_libp2p_node(&config).unwrap();
            assert!(!node.peer_id.to_string().is_empty());
        });
    }
}
