# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: Iteration 12, network-visible block payload propagation/admission is implemented and
  validated in the current worktree; commit/push evidence is recorded after the feature commit lands.
- Required resumed Gate 0 was run first: `cargo test -p tensor_vm local_testnet --release` passed with
  5 release local-testnet library tests and the seed CLI integration test.
- Iteration 11 feature and evidence commits are on `origin/main`: `e6129d1915562a1e865579e347d8cfb85855089e`
  and `800b031edea9b0b268cfe1fb487c9628cb2c782c`.
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
| Role-owned miner receipts | Started | Miner role submits receipts through `ChainCommand::SubmitReceipt` and publishes receipt announcements | Require positive live counters after service-owned timed production is removed |
| Role-owned validator attestations | Started | Validator role verifies assigned receipts, fetches missing tensors remotely, and submits attestations | Keep as input path for canonical blockspace |
| Remote tensor availability | Implemented/pushed | `2d6609e`; root-addressed tensor request-response and validator fetch counters | Reuse for block-check evidence; revisit slow-peer bounds later |
| Network-visible event ingestion | Implemented in current worktree | Node runtime ingests decoded jobs, receipts, attestations, and block payloads; headers/hashes are announcements only | Commit/push and rerun full Docker checker after `/health` blocker clears |
| Proposer/block production | Locally canonical core | `chain::proposer` selects registered validators; `produce_block` rejects unknown validators and ignores miner TensorWork | Wire live validator proposer networking in a later feature |
| Canonical useful-verification block validity | Partially implemented locally | Blocks carry selected-root/checks-root/beacon/target/nonce; strict vote validation checks state root, beacon, PoW, proposer, selected receipts, checks, attestation, and reward roots | Add exact parent snapshots, child-state apply theorem, challenge openings, retargeting, and fallback |
| Checker evidence | Updated | `tvmd service block` exposes PoW, canonical blockspace, checks-root, validator-proposer, and finality-validation evidence; checker asserts all booleans before scan exit | Full Docker checker still awaits `/health` blocker resolution |
| Restart/recovery matrix | Complete for current storage model | Rolling restart checker covers durable state/common head for current block model | Rerun after block serialization changes |
| Public deployment evidence | Not started | Public evidence fields still report incomplete independently-checkable status | Keep out of scope until local canonical path is stable |

## Active Feature Iteration

### Iteration 12: Network-Visible Block Payload Admission

Feature capability:
Replace non-producer block progress from header-triggered deterministic replay with full `TensorBlock`
payload gossip and strict chain admission through the shared node/chain boundary.

Readiness requirements covered:
- Blocks observed over libp2p must carry the exact validator-proposed useful-verification block payload.
- Non-producers must append received blocks only after validating parent linkage, registered validator
  proposer, proposer signature, aggregate signature, useful PoW, canonical settled-receipt selection,
  `checks_root`, attestation root, state root, reward root, and beacon.
- Runtime and checker evidence must distinguish block payload ingestion/application from header/hash
  announcements; header replay must not satisfy `role_network_applied_blocks`.

Files/modules likely touched:
- `crates/tensor_vm/src/api.rs`
- `crates/tensor_vm/src/p2p.rs`
- `crates/tensor_vm/src/chain/engine.rs`
- `crates/tensor_vm/src/chain/commands.rs`
- `crates/tensor_vm/src/chain/blocks.rs`
- `crates/tensor_vm/src/chain.rs`
- `crates/tensor_vm/src/node.rs`
- `crates/tensor_vm/src/main.rs`
- `crates/tensor_vm/src/lib.rs`
- `crates/tensor_vm/tests/tvmd_cli.rs`
- `crates/tensor_vm/tests/local_cpu_compose.rs`
- `deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`
- Readiness/exec-plan docs.

Parallel subagents completed before edits and verification:
- Readiness mapper: confirmed full block payload admission as the next useful slice and named required
  counters/checker evidence.
- Test coverage explorer: mapped existing header-replay coverage and proposed p2p, chain, node, CLI, and
  checker tests for block payload admission.
- Code-path explorer: identified p2p message/codec, node admission, runtime publish, and checker/status
  update points.
- Checker/docs explorer: identified status/checker fields required to prove block payload ingestion and
  application.
- Read-only verifier: reported invalid semantic block payloads could remain pending forever, remote
  admission bypassed modeled block-vote finality, the dormant header replay hook remained callable, and docs
  overclaimed full Docker status. The code/docs fixes are included in this iteration.

Implementation boundary:
- Add a canonical block payload codec/message and route it on the blocks gossip topic.
- Add a chain command/admission helper equivalent to `SubmitBlock(TensorBlock)` for strict external block
  append, including idempotent duplicate handling and rejection of conflicting/future/invalid payloads.
