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
- service endpoint identifiers, health paths, content paths, auth, and rate limiting

Public HTTPS service hosts must be externally reachable names or addresses. The local checker rejects
localhost, `.local` names, loopback, unspecified, private, and link-local IP addresses, including bracketed
IPv6 loopback literals.

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
service=rpc,<endpoint-id-hex>,https://rpc.example.test/health,/health,https://rpc.example.test/chain/head,/chain/head,true,true
service=explorer,<endpoint-id-hex>,https://explorer.example.test/health,/health,https://explorer.example.test/explorer,/explorer,true,true
service=faucet,<endpoint-id-hex>,https://faucet.example.test/health,/health,https://faucet.example.test/faucet/page,/faucet/page,true,true
service=telemetry,<endpoint-id-hex>,https://telemetry.example.test/health,/health,https://telemetry.example.test/telemetry/dashboard,/telemetry/dashboard,true,true
```

Each `service=...` line records the service kind, endpoint ID, public health URL, health path, public
content URL, required content path, auth flag, and rate-limit flag. The health URL path must match the
health path. The content URL path must match the required public surface for that service:
`/chain/head`, `/explorer`, `/faucet/page`, or `/telemetry/dashboard`.

The CLI reads a manifest file and reports launch readiness:

```bash
tvmd public-testnet preflight --manifest docs/tensorvm/public-testnet.preflight
```

A checked deployment scaffold is available under [`../../deploy/tensorvm`](../../deploy/tensorvm):

- `systemd/tensorvm.service` runs the explicit `tvmd` binary target with required `--p2p-listen`
- `env/public-testnet.env.example` records the service listen address, libp2p multiaddr, data directory,
  auth token, and request limit
- `nginx/tensorvm.conf` publishes RPC, explorer, faucet, and telemetry hostnames over external HTTPS
- `manifests/public-testnet.preflight.example` is a concrete preflight manifest shape for replacement by
  real public endpoint IDs and hostnames

The example manifest can be checked from the repository root with:

```bash
cargo run -p tensor_vm --bin tvmd -- public-testnet preflight --manifest deploy/tensorvm/manifests/public-testnet.preflight.example
```

The reference service process can be prepared and launched with:

```bash
tvmd service init --data-dir /var/lib/tensorvm
tvmd service serve --listen 0.0.0.0:8545 --p2p-listen /ip4/0.0.0.0/tcp/4001 --data-dir /var/lib/tensorvm --auth-token service-token --max-requests 0
```

The service exposes `GET /health` plus scoped `GET /rpc/health`, `GET /explorer/health`,
`GET /faucet/health`, and `GET /telemetry/health` endpoints for external monitors. The generic `/health`
path is suitable when each public service hostname routes to the same TensorVM service process. It also
exposes the content surfaces later required by public evidence: `GET /chain/head`, `GET /explorer`,
`GET /faucet/page`, and `GET /telemetry/dashboard`.

The output is a line-oriented readiness report. `public_testnet_preflight_ready=true` only means the
planned run has the required local shape and deployment plan; it still does not prove an external run
has happened. Failed launches can be diagnosed from the individual gate fields:

```text
public_testnet_preflight_ready=true
local_shape_ready=true
deployment_plan_ready=true
miners=10
validators=5
required_blocks=100800
required_miners=true
required_validators=true
positive_stakes=true
funded_faucet=true
cuda_kernels_available=true
production_libp2p_runtime=true
rpc_service_plan=true
explorer_service_plan=true
faucet_service_plan=true
telemetry_service_plan=true
public_service_content_planned=true
public_services_planned=true
```
