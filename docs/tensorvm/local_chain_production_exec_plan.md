# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: Iteration 11, canonical useful-verification block validity over deterministic
  settled-receipt blockspace is implemented locally and under final validation.
- Required resumed Gate 0 was run first: `cargo test -p tensor_vm local_testnet --release` passed with
  5 release local-testnet library tests and the seed CLI integration test.
- Current head and remote: `07f2b052b998d0a18974f824c07ce2d50d29d33c`
  (`Add TensorVM canonical encoding commitment model`) is both local `HEAD` and `origin/main`.
  `git status --short --branch` showed `## main...origin/main` plus untracked `goal.md`.
- Iteration 10 was implemented and pushed as `2d6609e Add remote validator tensor fetch`, with follow-up
  evidence commit `1687f86 Record iteration 10 push evidence`. Later proof/doc commits landed on top:
  `e20a879`, `41a20aa`, and `07f2b05`.
- Standing blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree and tracked `docs/tensorvm` files.
  - The full Docker runtime gate remains unresolved. The latest recorded `check-local-testnet.sh` run
    against an already-running Compose cluster failed at the bounded gateway `/health` probe with
    `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.

## Readiness Matrix

| Capability | Status | Current evidence | Next action |
| --- | --- | --- | --- |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Keep one transition engine while replacing block validity |
| Role-owned miner receipts | Started | Miner role submits receipts through `ChainCommand::SubmitReceipt` and publishes receipt announcements | Require positive live counters after deterministic replay is removed |
| Role-owned validator attestations | Started | Validator role verifies assigned receipts, fetches missing tensors remotely, and submits attestations | Keep as input path for canonical blockspace |
| Remote tensor availability | Implemented/pushed | `2d6609e`; root-addressed tensor request-response and validator fetch counters | Reuse for block-check evidence; revisit slow-peer bounds later |
| Network-visible event ingestion | Started | Node runtime ingests jobs, receipts, attestations, and block headers | Replace header replay with block payload propagation in a later feature |
| Proposer/block production | Locally canonical core | `chain::proposer` selects registered validators; `produce_block` rejects unknown validators and ignores miner TensorWork | Wire live validator proposer networking in a later feature |
| Canonical useful-verification block validity | Partially implemented locally | Blocks carry selected-root/checks-root/beacon/target/nonce; strict vote validation checks state root, beacon, PoW, proposer, selected receipts, checks, attestation, and reward roots | Add exact parent snapshots, child-state apply theorem, challenge openings, retargeting, and fallback |
| Checker evidence | Updated | `tvmd service block` exposes PoW, canonical blockspace, checks-root, validator-proposer, and finality-validation evidence; checker asserts all booleans before scan exit | Full Docker checker still awaits `/health` blocker resolution |
| Restart/recovery matrix | Complete for current storage model | Rolling restart checker covers durable state/common head for current block model | Rerun after block serialization changes |
| Public deployment evidence | Not started | Public evidence fields still report incomplete independently-checkable status | Keep out of scope until local canonical path is stable |

## Active Feature Iteration

### Iteration 11: Canonical Useful-Verification Block Validity

Feature capability:
Replace the active block/proposer path with validator-owned useful-verification PoW over deterministic
settled-receipt blockspace. Blocks must commit to the canonical selected receipt set, aggregate
`checks_root`, beacon, difficulty target, and nonce; votes/finality must reject invalid blocks instead of
counting stake over unknown-validity hashes.

Readiness requirements covered:
- TensorWork must not select proposers.
- Registered validators are the only eligible block proposers.
- Settled receipts become deterministic blockspace input, not global receipt/job roots copied into every
  block.
- Block-level validity includes canonical receipt selection, recomputed checks evidence, PoW target, parent
  linkage, and proposer eligibility.
- Local service block evidence can expose the fields the checker will later assert.

Files/modules likely touched:
- `crates/tensor_vm/src/chain/state.rs`
- `crates/tensor_vm/src/chain/blocks.rs`
- `crates/tensor_vm/src/chain/proposer.rs`
- `crates/tensor_vm/src/chain/roots.rs`
- `crates/tensor_vm/src/chain/validation.rs`
- `crates/tensor_vm/src/chain/commands.rs`
- `crates/tensor_vm/src/chain.rs`
- `crates/tensor_vm/src/storage.rs`
- `crates/tensor_vm/src/localnet.rs`
- `crates/tensor_vm/src/testnet.rs`
- `crates/tensor_vm/src/rpc.rs`
- `crates/tensor_vm/src/main.rs`
- `crates/tensor_vm_explorer/src/lib.rs`
- `deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`
- Status/readiness/coverage docs.

Parallel subagents completed before edits:
- Readiness mapper: confirmed canonical target and blockers.
- Codebase explorer: mapped block/proposer/storage/runtime couplings and highest-risk compile fallout.
- Test coverage explorer: identified replacement tests for blockspace, PoW, checks root, validator proposer
  eligibility, storage, localnet, and CLI/Compose surfaces.
- Checker/docs explorer: identified `tvmd service block` as the first evidence hook and recommended keeping
  live validator proposer networking out of scope for this slice.

Implementation boundary:
- Replace `TensorBlock` fields `job_root`, `receipt_root`, and `randomness` with canonical block fields:
  `settled_receipt_set_root`, `checks_root`, `beacon`, `difficulty_target`, and `nonce`.
- Add deterministic canonical selection over settled receipts with count/TWU/byte caps using available
  receipt metadata; mark selected receipts as included and persist local block-selected receipt evidence.
- Add useful-verification PoW header/hash helpers and a deterministic local nonce search with test-friendly
  difficulty.
- Make block production fallible and validator-owned; miners with settled TensorWork must not be eligible
  proposers.
- Validate block parent, proposer registration, beacon, selected receipt root, block `checks_root`,
  state/reward roots, and PoW before accepting block votes/finality.
- Update storage block encoding/decoding, RPC/explorer block views, localnet/testnet block production
  callers, and service block evidence fields.

Out of scope for this iteration:
- Full public deployment evidence.
- Difficulty retargeting economics.
- Full challenge-window opening/dispute rewards for block-level `checks_root`.
- Removing deterministic replay/header catch-up or requiring live validator proposer networking in Compose.
- Hard positive live work assertions for every miner/validator while replay can still pre-fill work.
- Zero-receipt PoW-skip fallback unless it is naturally small after the core block validity change.

Narrow validation commands:
- `cargo fmt --check`
- `cargo check -p tensor_vm --all-targets`
- `cargo test -p tensor_vm --lib chain::tests`
- `cargo test -p tensor_vm --lib storage::tests`
- `cargo test -p tensor_vm --lib localnet::tests`
- `cargo test -p tensor_vm --lib testnet::tests::local_testnet_runs_full_matmul_receipt_attestation_settlement_round`
- `cargo test -p tensor_vm --lib testnet::tests::local_testnet_runs_linear_training_receipt_state_transition_round`
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`

