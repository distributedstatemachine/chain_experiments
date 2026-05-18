# TensorVM Public Testnet Evidence

Status: no complete external public-testnet evidence bundle is available yet.

This document is the publication target for the independently checkable evidence bundle required before
the TensorVM MVP can be called fully complete. A complete bundle must be produced from an external public
run, not from the local harness.

For pre-run launch readiness, use [`public_testnet_preflight.md`](public_testnet_preflight.md). A passing
preflight report is not a substitute for this post-run evidence bundle.

A deployment runbook for collecting, validating, and publishing the external-run evidence lives at
[`../../deploy/tensorvm/RUNBOOK.md`](../../deploy/tensorvm/RUNBOOK.md).

A checked example manifest lives at
[`../../deploy/tensorvm/manifests/public-testnet.evidence.example`](../../deploy/tensorvm/manifests/public-testnet.evidence.example).
It is useful for validating the post-run manifest shape, signature domains, and reporting fields, but it is
deliberately only a 60-second, 10-block, 2-miner, 1-validator sample and is not full-spec public-testnet
evidence.

## Required Bundle

A complete evidence bundle must include:

- a public `https://`, `ipfs://`, or `ar://` location for the evidence manifest
- manifest signature records
- signed independent auditor or verifier records
- signed wall-clock run window covering the full 7-day run
- signed miner and validator heartbeat history for the full run
- independent operator identity or attestation records
- signed block-history summary root for the full 7-day run
- signed finality-history summary root for the full 7-day run
- signed production libp2p network-observation summary root
- signed data-availability measurement summary root for checked tensor receipts
- signed invalid-work submission and rejection evidence
- signed reward-settlement records for verified TensorWork
- signed external artifact locators for the raw supporting records behind each block/finality/libp2p/data
  availability/invalid-work/reward-settlement summary root
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
validator requires signed node-heartbeat summaries, signed operator identity attestations, signed
service-health summaries, signed independent auditor records, and an external publication URI. It verifies
a manifest publication signature over the bundle ID, public URI, manifest signature count, and independent
auditor count. It also verifies signed auditor records over the bundle ID, public URI, external audit URI,
auditor ID, and observation time, plus a signed run-window record over the manifest bundle ID, start time,
end time, and observed block count. It verifies signed supporting-record roots for block history, finality
history, production libp2p observations, data-availability measurements, invalid-work rejections, and
reward settlements. It also requires signed external artifact locators for the raw supporting records behind
each summary root, and it derives `external_operator_evidence` from signed operator identity attestation
records that match the signed node-heartbeat records. These local checks are still only evidence-format
validation until an external run publishes real records.

## Manifest Format

External evidence can be represented as a line-oriented manifest parsed by
`parse_public_testnet_evidence_manifest`. Blank lines and `#` comments are ignored. Hash values are
64-character hex strings with an optional `0x` prefix. Boolean values are `true` or `false`. The manifest
signature covers the bundle ID, public URI, manifest signature count, and independent auditor count.
Auditor signatures cover the bundle ID, public URI, auditor ID, external audit URI, and observation time.
Block, finality, network-runtime, data-availability, invalid-work, and reward-settlement signatures cover
the bundle ID, record-set kind, record-set root, and record count. The run-window signature covers the
bundle ID, Unix start time, Unix end time, and observed block count. Heartbeat signatures cover the node
role, address, operator ID, first/last observed block, and heartbeat count. Operator identity signatures
cover the node role, node address, operator ID, external identity URI, and observation time.
Service-health signatures cover the service kind, endpoint ID, public URL, health path, first/last observed
block, reachable observation count, and signed health-check count. Supporting-artifact signatures cover the
bundle ID, record-set kind, external artifact URI, record root, and record count. Service URLs, supporting
artifact HTTPS URIs, auditor HTTPS URIs, and operator identity HTTPS URIs must use external hosts;
localhost, `.local`, loopback, private, link-local, and unspecified hosts are rejected. Supporting artifact,
auditor, and operator identity URIs may also use non-empty `ipfs://` or `ar://` content identifiers.
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
auditor=<auditor-address-hex>,https://auditor.example.test/tensorvm/audit.json,<unix-seconds>,<auditor-signature-hex>
record_artifact=block-history,https://evidence.example.test/tensorvm/block-history.json,<history-root-hex>,100800,<artifact-signature-hex>
record_artifact=finality-history,https://evidence.example.test/tensorvm/finality-history.json,<finality-root-hex>,100800,<artifact-signature-hex>
record_artifact=network-runtime,https://evidence.example.test/tensorvm/network-runtime.json,<network-runtime-root-hex>,4,<artifact-signature-hex>
record_artifact=data-availability,https://evidence.example.test/tensorvm/data-availability.json,<da-root-hex>,1000,<artifact-signature-hex>
record_artifact=invalid-work,https://evidence.example.test/tensorvm/invalid-work.json,<invalid-work-root-hex>,1,<artifact-signature-hex>
record_artifact=reward-settlement,https://evidence.example.test/tensorvm/reward-settlement.json,<reward-settlement-root-hex>,1,<artifact-signature-hex>
block_history_records=100800
block_history_root=<history-root-hex>
block_history_signature=<history-signature-hex>
finality_history_records=100800
finality_history_root=<finality-root-hex>
finality_history_signature=<finality-signature-hex>
operator_identity_attestation_records=15
operator=miner,<address-hex>,<operator-id-hex>,https://operator-a.example.test/tensorvm.json,<unix-seconds>,<operator-signature-hex>
operator=validator,<address-hex>,<operator-id-hex>,https://operator-b.example.test/tensorvm.json,<unix-seconds>,<operator-signature-hex>
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
invalid_work_rejection_records=1
invalid_work_rejection_root=<invalid-work-root-hex>
invalid_work_rejection_signature=<invalid-work-signature-hex>
reward_settlement_records=1
reward_settlement_root=<reward-settlement-root-hex>
reward_settlement_signature=<reward-settlement-signature-hex>
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
tvmd public-evidence validate --manifest deploy/tensorvm/manifests/public-testnet.evidence.example
```

Operators can generate the signed publication, run-window, node-heartbeat, and operator-attestation
manifest fields:

```bash
tvmd public-evidence publication \
  --bundle-id <bundle-id-hex> \
  --public-uri https://example.test/tensorvm/public-evidence.json \
  --manifest-signer <manifest-signer-address-hex> \
  --manifest-signature-count 1 \
  --independent-auditor-count 1

