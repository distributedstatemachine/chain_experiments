# tensor_vm

Reference implementation of the TensorVM (TVM) MVP reviewed in
[`../../docs/tensorvm/mvp_spec.md`](../../docs/tensorvm/mvp_spec.md).

The crate implements the deterministic local/testnet core:

- row-major finite-field tensors
- self-contained field arithmetic, SHA-256 hashing, and oracle RNG primitives
- bounds-checked tensor row/cell access
- Merkle tensor commitments and openings
- deterministic TensorVM operations
- canonical TensorVM program hashing across all MVP operation variants
- full-output and row-sampled Freivalds checks
- TensorOp receipts and validator attestations
- LinearTrainingStep execution and verification
- settled TensorWork proposer selection and reward settlement
- operator identity on miner state with operator-separated replication assignment
- miner hardware-class and GPU utilization telemetry
- deterministic local-chain execution harness for adversarial tests
- rust-libp2p P2P runtime wiring with Gossipsub, Identify, Kademlia discovery, TCP/TLS/Yamux swarm
  construction, JSON request-response protocols, and a service runtime started by `tvmd service serve`
- durable libp2p peer-book storage for bootstrap peer IDs and multiaddrs
- mandatory libp2p networking for consensus propagation and bounded tensor/program fetches
- restartable `NodeStore` persistence for chain snapshots, full chain state, append-only block logs, and peer books
- explorer, telemetry, and local faucet RPC endpoints
- local browser-facing explorer, telemetry, and faucet HTML pages
- executable reference `tvmd` miner/validator CLI validation and readiness reports using libp2p multiaddrs
- `tvmd service init/serve` launch configuration for a NodeStore-backed RPC/explorer/faucet/telemetry process
  with mandatory libp2p listen configuration
- watcher scans for invalid receipts, data withholding, validator misconduct, and settlement blockers
- public-testnet evidence reporting that distinguishes local preflight shape from actual 7-day
  external-operator proof and checks distinct operators, signed heartbeats, finality, and data availability

Run from the workspace root:

```bash
cargo test -p tensor_vm --release
```
