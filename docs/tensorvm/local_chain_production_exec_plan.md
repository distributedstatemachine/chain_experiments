# Local Chain Production Execution Plan

This file is the source of truth for local-chain production-readiness progress, decisions, validation
commands, and blockers. It is intentionally compact; older detailed logs are archived as summaries once
their commits are pushed.

## Current State

- Active feature: Iteration 10, remote validator tensor fetch, is implemented, validated, and pushed; this
  follow-up evidence update records the feature push.
- Current status: Iteration 10 feature commit `2d6609eb47480dac8c57fd8727e30654b0fcb885`
  (`Add remote validator tensor fetch`) was pushed to `origin/main`. Push output reported the repository
  moved to `git@github.com:distributedstatemachine/tensor_vm.git`, but the push to
  `github.com:one-covenant/chain_experiments.git` succeeded: `98f968b..2d6609e main -> main`. Follow-up
  `git ls-remote origin refs/heads/main` confirmed `2d6609eb47480dac8c57fd8727e30654b0fcb885`.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is not present in the
    worktree or tracked `docs/tensorvm` files. The readiness document has been read in full; the missing
    workflow document cannot be read until restored or added.
  - The full Docker runtime gate has not passed since the recent role-loop work. The latest attempted
    `check-local-testnet.sh` run against an already-running Compose cluster failed at the bounded gateway
    health probe for `/health` with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes
    received`.
- Next action: commit and push Iteration 10, then move to proposer/block production from network-visible
  state.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Shared chain engine and profile-neutral API | Complete for current v1 core | `Chain`, `ChainEngine`, `ChainCommand`, `ChainEvent`, split chain modules, profile tests; latest broad tests passed in Iteration 9 | Keep using shared engine for new role work |
| Role-owned command surfaces | Complete for command/loop boundary | Compose uses `proposer run`, `miner run`, `validator run`; checker verifies runtime commands and wallet registration | Keep role commands as counted operator entrypoints |
| Libp2p/shared node event ingestion | Started | Reusable node runtime driver ingests decoded jobs, receipts, attestations, and block headers; non-producers apply payloads through `ChainCommand` | Replace deterministic replay/block assembly with network-visible state |
| Miner-owned receipt submission | Started | Commit `ac7e6eb`; miner role can execute assigned unreceipted work, insert tensors, submit `ChainCommand::SubmitReceipt`, publish receipt announcements, and expose counters | Eventually require live positive miner-owned submissions once deterministic replay no longer masks work |
| Validator-owned attestation submission | Started | Commit `c42235c`; validator role can attest assigned receipts, now including tensors fetched over libp2p request-response in the current Iteration 10 worktree | Require positive live validator-owned work after deterministic replay no longer masks work |
| Remote tensor availability for validators | Implemented in current worktree | Iteration 10 adds root-addressed tensor request-response, validator fetch/verify/insert/register, status/checker counters, and verifier re-review | Commit/push, then broaden live-runtime assertions after proposer work |
| Proposer/block production from network-visible state | Not implemented | Readiness doc still says proposer block assembly uses deterministic local replay/centralized production | Wire proposer loop to canonical network-visible receipts and attestations |
| Useful-verification PoW v2 block validity | Not implemented | MVP docs now identify v1 block/proposer path as unsound for v2 useful-verification PoW | Implement PoW puzzle, difficulty retargeting, canonical settled-receipt blockspace |
| Checker evidence for live acceptance items | Partial | Checker gates live jobs, receipts, settled receipts, rewards, attestations, tensor descriptor/row/chunk/opening fetch, telemetry, all-operator convergence, role counters, and block primitive evidence | Extend checks once role-owned network path replaces deterministic replay |
| Restart/recovery matrix | Complete for current local-store model | Rolling restart script checks stable peer IDs, advancing durable state, preserved finalized common head/state root, advancing block-log roots, and continued finalization | Rerun full matrix after role-owned block assembly changes |
| MVP core soundness boundary | Started | Commits `c42235c` and `c916b19`; formal proof and findings docs separate proved invariants, assumptions, and unsound gaps | Convert documented gaps into feature iterations |

## Active Feature Iteration

Iteration 10: Remote Validator Tensor Fetch

Feature capability:
Validator role loops can fetch missing receipt tensor artifacts from connected peers over the libp2p
request-response path, insert verified tensors into local runtime storage, and then submit validator-owned
attestations through `ChainCommand::SubmitAttestation`. The status/checker surface must distinguish remote
fetch attempts and successes from tensors made locally available by deterministic replay.

Readiness requirements covered:
- Validators fetch tensor data, validate receipts, and submit attestations from their role loop.
- Tensor fetches move through libp2p/request-response or the shared node event path rather than local
  in-memory shortcuts.
- Local checker/status evidence can distinguish validator remote tensor availability from gateway HTTP
  tensor serving and deterministic local replay.

Files/modules likely touched:
- `crates/tensor_vm/src/api.rs`
- `crates/tensor_vm/src/p2p.rs`
- `crates/tensor_vm/src/tensor.rs`
- `crates/tensor_vm/src/rpc.rs`
- `crates/tensor_vm/src/node.rs`
- `crates/tensor_vm/src/main.rs`
- `crates/tensor_vm/tests/tvmd_cli.rs`
- `crates/tensor_vm/tests/local_cpu_compose.rs`
- `deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`
- `docs/tensorvm/local_chain_production_readiness.md`
- `docs/tensorvm/implementation_status.md`
- `docs/tensorvm/tarpaulin_report.md`
- `docs/tensorvm/local_chain_production_exec_plan.md`

Parallel subagents to run:
- `readiness-mapper`: completed; mapped remote validator tensor fetch to readiness requirements and
  blockers.
- `tensorvm-codebase-explorer`: completed; identified validator role local-artifact gate, p2p request-
  response hooks, and the root-versus-tensor-id protocol gap.
- `tensorvm-test-coverage-explorer`: completed; identified missing validator remote-fetch, p2p service, and
  status/checker coverage.
- Second `tensorvm-codebase-explorer`: completed; focused on checker/status fields and Docker observability.
- `tensorvm-verifier`: run before commit against the integrated diff.

Parallelizable implementation workstreams:
- Parent owns code and docs in this worktree to avoid p2p/main/status file collisions.
- Read-only verifier/test-runner agents may run after the parent has a coherent diff; no parallel writers
  for this iteration.

Tests/checkers/docs to add or update:
- P2p message and service tests for root-addressed tensor request-response, successful response, not-found
  response, and outbound failure/timeout handling.
- Validator role tests proving an assigned receipt with missing local artifacts can fetch remote tensors,
  insert them, submit exactly one valid attestation, and skip unregistered/unassigned/already-attested or
  mismatched-fetch cases.
- Node runtime/status coverage for remote validator tensor fetch attempts, successes, failures, bytes, and
  inserted tensor counters.
- `tvmd service status`, `role-runtime.status`, and direct role-run stdout fields for validator remote-fetch
  counters.
- Static Compose/checker tests and `check-local-testnet.sh` parsing for the new fields.
- Readiness/status docs updated with the new capability and any full-Docker-gate blocker.

Narrow validation commands:
- `cargo fmt --check`
- `cargo check -p tensor_vm --all-targets`
- `cargo test -p tensor_vm --lib p2p`
- `cargo test -p tensor_vm --lib tensor`
- `cargo test -p tensor_vm --lib node::tests`
- `cargo test -p tensor_vm --bin tvmd validator_role`
- `cargo test -p tensor_vm --bin tvmd role`
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`