Broad validation before commit:
- `cargo test -p tensor_vm local_testnet --release`
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`
- `cargo tarpaulin --workspace --offline` if targeted gates are clean and runtime is not blocked.
- `git diff --check`
- Attempt the full Docker checker if the known `/health` blocker is not still present.

Expected observable evidence:
- A produced block exposes parent hash, proposer, `settled_receipt_set_root`, selected receipt ids/count/TWU,
  caps, `checks_root`, difficulty target, nonce, PoW hash, and validity booleans.
- A miner address is rejected as a block proposer even with high settled TensorWork.
- A validator-produced block with invalid nonce, target, selected receipt set, or `checks_root` cannot gain
  finality votes.
- Storage round-trips the new block schema, and stale block fields are absent from active block encoding.

Verifier review:
- Read-only verifier reported six findings. Fixes applied in this iteration: strict vote validation now
  checks `state_root`; selected receipts are tracked and excluded from future selection; block beacon is
  checked against genesis/parent transition; `produce_block_with_rewards` restores reward state on failure;
  service block evidence reports persisted selected receipts; the checker no longer exits before all new
  block evidence booleans are observed; stale negative-proof docs were corrected.
- Remaining accepted gaps: exact persisted parent snapshots/child-state apply theorem, challenge openings
  for `checks_root`, difficulty retargeting/economics, fallback liveness, selected-leaf lifecycle metadata,
  and live validator proposer networking.

## Recent Iterations

### Iteration 10: Remote Validator Tensor Fetch

- Feature: validator role loops fetch missing receipt tensor artifacts from connected peers over libp2p
  request-response, verify tensors against requested commitment roots, insert/register tensors, and submit
  validator-owned attestations.
- Main changes: root-addressed tensor request/response messages, protocol-specific request-response
  dispatch, service-level tensor registration/fetch, validator role remote fetch counters, status/checker
  fields, and protocol count docs.
- Verifier: initial findings on malformed-payload loop abort and non-specific protocol dispatch were fixed;
  re-review reported no findings in scope.
- Validation passed: `cargo fmt --check`, `cargo check -p tensor_vm --all-targets`, focused p2p/node/role
  tests, CLI/Compose artifact tests, `cargo test -p tensor_vm local_testnet --release`,
  Compose config, `cargo tarpaulin --workspace --offline` with 254 tests and 98.73% workspace coverage,
  and `git diff --check`.
- Full Docker gate: still blocked at gateway `/health`.
- Commits: `2d6609e Add remote validator tensor fetch`; `1687f86 Record iteration 10 push evidence`.

### Iteration 9: Formalize MVP Core Soundness Boundary

- Feature: formal proof/audit docs for the MVP core and receipt-bound validator assignment enforcement in
  the shared chain engine.
- Main changes: assignment draw includes `receipt_id`; `SubmitAttestation` rejects unassigned validators;
  soundness findings/proof docs separate proved invariants from current consensus gaps.
- Validation passed: formatting, `cargo check`, scheduler/chain/role/CLI/Compose/local-testnet targeted
  tests, and `git diff --check`.
- Commits: `c42235c Add validator attestations and proof boundary`; `c916b19 Compile MVP core soundness
  findings`.

## Decision Log

- Keep the missing workflow document visible as a standing blocker; do not treat the readiness doc as a
  substitute.
- Preserve one shared chain engine. Deployment profiles can vary, but transition logic should not fork.
- Role-owned miner and validator work must mutate chain state through `ChainCommand` and publish through the
  shared p2p/event path.
- Do not require positive live Compose miner/validator-owned submissions yet while deterministic local replay
  can pre-apply jobs, receipts, attestations, or blocks before role loops observe unhandled work.
- Validator assignment is receipt-bound and enforced in the chain engine; persisting per-receipt assignment
  seed/provenance remains future work.
- For Iteration 11, replace active behavior directly with canonical names and fields. Do not add
  compatibility shims, legacy aliases, or parallel consensus modes.

## Validation Evidence

Resumed Iteration 11 checkpoint:
- `git status --short --branch`: `## main...origin/main` plus untracked `goal.md`.
- `git rev-parse HEAD`: `07f2b052b998d0a18974f824c07ce2d50d29d33c`.
- `git ls-remote origin refs/heads/main`: `07f2b052b998d0a18974f824c07ce2d50d29d33c refs/heads/main`.
- First executable gate: `cargo test -p tensor_vm local_testnet --release` passed before exploration or
  edits.
