# TensorVM Public Deployment Scaffold

This directory contains operator-facing deployment templates for the external public services required by
the TensorVM MVP spec. These files are not public-testnet evidence by themselves; they are pre-run
artifacts for launching a service that can later produce independently checkable evidence.

## Files

- `env/public-testnet.env.example` - environment file consumed by the systemd unit
- `systemd/tensorvm.service` - `tvmd service serve` unit with mandatory libp2p listen configuration
- `nginx/tensorvm.conf` - TLS reverse-proxy template for RPC, explorer, faucet, and telemetry hostnames
- `manifests/public-testnet.preflight.example` - manifest accepted by
  `tvmd public-testnet preflight --manifest <path>` after replacing example IDs and hosts

## Deployment Shape

The reference service process exposes all required surfaces from one `tvmd service serve` process:

```text
GET /health
GET /rpc/health
GET /explorer/health
GET /faucet/health
GET /telemetry/health
```

The nginx template publishes separate external HTTPS hostnames for the four surfaces and routes each
hostname to the local service. Public evidence still has to include signed service-health records for each
external URL and signed network-observation records proving libp2p discovery, gossip, request/response,
and configured DoS controls during the external run.

## Minimal Operator Flow

```bash
cargo build -p tensor_vm --release --features cuda-kernels
sudo install -m 0755 target/release/tvmd /usr/local/bin/tvmd
sudo useradd --system --home-dir /var/lib/tensorvm --shell /usr/sbin/nologin tensorvm
sudo install -d -o tensorvm -g tensorvm /var/lib/tensorvm
sudo install -d /etc/tensorvm
sudo install -m 0640 deploy/tensorvm/env/public-testnet.env.example /etc/tensorvm/public-testnet.env
sudo install -m 0644 deploy/tensorvm/systemd/tensorvm.service /etc/systemd/system/tensorvm.service
sudo systemctl daemon-reload
sudo systemctl enable --now tensorvm.service
```

Before advertising the run, replace all example hostnames, tokens, and IDs, publish HTTPS with valid TLS,
and run:

```bash
tvmd public-testnet preflight --manifest deploy/tensorvm/manifests/public-testnet.preflight.example
```

The full-spec gate remains closed until a real 7-day public run publishes the evidence bundle documented in
`docs/tensorvm/public_testnet_evidence.md`.