Broad validation commands before commit:
- `cargo test -p tensor_vm local_testnet --release`
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`
- `cargo tarpaulin --workspace --offline`
- `git diff --check`
- Attempt the full Docker checker if the narrower gates pass and the existing `/rpc/health` blocker is not
  still present.

Expected observable evidence:
- A validator with an assigned receipt and no local tensors issues bounded libp2p request-response fetches
  for the receipt's committed tensor roots.
- Successfully fetched tensors are reconstructed/verified against the requested commitment root before
  insertion.
- The validator submits a valid attestation only after the required fetched/local artifact bundle exists.
- Remote-fetch counters appear in direct role-run output, `role-runtime.status`, `tvmd service status`, and
  local checker output.
- Request-response messages remain outside gossip ingestion and do not increment `network_invalid_events`.

Out of scope:
- Proposer/block production from network-visible receipts and attestations.
- Useful-verification PoW v2, canonical settled-receipt blockspace, difficulty retargeting, and
  receipt-lifecycle assignment seed persistence.
- Requiring every live Compose validator to report positive remote fetches while deterministic block replay
  can still pre-fill or pre-attest work.
- Public deployment evidence or mainnet security claims.

Split trigger: what would force this feature to be split smaller?
- If service-level libp2p request-response support requires a broad async runtime rewrite, split out
  `TensorVmLibp2pService` request-response API and counters first.
- If root-addressed tensor fetch requires a larger protocol redesign than a bounded tensor-by-root message,
  split protocol extension from validator role integration.
- If targeted p2p service tests are flaky or blocked by runtime behavior, commit the request-response
  service API with proof tests before integrating validator role fetch.

## Recent Iterations

### Iteration 10: Remote Validator Tensor Fetch

Feature capability:
Validator role loops fetch missing receipt tensor artifacts from connected peers over libp2p
request-response, verify fetched tensor payloads against the requested commitment roots, insert/register
the tensors locally, and then submit validator-owned attestations through `ChainCommand::SubmitAttestation`.

Changes made:
- Added root-addressed tensor request/response messages and bounded tensor payload encoding/decoding.
- Split libp2p request-response into protocol-specific behaviours so by-root tensor fetches use
  `/tensorchain/1/tensor/by-root` rather than the generic first matching protocol.
- Added service-level connected-peer inspection, tensor registration, and bounded request-response calls.
- Registered miner-produced, catch-up, and synthetic local tensors with the p2p service for remote serving.
- Added validator role remote fetch for missing receipt roots; corrupt payloads, mismatched response roots,
  and wrong tensor roots are counted as failed responses and do not stop the validator loop.
- Surfaced remote fetch attempts, successes, failures, bytes, and inserted tensor counters through direct
  role output, `role-runtime.status`, `tvmd service status`, static compose tests, and
  `check-local-testnet.sh`.
- Updated readiness/status/networking/public-evidence docs and protocol count examples from 3 to 4
  request-response protocols.

Verifier:
- Initial read-only verifier found a malformed-payload loop abort and non-specific request-response
  protocol dispatch.
- Both issues were fixed; re-review reported no findings in the fix scope.
- Residual operational risk: the validator fetch path is bounded per request, but a tick can still spend
  `missing_roots * connected_peers * request_timeout` when many connected peers are slow. This is acceptable
  for the current local feature slice and should be revisited before hard live-validator assertions.

Validation evidence:
- `cargo fmt --check`: passed.
- `cargo check -p tensor_vm --all-targets`: passed.
- `cargo test -p tensor_vm --lib p2p`: passed; 26 p2p tests.
- `cargo test -p tensor_vm --lib node::tests`: passed; 15 node runtime tests.
- `cargo test -p tensor_vm --bin tvmd validator_role`: passed; 3 validator-role tests.
- `cargo test -p tensor_vm --bin tvmd role`: passed; 11 role/runtime tests.
- `cargo test -p tensor_vm --bin tvmd validator_remote_tensor_response_rejects_corrupt_or_mismatched_payloads`:
  passed.
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`:
  passed.
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`:
  passed.
- `cargo test -p tensor_vm local_testnet --release`: passed; 5 release local-testnet library tests plus
  the seed CLI integration test.
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`: passed.
- `cargo tarpaulin --workspace --offline`: passed; 254 instrumented tests, 98.73% workspace coverage
  (`11063/11205` lines).