tvmd public-evidence auditor-record \
  --bundle-id <bundle-id-hex> \
  --public-uri https://example.test/tensorvm/public-evidence.json \
  --auditor-id <auditor-address-hex> \
  --audit-uri https://auditor.example.test/tensorvm/audit.json \
  --observed-at <unix-seconds>

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

tvmd public-evidence operator-attestation \
  --role miner \
  --address <node-address-hex> \
  --operator-id <operator-id-hex> \
  --identity-uri https://operator-a.example.test/tensorvm.json \
  --observed-at <unix-seconds>
```

The publication command rejects local/private evidence URIs, zero bundle IDs, zero manifest signers, and
zero signature or auditor counts. The auditor-record command rejects zero bundle IDs, local/private public
or audit URIs, zero auditor IDs, and empty observation times. Its output can be inserted directly as an
`auditor=...` line in the evidence manifest. The run-window command rejects zero IDs/signers, inverted
time windows, and empty block counts. The node-heartbeat command rejects zero node addresses, zero
operator IDs, inverted block ranges, and unsigned heartbeat summaries. The operator-attestation command
rejects zero node addresses, zero operator IDs, local/private identity URIs, and empty observation times.
Its output can be inserted directly as an `operator=...` line in the evidence manifest.

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

Operators can also generate signed production libp2p runtime observation records before rolling them into
the required network-runtime summary root:

```bash
tvmd public-evidence network-observation \
  --operator-id <operator-id-hex> \
  --peer-id <libp2p-peer-id> \
  --listen-address /dns/node-a.example.test/tcp/4001 \
  --observed-at <unix-seconds> \
  --gossip-topics 5 \
  --request-response-protocols 3 \
  --bootstrap-peers 2 \
  --max-transmit-bytes 1048576 \
  --request-timeout-seconds 10 \
  --max-concurrent-streams 128 \
  --idle-timeout-seconds 60
```

The command rejects zero operator IDs, malformed peer IDs, local/private or malformed libp2p multiaddrs,
missing discovery/bootstrap observations, missing gossip or request-response protocol counts, and missing
DoS-control limits. Its output is a signed `network_runtime_observation=...` line suitable for external
aggregation into the `network-runtime` record summary.

Operators can also generate signed supporting-record summary lines, including the production libp2p
network-observation summary required by full-spec evidence:

```bash
tvmd public-evidence record-summary \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --record-root <network-runtime-root-hex> \
  --record-count 4

tvmd public-evidence record-artifact \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --artifact-uri https://evidence.example.test/tensorvm/network-runtime.json \
  --record-root <network-runtime-root-hex> \
  --record-count 4

tvmd public-evidence record-summary-from-roots \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --record-roots <comma-separated-record-roots>
```

Supported record kinds are `block-history`, `finality-history`, `network-runtime`, `data-availability`,
`invalid-work`, and `reward-settlement`. The command emits the corresponding `<record>_records`,
`<record>_root`, and `<record>_signature` manifest fields using the same signature domain the validator
checks.
The `record-artifact` command emits a signed `record_artifact=...` manifest line that binds an external
raw-record artifact URI to the same record kind, root, and count. The full independently checkable gate
requires one valid artifact locator for every required supporting-record summary root.
The `record-summary-from-roots` variant derives a deterministic aggregate root and record count from the
provided supporting-record roots before signing those same summary fields.

The output is a line-oriented evidence report. `public_evidence_full_spec=true` requires both
`public_criterion=true` and `independently_checkable=true`. The `external_operator_evidence` field is true
only when enough signed node evidence and matching signed operator identity attestation records are present.
The individual fields identify which post-run artifact or protocol observation is missing:

```text
public_evidence_full_spec=false
public_criterion=false
independently_checkable=true
published_evidence_bundle=true
independent_auditor_records=true
signed_run_window=true
block_history=true
finality_history=true
operator_identity_attestations=true
network_runtime_observations=true
data_availability_measurements=true
signed_invalid_work_rejection_records=true
signed_reward_settlement_records=true
supporting_record_artifacts=true
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
