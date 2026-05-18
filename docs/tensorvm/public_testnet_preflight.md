# TensorVM Public Testnet Preflight

Status: local launch-readiness check only; this is not public-testnet evidence.

The preflight manifest records whether a proposed public TensorVM run has the local shape and deployment
plan needed before starting the external 7-day testnet. Passing preflight does not prove the run happened.
The post-run evidence bundle in [`public_testnet_evidence.md`](public_testnet_evidence.md) is still required
before the MVP can be called fully complete.

## Checked Gates

The local preflight report checks:

- at least 10 planned miners and 5 planned validators under the default public criteria
- positive miner and validator stakes
- funded faucet configuration
- available CUDA kernels for the claimed GPU mining path
- production libp2p runtime plan with discovery, gossip, request/response, and DoS controls
- public HTTPS RPC, explorer, faucet, and telemetry service plans
- service endpoint identifiers, health paths, auth, and rate limiting

## Manifest Format

The parser is `parse_public_testnet_preflight_manifest`. Blank lines and `#` comments are ignored. Hash
values are 64-character hex strings with an optional `0x` prefix. Boolean values are `true` or `false`.

```text
version=tensor-vm-public-testnet-preflight-v1
miner_count=10
validator_count=5
miner_stake=100
validator_stake=10000
faucet_balance=1000000
faucet_drip=100
cuda_kernels_available=true
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
service=rpc,<endpoint-id-hex>,https://rpc.example.test/health,/health,true,true
service=explorer,<endpoint-id-hex>,https://explorer.example.test/health,/health,true,true
service=faucet,<endpoint-id-hex>,https://faucet.example.test/health,/health,true,true
service=telemetry,<endpoint-id-hex>,https://telemetry.example.test/health,/health,true,true
```

The CLI reads a manifest file and reports launch readiness:

```bash
tvmd public-testnet preflight --manifest docs/tensorvm/public-testnet.preflight
```