- `git diff --check`: passed.

Full Docker gate status:
- `TENSORVM_LOCAL_CPU_EXPLORER_PORT=18080 deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh` failed
  against an already-running Compose cluster at `/health` with a 15-second curl timeout and 0 bytes
  received. The new validator remote-fetch checker assertions were not reached because the gateway health
  probe failed first.

Commit and push:
- `2d6609e Add remote validator tensor fetch`
- Pushed to `origin/main`; remote head confirmed at
  `2d6609eb47480dac8c57fd8727e30654b0fcb885` before this follow-up evidence update.

Known remaining gaps:
- Proposer/block production still needs to move to network-visible receipts and attestations.
- The current block type and proposer path are still v1; useful-verification PoW over canonical
  settled-receipt blockspace remains unimplemented.
- Full live Compose positive role-owned work assertions remain deferred while deterministic replay can
  pre-fill or pre-attest work.

### Iteration 9: Formalize MVP Core Soundness Boundary

Feature capability:
Create a formal proof/audit document for the MVP core and harden the attestation admission invariant exposed
by that proof work: validator assignment must be receipt-bound and enforced by the shared chain engine, not
only by role-loop callers.

Changes made:
- Validator assignment now includes `receipt_id` in the assignment draw.
- `ChainCommand::SubmitAttestation` rejects validators not assigned to the receipt.
- Added `docs/tensorvm/mvp_core_formal_proofs.md` with locally proved verifier/attestation claims,
  explicit assumptions, and current unsound consensus gaps.