- Subagents completed: readiness mapper, codebase explorer, test coverage explorer, checker/docs explorer.

Post-implementation validation currently passed:
- `cargo check -p tensor_vm --all-targets`
- `cargo fmt --check`
- `cargo test -p tensor_vm --lib`
- `cargo test -p tensor_vm --lib chain::tests`
- `cargo test -p tensor_vm --lib storage::tests`
- `cargo test -p tensor_vm --lib localnet::tests`
- `cargo test -p tensor_vm --lib testnet::tests::local_testnet_runs_full_matmul_receipt_attestation_settlement_round`
- `cargo test -p tensor_vm --lib testnet::tests::local_testnet_runs_linear_training_receipt_state_transition_round`
- `cargo test -p tensor_vm --lib study::tests::zero_work_liveness_study_produces_blocks_from_validators`
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`
- `cargo test -p tensor_vm local_testnet --release`
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`
- `cargo tarpaulin --workspace --offline`: passed with 256 tests and 98.51% workspace line coverage.
- `git diff --check`
- Gateway `/health` re-check remains blocked:
  `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.

Latest unresolved full-gate output:

```text
curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received
local CPU testnet check failed: gateway route is not reachable: /health
```

## Archive

- Iteration 1, `56da38a Extract reusable node runtime state`: extracted reusable node runtime state,
  pending payloads, event ingest, and counters.
- Iteration 2, `1b9a104 Move network payload application into node runtime`: moved decoded job, receipt,
  and attestation payload application into chain-centric helpers using `ChainCommand`.
- Iteration 3, `0b19f62 Extract reusable network event driver`: moved event ordering, invalid accounting,
  pending retry, and block-header dispatch into the reusable node runtime driver.
- Iteration 4, `8f24509 Bind role runtimes to chain identities`: role commands derive wallet addresses,
  check registration, persist identity status, and expose checker evidence.
- Iteration 5, `286ef9a Extract role runtime loop boundary`: added named role loop wrappers with RPC serving,
  network ingestion, local production, and status output.
- Iteration 6, `7262aaa Track miner work readiness in role loop`: miner role readiness counters detect
  assigned, unreceipted jobs; full Docker checker timed out at gateway health.
- Iteration 7, `ac7e6eb Submit miner receipts from role loop`: miner role executes assigned work, inserts
  tensors, submits receipts, publishes announcements, and exposes counters.
- Iteration 8: validator role submits assigned receipt attestations through the shared chain engine when
  local tensor artifacts are present; remote fetching was deferred to Iteration 10.
