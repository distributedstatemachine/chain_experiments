# TensorVM Public Testnet Evidence

Status: no complete external public-testnet evidence bundle is available yet.

This document is the publication target for the independently checkable evidence bundle required before
the TensorVM MVP can be called fully complete. A complete bundle must be produced from an external public
run, not from the local harness.

For pre-run launch readiness, use [`public_testnet_preflight.md`](public_testnet_preflight.md). A passing
preflight report is not a substitute for this post-run evidence bundle.

## Required Bundle

A complete evidence bundle must include:

- a public `https://`, `ipfs://`, or `ar://` location for the evidence manifest
- manifest signature records
- independent auditor or verifier records
- signed wall-clock run window covering the full 7-day run
- signed miner and validator heartbeat history for the full run
- independent operator identity or attestation records
- signed block-history summary root for the full 7-day run
- signed finality-history summary root for the full 7-day run
- signed production libp2p network-observation summary root
- signed data-availability measurement summary root for checked tensor receipts
- invalid-work submission and rejection evidence
- reward-settlement records for verified TensorWork
- proof that production libp2p was used for peer discovery, gossip, and request/response propagation
- external HTTPS URLs, health paths, and reachability records for deployed RPC, explorer, faucet, and
  telemetry services

A public `https://` evidence URI must use an external host. The local validator rejects localhost, `.local`
names, loopback, unspecified, private, and link-local IP addresses. `ipfs://` and `ar://` publication URIs
must include a non-empty content identifier.

## Current Repository Evidence

The local reference crate exposes typed validation for this future bundle through
`PublicTestnetEvidenceBundle`. The validator intentionally separates:

- `PublicTestnetRunEvidence`, which checks run-level protocol evidence
- `PublicTestnetEvidenceBundle`, which additionally checks publication, signatures, auditors, and
  independently checkable supporting records

The current local reference implementation and docs do not satisfy this bundle requirement. The manifest
validator requires signed node-heartbeat summaries, signed service-health summaries, and an external
publication URI. It verifies a manifest publication signature over the bundle ID, public URI, manifest
signature count, and independent auditor count. It also verifies a signed run-window record over the
manifest bundle ID, start time, end time, and observed block count. It verifies signed supporting-record
roots for block history, finality history, production libp2p observations, and data-availability
measurements, and it derives
`external_operator_evidence` from the manifest's operator identity attestation record count rather than from
a CLI flag. These local checks are still only evidence-format validation until an external run publishes
real records.

## Manifest Format

External evidence can be represented as a line-oriented manifest parsed by
`parse_public_testnet_evidence_manifest`. Blank lines and `#` comments are ignored. Hash values are
64-character hex strings with an optional `0x` prefix. Boolean values are `true` or `false`. The manifest
signature covers the bundle ID, public URI, manifest signature count, and independent auditor count. Block,
finality, network-runtime, and data-availability signatures cover the bundle ID, record-set kind,
record-set root, and record count. The run-window signature covers the bundle ID, Unix start time, Unix end
time, and observed block count. Heartbeat signatures cover the node role, address, operator ID, first/last
observed block, and heartbeat count. Service-health signatures cover the service kind, endpoint ID, public
URL, health path, first/last observed block, reachable observation count, and signed health-check count.
Service URLs must be external HTTPS endpoints; localhost, `.local`, loopback, private, link-local, and
unspecified hosts are rejected.
The reference service process serves `GET /health` for shared-host deployments and scoped
`GET /rpc/health`, `GET /explorer/health`, `GET /faucet/health`, and `GET /telemetry/health` endpoints
when operators publish distinct public service hostnames or paths.

```text
version=tensor-vm-public-testnet-evidence-v1
bundle_id=0x<64-hex>
public_uri=https://example.test/tensorvm/public-evidence.json
manifest_signer=<address-hex>
manifest_signature=<signature-hex>
manifest_signature_count=1
independent_auditor_count=1
block_history_records=100800
block_history_root=<history-root-hex>
block_history_signature=<history-signature-hex>
finality_history_records=100800
finality_history_root=<finality-root-hex>
finality_history_signature=<finality-signature-hex>
operator_identity_attestation_records=15
network_runtime_observation_records=4
network_runtime_observation_root=<network-runtime-root-hex>
network_runtime_observation_signature=<network-runtime-signature-hex>
data_availability_measurement_records=1000
data_availability_measurement_root=<da-root-hex>
data_availability_measurement_signature=<da-signature-hex>
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
run_started_at_unix_seconds=<unix-seconds>
run_ended_at_unix_seconds=<unix-seconds-plus-at-least-604800>
run_window_signature=<window-signature-hex>
observed_blocks=100800
finalized_blocks=100800
checked_receipts=1000
available_receipts=1000
invalid_receipts_submitted=1
invalid_receipts_rejected=1
reward_settlement_records=1
node=miner,<address-hex>,<operator-id-hex>,0,100799,<heartbeat-count>,<heartbeat-signature-hex>
node=validator,<address-hex>,<operator-id-hex>,0,100799,<heartbeat-count>,<heartbeat-signature-hex>
service=rpc,<endpoint-id-hex>,https://rpc.example.test/health,/health,0,100799,<reachable-count>,<signed-health-check-count>,<health-signature-hex>
service=explorer,<endpoint-id-hex>,https://explorer.example.test/health,/health,0,100799,<reachable-count>,<signed-health-check-count>,<health-signature-hex>
service=faucet,<endpoint-id-hex>,https://faucet.example.test/health,/health,0,100799,<reachable-count>,<signed-health-check-count>,<health-signature-hex>
service=telemetry,<endpoint-id-hex>,https://telemetry.example.test/health,/health,0,100799,<reachable-count>,<signed-health-check-count>,<health-signature-hex>
```

