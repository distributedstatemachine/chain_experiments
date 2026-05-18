# TensorVM Networking Choice

## Recommendation

Use **libp2p as the mandatory MVP networking stack**.

The TensorVM MVP needs a blockchain control plane before it needs optimized blob movement:

- gossip for `NewBlock`, `NewJob`, `NewReceipt`, `NewAttestation`, and `PeerInfo`
- peer discovery/bootstrap for independent nodes
- request/response protocols for tensor rows, tensor chunks, and program bytes
- bounded message sizes, connection timeouts, stream limits, and DoS policy hooks around consensus messages

That maps directly to libp2p's Gossipsub, Identify, Kademlia, bootstrap dialing, and request-response
model. The current reference crate therefore uses rust-libp2p as a non-optional P2P runtime dependency:
it builds a TCP/TLS/Yamux swarm, subscribes to TensorVM Gossipsub topics, installs Identify and Kademlia,
exposes JSON request-response protocols for tensor/program fetches, and persists libp2p bootstrap peer
records. The disabled upstream `default-features` setting in `Cargo.toml` only narrows rust-libp2p to the
explicit protocols TensorVM uses; it is not a TensorVM feature gate.

## Practical Path

1. Keep one production network runtime: libp2p.
2. Implement Gossipsub for block/job/receipt/attestation/peer announcements.
3. Implement Kademlia-backed bootstrap/discovery for independent nodes.
4. Implement request-response for tensor rows/chunks and program fetches.
5. Keep tensor payloads bounded for v0 with connection and stream limits in the libp2p runtime config.

Primary references:

- libp2p docs: <https://libp2p.io/docs/>
- libp2p pubsub docs: <https://libp2p.io/docs/pubsub/>
- rust-libp2p Gossipsub docs: <https://docs.rs/libp2p/latest/libp2p/gossipsub/>