- Expand the MVP block aggregate certificate into local `BlockVote` records during remote admission so
  `is_block_finalized` remains backed by `has_block_finality`.
- Classify malformed or same-height semantic block payload failures as invalid instead of leaving them in
  pending retry forever; only missing-parent block payloads remain pending.
- Teach node event ingestion to count `block_payloads` and `block_payloads_applied`, apply payloads for
  non-producers, and keep `NewBlock`/`NewBlockHeader` as announcements only.
- Publish full block payloads after local production alongside the existing hash/header announcements.
- Surface role/status/checker evidence requiring positive non-producer block payload application and zero
  invalid network events.
- Remove the dormant runtime header-replay catch-up hook.

Out of scope for this iteration:
- Full validator/proposer role-owned block assembly.
- Public testnet evidence, TLS/DNS, seven-day run, CUDA, independent operators, or challenge-window
  economics.
- Retarget tuning and fallback liveness beyond the existing local useful-PoW target.

Narrow validation commands:
- `cargo fmt --check`
- `cargo check -p tensor_vm --all-targets`
- `cargo test -p tensor_vm --lib p2p::tests`
- `cargo test -p tensor_vm --lib node::tests`
- `cargo test -p tensor_vm --lib chain::tests`
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`

Broad validation before commit:
- `cargo test -p tensor_vm local_testnet --release`
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`
- `cargo tarpaulin --workspace --offline` if targeted gates are clean and runtime is not blocked.
- `git diff --check`
- Attempt the full Docker checker only if the known gateway `/health` blocker is clear.

## Recent Iterations

### Iteration 11: Canonical Useful-Verification Block Validity

- Feature: validator-owned useful-verification PoW over deterministic settled-receipt blockspace, replacing
  the prior settled-TensorWork proposer model.
- Main changes: canonical selected-receipt roots, `checks_root`, beacon, difficulty target, nonce, validator
  proposer checks, strict block-vote validation, selected-receipt inclusion tracking, and service-block/checker
  evidence for useful-PoW finality.
- Validation passed: formatting, `cargo check`, focused chain/storage/localnet/testnet/CLI/Compose gates,
  `cargo test -p tensor_vm local_testnet --release`, Compose config, Tarpaulin, and `git diff --check`.
- Full Docker gate: still blocked at gateway `/health`.
- Commits: `e6129d1915562a1e865579e347d8cfb85855089e`; `800b031edea9b0b268cfe1fb487c9628cb2c782c`.

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

Resumed Iteration 12 checkpoint:
- `git status --short --branch`: `## main...origin/main` plus untracked `goal.md`.
- Starting `HEAD`/`origin/main`: `800b031edea9b0b268cfe1fb487c9628cb2c782c`.
- First executable gate before exploration or edits:
  `cargo test -p tensor_vm local_testnet --release` passed with 5 release local-testnet library tests and
  the seed CLI integration test.
- Subagents completed: readiness mapper, code-path explorer, test coverage explorer, checker/docs explorer,
  and one read-only verifier.
- Verifier fixes applied: semantic invalid block payloads now count invalid instead of staying pending;
  remote block admission records modeled `BlockVote`s before finalization; dormant header replay mutation was
  removed; docs now keep the full Docker `/health` blocker visible.

Post-implementation validation currently passed:
- `cargo fmt --check`
- `cargo check -p tensor_vm --all-targets`
- `git diff --check`
- `cargo test -p tensor_vm --lib p2p::tests`
- `cargo test -p tensor_vm --lib node::tests`
- `cargo test -p tensor_vm --lib chain::tests`
- `cargo test -p tensor_vm --lib`
- `cargo test -p tensor_vm --tests`: 245 library tests, 21 `tvmd` binary tests, 1 local CPU Compose
  integration test, and 7 `tvmd_cli` integration tests passed.
- `cargo test -p tensor_vm local_testnet --release`: 5 release local-testnet library tests and the seed CLI
  integration test passed.
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`
- `cargo tarpaulin --workspace --offline`: passed with 260 instrumented workspace tests and 98.14%
  workspace line coverage (11,495/11,713 lines).
- `cargo fmt --check`, `cargo check`, and `git diff --check` were re-run after the verifier fixes.
- Full Docker checker was not rerun because the standing gateway `/health` blocker remains unresolved:
  `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.

Previous Iteration 11 evidence:
- Feature commit: `e6129d1915562a1e865579e347d8cfb85855089e`.
- Evidence commit: `800b031edea9b0b268cfe1fb487c9628cb2c782c`, confirmed on `origin/main`.

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
