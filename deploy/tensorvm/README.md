# TensorVM Public Deployment Scaffold

This directory contains operator-facing deployment templates for the external public services required by
the TensorVM MVP spec. These files are not public-testnet evidence by themselves; they are pre-run
artifacts for launching a service that can later produce independently checkable evidence.

## Files

- `env/public-testnet.env.example` - environment file consumed by the systemd unit
- `RUNBOOK.md` - operator runbook for launch, evidence collection, validation, and publication
- `systemd/tensorvm.service` - `tvmd service serve` unit with mandatory libp2p listen configuration
- `nginx/tensorvm.conf` - TLS reverse-proxy template for RPC, explorer, faucet, and telemetry hostnames
- `manifests/public-testnet.preflight.example` - manifest accepted by
  `tvmd public-testnet preflight --manifest <path>` after replacing example IDs and hosts
- `manifests/public-testnet.evidence.example` - structurally valid post-run evidence example accepted by
  `tvmd public-evidence validate --manifest <path>`, but intentionally too short and too small to satisfy
  the default full-spec 7-day public-testnet criteria; it includes the required signed external
  supporting-record artifact locators

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
and configured DoS controls during the external run. Each signed block, finality, libp2p,
data-availability, invalid-work, and reward summary root also needs a signed external artifact locator for
the raw records behind that root.

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

After a run, operators can use the post-run evidence shape with real roots and signatures:

```bash
tvmd public-evidence validate --manifest deploy/tensorvm/manifests/public-testnet.evidence.example
```

The checked example reports `independently_checkable=true` but `public_evidence_full_spec=false` because it
contains only 60 seconds, 10 observed blocks, 2 miners, and 1 validator. The full-spec gate remains closed
until a real 7-day public run publishes the evidence bundle documented in
`docs/tensorvm/public_testnet_evidence.md`.

Use [`RUNBOOK.md`](RUNBOOK.md) for the required external operator flow, including daily evidence
collection, post-run validation, and final publication.
