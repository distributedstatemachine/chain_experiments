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
- signed miner and validator heartbeat history for the full run
- independent operator identity or attestation records
- block history for the full 7-day run
- finality history for the full 7-day run
- data-availability measurements for checked tensor receipts
- invalid-work submission and rejection evidence
- reward-settlement records for verified TensorWork
- proof that production libp2p was used for peer discovery, gossip, and request/response propagation
- reachability records for deployed RPC, explorer, faucet, and telemetry services

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
publication URI. It also derives `external_operator_evidence` from the manifest's operator identity
attestation record count rather than from a CLI flag. These local checks are still only evidence-format
validation until an external run publishes real records.

## Manifest Format

External evidence can be represented as a line-oriented manifest parsed by
`parse_public_testnet_evidence_manifest`. Blank lines and `#` comments are ignored. Hash values are
64-character hex strings with an optional `0x` prefix. Boolean values are `true` or `false`. Heartbeat
signatures cover the node role, address, operator ID, first/last observed block, and heartbeat count.
Service-health signatures cover the service kind, endpoint ID, first/last observed block, reachable
observation count, and signed health-check count.

```text
version=tensor-vm-public-testnet-evidence-v1
bundle_id=0x<64-hex>
public_uri=https://example.test/tensorvm/public-evidence.json
manifest_signature_count=1
independent_auditor_count=1
block_history_records=100800
finality_history_records=100800
operator_identity_attestation_records=15
data_availability_measurement_records=1000
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
observed_blocks=100800
finalized_blocks=100800
checked_receipts=1000
available_receipts=1000
invalid_receipts_submitted=1
invalid_receipts_rejected=1
reward_settlement_records=1
node=miner,<address-hex>,<operator-id-hex>,0,100799,<heartbeat-count>,<heartbeat-signature-hex>
node=validator,<address-hex>,<operator-id-hex>,0,100799,<heartbeat-count>,<heartbeat-signature-hex>
service=rpc,<endpoint-id-hex>,0,100799,<reachable-count>,<signed-health-check-count>,<health-signature-hex>
service=explorer,<endpoint-id-hex>,0,100799,<reachable-count>,<signed-health-check-count>,<health-signature-hex>
service=faucet,<endpoint-id-hex>,0,100799,<reachable-count>,<signed-health-check-count>,<health-signature-hex>
service=telemetry,<endpoint-id-hex>,0,100799,<reachable-count>,<signed-health-check-count>,<health-signature-hex>
```

The CLI reads a manifest file and reports the default full-spec evidence status:

```bash
tvmd public-evidence validate --manifest docs/tensorvm/public-testnet.evidence
```

The output is a line-oriented evidence report. `public_evidence_full_spec=true` requires both
`public_criterion=true` and `independently_checkable=true`. The `external_operator_evidence` field is true
only when enough signed node evidence and operator identity attestation records are present. The individual
fields identify which post-run artifact or protocol observation is missing:

```text
public_evidence_full_spec=false
public_criterion=false
independently_checkable=true
published_evidence_bundle=true
block_history=true
finality_history=true
operator_identity_attestations=true
data_availability_measurements=true
miners=2
validators=1
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
