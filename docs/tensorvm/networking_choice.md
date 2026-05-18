# TensorVM Networking Choice

## Recommendation

Use **libp2p as the primary MVP networking stack**.

The TensorVM MVP needs a blockchain control plane before it needs optimized blob movement:

- gossip for `NewBlock`, `NewJob`, `NewReceipt`, `NewAttestation`, and `PeerInfo`
- peer discovery/bootstrap for independent nodes
- request/response protocols for tensor rows, tensor chunks, and program bytes
- bounded message sizes, connection timeouts, stream limits, and DoS policy hooks around consensus messages

That maps directly to libp2p's Gossipsub, Identify, Kademlia, bootstrap dialing, and request-response
model. The current reference crate therefore uses rust-libp2p as the default P2P runtime dependency: it
builds a TCP/TLS/Yamux swarm, subscribes to TensorVM Gossipsub topics, installs Identify and Kademlia,
exposes JSON request-response protocols for tensor/program fetches, and persists libp2p bootstrap peer
records.

Iroh is a strong candidate for a later tensor data plane. Its endpoint/blobs/gossip model is useful for
content-addressed, verified blob transfer and direct QUIC connections, but making it the first consensus
network would split the MVP before the block/job/receipt/attestation propagation path is proven.

## Practical Path

1. Keep one production control plane: libp2p.
2. Implement Gossipsub for block/job/receipt/attestation/peer announcements.
3. Implement Kademlia-backed bootstrap/discovery for independent nodes.
4. Implement request-response for tensor rows/chunks and program fetches.
5. Keep tensor payloads bounded for v0 with connection and stream limits in the libp2p runtime config.
6. Add Iroh later only as a specialized content-addressed tensor/blob sidecar if libp2p request-response
   becomes the bottleneck.

Primary references:

- libp2p docs: <https://libp2p.io/docs/>
- libp2p pubsub docs: <https://libp2p.io/docs/pubsub/>
- rust-libp2p Gossipsub docs: <https://docs.rs/libp2p/latest/libp2p/gossipsub/>
- Iroh blobs docs: <https://docs.iroh.computer/protocols/blobs>
- Iroh endpoint docs: <https://docs.rs/iroh/latest/iroh/endpoint/>