- Added `docs/tensorvm/mvp_core_soundness_findings.md` with a structured audit of the MVP core boundary.
- Updated status/audit/coverage docs so superseded settled-TensorWork proposer evidence is no longer
  counted as v2 MVP proof evidence.

Validation evidence:
- `cargo fmt`: passed.
- `cargo fmt --check`: passed.
- `cargo check -p tensor_vm --all-targets`: passed.
- `cargo test -p tensor_vm --lib scheduler::tests`: passed; 9 scheduler tests.
- `cargo test -p tensor_vm --lib chain::tests::unassigned_validator_attestations_are_rejected`: passed.
- `cargo test -p tensor_vm --lib chain::tests`: passed; 27 chain tests.
- `cargo test -p tensor_vm --bin tvmd validator_role`: passed; 2 validator-role tests.
- `cargo test -p tensor_vm --bin tvmd role`: passed; 10 role/runtime tests.
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`:
  passed.
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`:
  passed.
- `cargo test -p tensor_vm local_testnet --release`: passed.
- `git diff --check`: passed.

Commit and push:
- `c42235c Add validator attestations and proof boundary`
- `c916b19 Compile MVP core soundness findings`
- Pushed to `origin/main`; remote head confirmed at
  `c916b192cb50318b23f9a84370559ef4520c6a37`.

Known remaining gaps:
- The current block type and proposer path are still v1. Useful-verification PoW over canonical
  settled-receipt blockspace is the next coherent core feature after role-owned data availability.
- Validator assignment is receipt-bound but still uses current finalized randomness at attestation admission
  rather than a stored receipt-lifecycle seed.
- The full Docker runtime gate was not rerun for this iteration.

### Iteration 8: Submit Validator Attestations From Role Loop

Feature capability:
Move the first validator-owned mutating work step into the validator role loop. Registered validator roles
submit attestations through the shared chain engine when assigned receipts have local tensor artifacts.

Changes made:
- Implemented validator-owned attestation submission inside the role loop for the first assigned unattested
  receipt with local tensor artifacts, using `ReferenceValidatorRole` for verification and
  `ChainCommand::SubmitAttestation` for chain mutation.
- Added local tensor lookup on `RpcNode` so validator role code can verify from locally stored artifacts
  without reaching into private RPC state or re-executing miner work.
- Missing local artifacts are reported through validator role counters and skipped.
- Surfaced validator work and attestation counters through direct role-run stdout, `role-runtime.status`,
  `tvmd service status`, and the local checker.

Validation evidence:
- `cargo fmt`: passed.
- `cargo fmt --check`: passed.
- `cargo check -p tensor_vm --all-targets`: passed.
- `cargo test -p tensor_vm --lib node::tests`: passed; 15 node runtime tests.
- `cargo test -p tensor_vm --lib rpc::tests::tensor_rpc_serves_descriptor_rows_chunks_and_openings`:
  passed.
- `cargo test -p tensor_vm --bin tvmd validator_role`: passed; 2 validator-role tests covering assigned
  unattested observation, missing local artifacts, unregistered/unassigned skips, submission, and duplicate
  skip behavior.
- `cargo test -p tensor_vm --bin tvmd role`: passed; 10 binary role tests.
- `cargo test -p tensor_vm --bin tvmd role_loop`: passed; 2 role-loop tests.
- `cargo test -p tensor_vm --bin tvmd miner_role`: passed; 4 miner-role tests.
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`:
  passed.
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`:
  passed.