The CLI reads a manifest file and reports the default full-spec evidence status:

```bash
tvmd public-evidence validate --manifest docs/tensorvm/public-testnet.evidence
```

Operators can generate the signed publication, run-window, and node-heartbeat manifest fields:

```bash
tvmd public-evidence publication \
  --bundle-id <bundle-id-hex> \
  --public-uri https://example.test/tensorvm/public-evidence.json \
  --manifest-signer <manifest-signer-address-hex> \
  --manifest-signature-count 1 \
  --independent-auditor-count 1

tvmd public-evidence run-window \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --started-at <unix-seconds> \
  --ended-at <unix-seconds-plus-at-least-604800> \
  --observed-blocks 100800

tvmd public-evidence node-heartbeat \
  --role miner \
  --address <node-address-hex> \
  --operator-id <operator-id-hex> \
  --first-block 0 \
  --last-block 100799 \
  --heartbeat-count 100800
```

The publication command rejects local/private evidence URIs, zero bundle IDs, zero manifest signers, and
zero signature or auditor counts. The run-window command rejects zero IDs/signers, inverted time windows,
and empty block counts. The node-heartbeat command rejects zero node addresses, zero operator IDs,
inverted block ranges, and unsigned heartbeat summaries.

Operators can generate a signed service-health manifest line for RPC, explorer, faucet, or telemetry
evidence:

```bash
tvmd public-evidence service-health \
  --kind rpc \
  --endpoint-id <endpoint-id-hex> \
  --public-url https://rpc.example.test/health \
  --health-path /health \
  --first-block 0 \
  --last-block 100799 \
  --reachable-count 100800 \
  --signed-health-check-count 100800
```

The command rejects local/private service URLs, malformed endpoint IDs, invalid block ranges, and unsigned
or unreachable service-health summaries. Its output can be inserted directly as a `service=...` line in
the evidence manifest.

Operators can also generate signed supporting-record summary lines, including the production libp2p
network-observation summary required by full-spec evidence:

```bash
tvmd public-evidence record-summary \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --record-root <network-runtime-root-hex> \
  --record-count 4
```

Supported record kinds are `block-history`, `finality-history`, `network-runtime`, and
`data-availability`. The command emits the corresponding `<record>_records`, `<record>_root`, and
`<record>_signature` manifest fields using the same signature domain the validator checks.

The output is a line-oriented evidence report. `public_evidence_full_spec=true` requires both
`public_criterion=true` and `independently_checkable=true`. The `external_operator_evidence` field is true
only when enough signed node evidence and operator identity attestation records are present. The individual
fields identify which post-run artifact or protocol observation is missing:

```text
public_evidence_full_spec=false
public_criterion=false
independently_checkable=true
published_evidence_bundle=true
signed_run_window=true
block_history=true
finality_history=true
operator_identity_attestations=true
network_runtime_observations=true
data_availability_measurements=true
miners=2
validators=1
run_started_at_unix_seconds=1700000000
run_ended_at_unix_seconds=1700000060
observed_duration_seconds=60
required_duration_seconds=604800
observed_blocks=10
required_blocks=100800
finality_rate_bps=10000
data_availability_bps=9500
invalid_receipts_submitted=1
invalid_receipts_rejected=1
invalid_work_rejection_rate_bps=10000
reward_settlement_records=1
external_operator_evidence=true
required_miners=false
required_validators=false
required_run_duration=false
required_block_count=false
required_finality=true
required_data_availability=true
invalid_work_rejection_evidence=true
reward_settlement_evidence=true
production_libp2p_runtime=true
deployed_rpc_service=true
deployed_explorer_service=true
deployed_faucet_service=true
deployed_telemetry_service=true
deployed_public_services=true
```
