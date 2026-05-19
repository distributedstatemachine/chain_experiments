# TensorVM Public Testnet Runbook

This runbook is for the external public run required by the TensorVM MVP spec. It does not create public
testnet evidence by itself. Full-spec completion still requires independently operated public nodes,
externally reachable HTTPS services, mandatory libp2p propagation, a 7-day run, and a published evidence
bundle that validates with `public_evidence_full_spec=true`.

## Preconditions

Before advertising a public run:

- Build `tvmd` with the CUDA kernels enabled on the miner hosts that claim GPU execution.
- Provision at least 10 miner operators and 5 validator operators with independent operator identities.
- Publish external DNS names and valid TLS certificates for RPC, explorer, faucet, and telemetry.
- Publish reachable libp2p TCP listen addresses with nonzero ports for every node; localhost, private,
  link-local, `.local`, `.localhost`, `.test`, `.example`, `.invalid`, RFC example domains,
  documentation, shared-address, benchmarking, multicast, and reserved addresses are not acceptable for
  public evidence.
- On every non-bootstrap node, seed the durable peer book with `tvmd service peer add --data-dir
  <data-dir> --peer-id <bootstrap-peer-id> --address <bootstrap-libp2p-tcp-multiaddr>` before the service
  starts. The stored peer ID must match any `/p2p/<peer-id>` suffix already present in the address.
- Replace every placeholder in `env/public-testnet.env.example` and
  `manifests/public-testnet.preflight.example`.
- Start services through `systemd/tensorvm.service` or an equivalent unit that invokes
  `tvmd service serve` with `--p2p-listen` and the seeded peer book.
- Configure the reverse proxy from `nginx/tensorvm.conf` or an equivalent TLS proxy.

Run the preflight gate from the repository root or from a host with the same manifest:

```bash
tvmd public-testnet preflight --manifest deploy/tensorvm/manifests/public-testnet.preflight.example
```

The run must not start as a public MVP attempt unless the preflight reports:

```text
public_testnet_preflight_ready=true
deployment_plan_ready=true
cuda_ready_miners=true
production_libp2p_runtime=true
public_service_content_planned=true
public_services_planned=true
```

## Evidence Collection

Assign these identifiers before block production starts:

- one `bundle_id` for the public evidence bundle
- one `manifest_signer`
- one or more independent `auditor_id` values distinct from the `manifest_signer`
- one stable node address and one external operator ID per miner and validator, with disjoint operator
  IDs across miner and validator roles
- one stable endpoint ID for each public RPC, explorer, faucet, and telemetry service

Generate or collect these records during the run:

```bash
tvmd public-evidence publication ...
tvmd public-evidence auditor-record ...
tvmd public-evidence run-window ...
tvmd public-evidence run-window-from-file ...
tvmd public-evidence node-heartbeat ...
tvmd public-evidence node-heartbeat-from-file ...
tvmd public-evidence operator-attestation ...
tvmd public-evidence service-health ...
tvmd public-evidence service-health-from-file ...
tvmd public-evidence service-content ...
tvmd public-evidence service-content-from-bytes ...
tvmd public-evidence service-content-from-file ...
tvmd public-evidence network-observation ...
tvmd public-evidence network-observation-from-service-log ...
tvmd public-evidence record-summary ...
tvmd public-evidence record-artifact ...
tvmd public-evidence record-artifact-from-roots ...
tvmd public-evidence record-artifact-from-file ...
tvmd public-evidence record-summary-from-roots ...
tvmd public-evidence record-summary-from-file ...
```

The collected records must cover the full 7-day window, not only a final snapshot. The block observation
file for the signed run window should contain one
`run_window_observation=<block>,<unix-seconds>` line per observed block; prefer `run-window-from-file`
over manually copying start/end/block-count values when the raw block observation file is available. Each
node heartbeat
observation file should contain one
`node_heartbeat_observation=<role>,<node-address-hex>,<operator-id-hex>,<block>` line per observed block;
prefer `node-heartbeat-from-file` over manually copying first/last/count values when the raw heartbeat
file is available. The final node heartbeat count and each public service's reachable and signed
health-check counts must be at least the manifest's observed block count. Operator-attestation and
service-content observation times must fall
inside the signed run window, and every service-content URL must use the same HTTPS authority as the
matching service-health URL for that endpoint ID. Finalized blocks must not exceed observed blocks, and
available tensor receipts must not exceed checked tensor receipts. The network-runtime summary must be
derived from exactly one signed `network_runtime_observation=...` line per counted miner and validator
operator, with each observation using a public libp2p listen multiaddr and an observation timestamp inside
the signed run window. Prefer `network-observation-from-service-log` when the raw `tvmd service serve`
stdout/stderr log is available, so the peer ID, protocol counts, bootstrap-peer count, and DoS-control
settings are derived from the captured service runtime instead of being copied by hand. Every
service health observation file should contain one
`service_health_observation=<block>,reachable` or
`service_health_observation=<block>,unreachable` line per observed block; prefer
`service-health-from-file` over manually copying first/last/count values when the raw observation file is
available.
`record-summary` root must have a matching signed `record-artifact` locator for the external raw-record
artifact; when possible, use `record-summary-from-file` and `record-artifact-from-file` over the saved raw
record file instead of manually copying comma-separated roots. Preserve raw supporting records for:

- block history
- finality history
- node heartbeat observations for every counted miner and validator
- production libp2p observations for discovery, gossip, request/response, and DoS controls
- data-availability measurements
- invalid-work submissions and rejections
- reward-settlement records
- public service health observations for RPC, explorer, faucet, and telemetry
- public service content observations for `/chain/head`, `/explorer`, `/faucet/page`, and
  `/telemetry/dashboard`, each proving at least 64 observed bytes

For block, finality, data-availability, invalid-work, and reward summaries, the saved raw-record file may
contain exact typed lines:

```text
block_history_record=...
finality_history_record=...
data_availability_measurement=...
invalid_work_rejection=...
reward_settlement=...
```

`record-summary-from-file` and `record-artifact-from-file` hash each exact typed line with the selected
record kind before aggregation. Do not trim or pad those lines; whitespace-padded record lines are
rejected.

## Daily Checks

During the run, preserve signed checkpoint batches for every operator and public service each day. The
final summaries must cover the full observed block range and the full wall-clock run window:

- node heartbeats for every active miner and validator
- service-health records for every public service
- service-content records for every public service
- libp2p network-observation records from independent observers, one per counted public operator
- finalized block count and finality rate
- tensor receipt availability sample count and available count
- invalid work submitted and rejected
- reward settlements paid from verified TensorWork

Any outage or operator replacement must be reflected in the final evidence bundle. Do not backfill
operator identities, service-health records, or service-content records after the fact.

## Post-Run Validation

Create a real post-run manifest by copying
`manifests/public-testnet.evidence.example` and replacing every example ID, URI, count, root, and signature
with records from the external run. Then validate it:

```bash
tvmd public-evidence validate --manifest path/to/public-testnet.evidence
```

The public MVP gate requires the report to include at least:

```text
public_evidence_full_spec=true
public_criterion=true
independently_checkable=true
production_libp2p_runtime=true
deployed_public_service_content=true
deployed_public_services=true
required_miners=true
required_validators=true
required_run_duration=true
required_block_count=true
invalid_work_rejection_evidence=true
reward_settlement_evidence=true
supporting_record_artifacts=true
```

The default criteria currently require at least 10 miners, 5 validators, 604800 observed seconds, and
100800 observed blocks at the default block time.

## Publication

Publish the final evidence bundle to an external `https://`, `ipfs://`, or `ar://` URI accepted by the
validator. `https://` evidence URLs must use well-formed authorities without userinfo, whitespace, invalid
ports, malformed bracketed IPv6 authorities, or non-public hosts. Content-addressed `ipfs://` and `ar://`
URIs must start with a well-formed identifier segment using only ASCII alphanumerics, `-`, or `_`. The
publication must include:

- the validated manifest
- raw supporting records used to derive each summary root
- exactly one signed artifact locator line for each required raw supporting-record kind
- independent auditor records and audit artifacts
- operator identity artifacts
- public service health artifacts, including the raw observation files used with `service-health-from-file`
- public service content artifacts and content-root observations generated from exact captured response
  files with `service-content-from-file` when possible, or exact hex bytes with `service-content-from-bytes`
- libp2p network-observation artifacts, including the per-operator `network_runtime_observation=...`
  manifest lines, the source `tvmd service serve` logs used with
  `network-observation-from-service-log`, and the roots passed to
  `record-summary-from-roots --kind network-runtime` or the raw file passed to
  `record-summary-from-file --kind network-runtime`

After validation returns `public_evidence_full_spec=true`, link the published bundle from
`docs/tensorvm/implementation_status.md` and rerun the required verification commands from the repository
root.

## Current Blocker

This repository currently contains the deployment scaffold, preflight example, evidence example, and local
validators. It does not contain a real external 7-day public run or a published independently checkable
evidence bundle, so the full MVP spec remains incomplete.