- `cargo test -p tensor_vm local_testnet --release`: passed.
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`: passed.
- `cargo tarpaulin --workspace --offline`: passed; 250 instrumented tests, 99.21% workspace coverage
  (`10865/10952` lines), 99.95% `tensor_vm` coverage (`10020/10025` lines).

Full Docker gate status:
- `TENSORVM_LOCAL_CPU_EXPLORER_PORT=18080 deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh` failed
  against an already-running Compose cluster at `/rpc/health` with a 15-second curl timeout and 0 bytes
  received. Docker Compose config and static checker shape gates passed; full runtime gate did not.

Out of scope preserved:
- Remote tensor request-response fetching.
- Requiring live Compose validators to report positive validator-owned submissions while deterministic
  block-header catch-up can still replay already-attested receipts first.
- Replacing deterministic proposer block replay or synthetic local production.

## Decision Log

- Keep `goal.md`'s missing workflow-doc requirement visible as a standing blocker. Do not treat
  `docs/tensorvm/local_chain_production_readiness.md` as a substitute for the absent
  `docs/tensorvm/codex_5_5_local_chain_workflow.md`.
- Preserve one shared chain engine. Local/testnet/mainnet may vary by profile configuration and deployment
  adapters, not by separate transition logic.
- Role-owned miner and validator work must mutate chain state through `ChainCommand` and publish through
  the existing p2p/shared event path.
- Do not require positive live Compose miner/validator-owned submissions yet while deterministic local
  replay can pre-apply jobs, receipts, and attestations before role loops observe unhandled work.
- Missing local tensor artifacts in validator role code are counted and skipped; submitting `Unavailable`
  attestations and remote tensor fetching remain future work.
- Validator assignment is now receipt-bound and enforced in the chain engine. Persisting per-receipt
  assignment seed/provenance remains future work.

## Validation Evidence

Iteration 10 feature push evidence:
- `git status --short --branch` after feature push: `## main...origin/main` plus untracked `goal.md`.
- `git rev-parse HEAD`: `2d6609eb47480dac8c57fd8727e30654b0fcb885`.
- `git ls-remote origin refs/heads/main`: `2d6609eb47480dac8c57fd8727e30654b0fcb885
  refs/heads/main`.

Latest full broad validation from Iteration 10:
- `cargo fmt --check`, `cargo check -p tensor_vm --all-targets`, focused p2p/node/role/status tests,
  static Compose artifact test, `cargo test -p tensor_vm local_testnet --release`,
  `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`, `cargo tarpaulin
  --workspace --offline`, and `git diff --check` passed.

Current unresolved full-gate output:

```text
curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received
local CPU testnet check failed: gateway route is not reachable: /health
```

## Archive

- Iteration 1, `56da38a Extract reusable node runtime state`: extracted `NodeRuntimeState`,
  `NetworkEventIngest`, `PendingNetworkPayloads`, and runtime counters into reusable library boundaries;
  targeted node/runtime tests and tarpaulin passed.
- Iteration 2, `1b9a104 Move network payload application into node runtime`: moved decoded job, receipt,
  and attestation payload application into chain-centric node helpers that use `ChainCommand`; targeted
  payload, network, Compose-shape, local-testnet, and tarpaulin gates passed.
- Iteration 3, `0b19f62 Extract reusable network event driver`: moved network event ordering, invalid
  event accounting, pending retry, and block-header dispatch into the reusable node runtime driver while
  preserving deterministic catch-up; targeted and broad Rust validation plus tarpaulin passed.
- Iteration 4, `8f24509 Bind role runtimes to chain identities`: role commands derive wallet addresses,
  check miner/validator registration, persist role identity status, and checker verifies registered
  role wallets; targeted Rust, Compose config, and tarpaulin gates passed.
- Iteration 5, `286ef9a Extract role runtime loop boundary`: added role-run loop wrappers and a named
  `RoleRuntimeLoop` with status, RPC serving, network ingestion, and local production steps; targeted Rust,
  Compose config, and tarpaulin gates passed.
- Iteration 6, `7262aaa Track miner work readiness in role loop`: miner role loop detects registered,
  assigned, unreceipted jobs and exposes readiness counters; targeted Rust, Compose config, tarpaulin, and
  `git diff --check` passed; full Docker checker timed out at gateway `/rpc/health`.
- Iteration 7, `ac7e6eb Submit miner receipts from role loop`: miner role executes assigned unreceipted
  jobs, inserts served tensors, submits receipts through `ChainCommand::SubmitReceipt`, publishes receipt
  announcements, and exposes receipt/tensor counters; targeted Rust, Compose config, and tarpaulin gates
  passed; full Docker checker still timed out at gateway `/rpc/health`.
