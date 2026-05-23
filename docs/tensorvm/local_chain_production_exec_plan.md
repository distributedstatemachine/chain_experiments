# Local Chain Production Execution Plan

This file is the source of truth for local-chain production-readiness progress, decisions, validation
commands, and blockers. It is intentionally compact; older detailed logs are archived as summaries once
their commits are pushed.

## Current State

- Active feature: none in progress. Next feature should start from the recommended sequence in
  `local_chain_production_readiness.md`: remote tensor request-response fetching for validator-owned
  attestations.
- Current status: local `main` is pushed to `origin/main` at
  `c916b192cb50318b23f9a84370559ef4520c6a37` (`Compile MVP core soundness findings`). Push output:
  remote reported the repository moved to `git@github.com:distributedstatemachine/tensor_vm.git`, but the
  push to `github.com:one-covenant/chain_experiments.git` succeeded:
  `4058e20..c916b19 main -> main`. Follow-up `git ls-remote origin refs/heads/main` confirmed the remote
  branch at `c916b192cb50318b23f9a84370559ef4520c6a37`.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is not present in the
    worktree or tracked `docs/tensorvm` files. The readiness document has been read in full; the missing
    workflow document cannot be read until restored or added.
  - The full Docker runtime gate has not passed since the recent role-loop work. The last attempted
    `check-local-testnet.sh` run against an already-running Compose cluster failed at the bounded gateway
    health probe for `/rpc/health` with `curl: (28) Operation timed out after 15002 milliseconds with 0
    bytes received`.
- Next action: open the next feature-sized iteration with the required checkpoint and parallel read-only
  subagents before implementation.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Shared chain engine and profile-neutral API | Complete for current v1 core | `Chain`, `ChainEngine`, `ChainCommand`, `ChainEvent`, split chain modules, profile tests; latest broad tests passed in Iteration 9 | Keep using shared engine for new role work |
| Role-owned command surfaces | Complete for command/loop boundary | Compose uses `proposer run`, `miner run`, `validator run`; checker verifies runtime commands and wallet registration | Keep role commands as counted operator entrypoints |
| Libp2p/shared node event ingestion | Started | Reusable node runtime driver ingests decoded jobs, receipts, attestations, and block headers; non-producers apply payloads through `ChainCommand` | Replace deterministic replay/block assembly with network-visible state |
| Miner-owned receipt submission | Started | Commit `ac7e6eb`; miner role can execute assigned unreceipted work, insert tensors, submit `ChainCommand::SubmitReceipt`, publish receipt announcements, and expose counters | Eventually require live positive miner-owned submissions once deterministic replay no longer masks work |
| Validator-owned attestation submission | Started | Commit `c42235c`; validator role can attest assigned receipts when local tensor artifacts exist, submits through `ChainCommand::SubmitAttestation`, publishes announcements, and exposes counters | Add remote tensor request-response fetching so validators are not limited to local artifacts |
| Remote tensor availability for validators | Not implemented | Readiness doc marks remote tensor request-response as next recommended commit | Implement validator fetch of miner-served tensors |
| Proposer/block production from network-visible state | Not implemented | Readiness doc still says proposer block assembly uses deterministic local replay/centralized production | Wire proposer loop to canonical network-visible receipts and attestations |
| Useful-verification PoW v2 block validity | Not implemented | MVP docs now identify v1 block/proposer path as unsound for v2 useful-verification PoW | Implement PoW puzzle, difficulty retargeting, canonical settled-receipt blockspace |
| Checker evidence for live acceptance items | Partial | Checker gates live jobs, receipts, settled receipts, rewards, attestations, tensor descriptor/row/chunk/opening fetch, telemetry, all-operator convergence, role counters, and block primitive evidence | Extend checks once role-owned network path replaces deterministic replay |
| Restart/recovery matrix | Complete for current local-store model | Rolling restart script checks stable peer IDs, advancing durable state, preserved finalized common head/state root, advancing block-log roots, and continued finalization | Rerun full matrix after role-owned block assembly changes |
| MVP core soundness boundary | Started | Commits `c42235c` and `c916b19`; formal proof and findings docs separate proved invariants, assumptions, and unsound gaps | Convert documented gaps into feature iterations |

## Active Feature Iteration

None. Before the next code edit, write the full checkpoint required by `goal.md`, including:

```text
Iteration N: <short title>
Feature capability:
Readiness requirements covered:
Files/modules likely touched:
Parallel subagents to run:
Parallelizable implementation workstreams:
Tests/checkers/docs to add or update:
Narrow validation commands:
Broad validation commands before commit:
Expected observable evidence:
Out of scope:
Split trigger: what would force this feature to be split smaller?
```

## Recent Iterations

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

Latest pushed state:
- `git status --short --branch`: `## main...origin/main` plus untracked `goal.md`.
- `git rev-parse HEAD`: `c916b192cb50318b23f9a84370559ef4520c6a37`.
- `git ls-remote origin refs/heads/main`: `c916b192cb50318b23f9a84370559ef4520c6a37 refs/heads/main`.

Latest full broad validation from Iteration 9:
- `cargo fmt`, `cargo fmt --check`, `cargo check -p tensor_vm --all-targets`, focused scheduler/chain/role
  tests, static Compose artifact test, `cargo test -p tensor_vm local_testnet --release`, and
  `git diff --check` passed.

Current unresolved full-gate output:

```text
curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received
local CPU testnet check failed: gateway route is not reachable: /rpc/health
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
