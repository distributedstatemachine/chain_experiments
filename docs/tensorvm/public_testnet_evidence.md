# TensorVM Public Testnet Evidence

Status: no complete external public-testnet evidence bundle is available yet.

This document is the publication target for the independently checkable evidence bundle required before
the TensorVM MVP can be called fully complete. A complete bundle must be produced from an external public
run, not from the local harness.

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

## Current Repository Evidence

The local reference crate exposes typed validation for this future bundle through
`PublicTestnetEvidenceBundle`. The validator intentionally separates:

- `PublicTestnetRunEvidence`, which checks run-level protocol evidence
- `PublicTestnetEvidenceBundle`, which additionally checks publication, signatures, auditors, and
  independently checkable supporting records

The current local simulation and docs do not satisfy this bundle requirement.

## Manifest Format

External evidence can be represented as a line-oriented manifest parsed by
`parse_public_testnet_evidence_manifest`. Blank lines and `#` comments are ignored. Hash values are
64-character hex strings with an optional `0x` prefix. Boolean values are `true` or `false`.

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
node=miner,<address-hex>,<operator-id-hex>,0,100799,<heartbeat-count>
node=validator,<address-hex>,<operator-id-hex>,0,100799,<heartbeat-count>
service=rpc,<endpoint-id-hex>,0,100799,<reachable-count>,<signed-health-check-count>
service=explorer,<endpoint-id-hex>,0,100799,<reachable-count>,<signed-health-check-count>
service=faucet,<endpoint-id-hex>,0,100799,<reachable-count>,<signed-health-check-count>
service=telemetry,<endpoint-id-hex>,0,100799,<reachable-count>,<signed-health-check-count>
```

The CLI reads a manifest file and reports the default full-spec evidence status:

```bash
tvmd public-evidence validate --manifest docs/tensorvm/public-testnet.evidence
```
