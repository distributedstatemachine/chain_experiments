# Local Chain Production Execution Plan

This file is the source of truth for local-chain production-readiness progress, decisions, validation
commands, and blockers.

## Standing Blockers

- `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is not present in the
  worktree or tracked `docs/tensorvm` files as of this checkpoint. The production-readiness document has
  been read in full; the missing workflow document cannot be read until it is restored or added.

## Iteration 1: Extract Reusable Node Runtime State

Readiness requirement:
Move node runtime counters, network-ingest accounting, and pending out-of-order payload retry state out of
the `tvmd` binary service loop so miner, validator, and proposer role loops can share the same runtime
boundary instead of depending on private binary state.

Files likely touched:
- `crates/tensor_vm/src/node.rs`
- `crates/tensor_vm/src/lib.rs`
- `crates/tensor_vm/src/main.rs`
- `docs/tensorvm/local_chain_production_readiness.md`
- `docs/tensorvm/implementation_status.md`
- `docs/tensorvm/tarpaulin_report.md`

Subagents to run:
- Read-only codebase exploration before further implementation.
- Read-only test/coverage exploration before further implementation.
- Diff verification before commit, using the available verifier-style subagent path.

Tests/checkers to add or update:
- Unit coverage for `NodeRuntimeState` loop counters.
- Unit coverage for `NetworkEventIngest::has_activity`.
- Unit coverage for `PendingNetworkPayloads::retry_with` applied, invalid, and still-pending outcomes.
- Unit coverage for duplicate pending payload IDs preserving the first queued payload.
- Existing runtime integration tests covering out-of-order network receipt/attestation retry through
  `RuntimeNetworkPayloadProcessor`.

Commands to run before commit:
- `cargo fmt`
- `cargo fmt --check`
- `cargo test -p tensor_vm --lib node::tests`
- `cargo test -p tensor_vm node`
- `cargo test -p tensor_vm --bin tvmd service_runtime_state_owns_loop_counters_and_pending_payloads`
- `cargo test -p tensor_vm --bin tvmd network_payload`
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --offline`

Expected observable evidence:
- `NodeRuntimeState`, `NetworkEventIngest`, `PendingNetworkPayloads`, `NetworkPayloadApply`, and
  `NetworkPayloadProcessor` are public library types exported from `tensor_vm`.
- `serve_service_with_runtime` uses `NodeRuntimeState` and no longer owns the reusable pending-payload data
  structures privately.
- Tests pass for the new `node` module and the existing binary runtime retry path.

Out of scope:
- Splitting miner, validator, and proposer into fully independent role-owned production loops.
- Replacing deterministic local replay block catch-up with fully network-assembled block production.
- Running the full Docker acceptance gate unless the narrower Rust validation passes first.

## Validation Log

- `cargo fmt`: passed.
- `cargo fmt --check`: passed.
- `cargo test -p tensor_vm --lib node::tests`: passed; 6 node runtime tests passed.
- `cargo test -p tensor_vm node`: passed; 22 filtered tests passed, including all node runtime tests.
- `cargo test -p tensor_vm --bin tvmd service_runtime_state_owns_loop_counters_and_pending_payloads`:
  passed; 1 binary runtime test passed.
- `cargo test -p tensor_vm --bin tvmd network_payload`: passed; 2 binary network-payload tests passed.
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`:
  passed; 1 process-level role command test passed.
- `cargo test -p tensor_vm local_testnet --release`: passed; 5 tensor_vm library tests and 1 `tvmd_cli`
  local-testnet seed test passed.
- `cargo tarpaulin --workspace --offline`: passed; 241 instrumented library tests passed with 99.24%
  workspace line coverage and 100.00% tensor_vm crate line coverage.

## Decisions And Notes

- Kept this iteration scoped to extracting reusable node runtime state and pending payload retry accounting.
  Full role-owned miner, validator, and proposer production loops remain out of scope for this slice.
- `PendingNetworkPayloads::retry_with` accepts `?Sized` processors so future role runtimes can use concrete
  or trait-object processors.
- Duplicate network payload IDs keep the first queued payload and ignore later duplicates; this preserves the
  existing `or_insert` behavior and is now covered by a focused unit test.

## Iteration 2: Move Network Payload Application To Node Runtime

Readiness requirement:
Move decoded job, receipt, and attestation payload application out of the `tvmd` binary and into the reusable
node runtime boundary so future miner, validator, and proposer loops can apply network-visible role work
through the shared chain engine without depending on private binary helpers.

Files likely touched:
- `crates/tensor_vm/src/node.rs`
- `crates/tensor_vm/src/main.rs`
- `crates/tensor_vm/src/lib.rs`
- `docs/tensorvm/local_chain_production_readiness.md`
- `docs/tensorvm/implementation_status.md`
- `docs/tensorvm/local_chain_production_exec_plan.md`

Subagents to run:
- Read-only codebase exploration for the payload application extraction.
- Read-only test/coverage exploration for the payload application extraction.
- Diff verification before commit, using the available verifier-style subagent path.

Tests/checkers to add or update:
- Move or add unit coverage for decoded network job payload application.
- Move or add unit coverage for receipt and attestation payload pending/applied/invalid outcomes.
- Keep binary coverage for out-of-order retry through the service runtime.

Commands to run before commit:
- `cargo fmt`
- `cargo fmt --check`
- `cargo check -p tensor_vm --all-targets`
- `cargo test -p tensor_vm --lib node::tests`
- `cargo test -p tensor_vm --lib payload`
- `cargo test -p tensor_vm --bin tvmd network_payload`
- `cargo test -p tensor_vm --bin tvmd network_ingest`
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --offline`

Expected observable evidence:
- `tvmd` delegates decoded network payload application to library node-runtime helpers.
- Job, receipt, and attestation payload application still use `ChainCommand::SubmitJob`,
  `ChainCommand::SubmitReceipt`, and `ChainCommand::SubmitAttestation`.
- Existing out-of-order receipt/attestation retry behavior is preserved.

Out of scope:
- Moving block catch-up and synthetic replay out of `tvmd`.
- Creating fully role-owned miner, validator, and proposer loops.
- Changing consensus semantics or local synthetic production policy.

## Iteration 2 Validation Log

- `cargo fmt`: passed.
- `cargo fmt --check`: passed.
- `cargo check -p tensor_vm --all-targets`: passed.
- `cargo test -p tensor_vm --lib node::tests`: passed; 10 node runtime tests passed.
- `cargo test -p tensor_vm --lib payload`: passed; 17 filtered library payload tests passed.
- `cargo test -p tensor_vm --bin tvmd network_payload`: passed; 2 binary network-payload tests passed.
- `cargo test -p tensor_vm --bin tvmd network_ingest`: passed; 1 binary network-ingest ordering test passed.
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`:
  passed; 1 Compose spec-shape test passed.
- `cargo test -p tensor_vm local_testnet --release`: passed; 5 tensor_vm library tests and 1 `tvmd_cli`
  local-testnet seed test passed.
- `cargo tarpaulin --workspace --offline`: passed; 245 instrumented library tests passed with 99.24%
  workspace line coverage and 100.00% tensor_vm crate line coverage.

## Iteration 2 Decisions And Notes

- Made payload application chain-centric instead of RPC-server-centric: the reusable helper boundary accepts
  `LocalChain` and the service runtime adapts with `ChainNetworkPayloadProcessor`.
- Kept libp2p message draining, event ordering, block catch-up, and persistence decisions inside `tvmd` for
  this slice.
- Simplified receipt and attestation application after explicit prechecks so accepted payloads go through
  `ChainCommand` and unexpected command failures classify as invalid, while missing prerequisites still
  classify as pending before command application.

## Iteration 3: Extract Reusable Network Event Driver

Readiness requirement:
Move network event ordering, decoded payload ingestion, pending payload retry integration, and block-header
application dispatch out of private `tvmd` helpers and into a reusable node runtime event driver. Keep the
current deterministic block catch-up and synthetic production semantics unchanged.

Files likely touched:
- `crates/tensor_vm/src/node.rs`
- `crates/tensor_vm/src/main.rs`
- `crates/tensor_vm/src/lib.rs`
- `docs/tensorvm/local_chain_production_readiness.md`
- `docs/tensorvm/implementation_status.md`
- `docs/tensorvm/local_chain_production_exec_plan.md`

Subagents to run:
- Read-only codebase exploration for event-driver extraction.
- Read-only test/coverage exploration for event-driver extraction.
- Diff verification before commit, using the available verifier-style subagent path.

Tests/checkers to add or update:
- Move or add unit coverage for network event ordering and invalid event counting in the reusable driver.
- Add unit coverage proving block-header application is dispatched only for non-producers.
- Keep binary coverage for service-runtime network payload retry and event ingestion.

Commands to run before commit:
- `cargo fmt`
- `cargo fmt --check`
- `cargo check -p tensor_vm --all-targets`
- `cargo test -p tensor_vm --lib node::tests`
- `cargo test -p tensor_vm --lib network_event`
- `cargo test -p tensor_vm --bin tvmd network_payload`
- `cargo test -p tensor_vm --bin tvmd network_ingest`
- `cargo test -p tensor_vm --bin tvmd network_catchup`
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`
- `cargo test -p tensor_vm --test tvmd_cli local_testnet_seed_cli_persists_cpu_chain_for_service_gateway`
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --offline`

Expected observable evidence:
- `tvmd` delegates message ordering and job/receipt/attestation/block-header ingestion to a library node
  runtime driver.
- The driver still applies payloads through `ChainCommand` and preserves pending receipt/attestation retry
  behavior.
- Local producer behavior and deterministic non-producer block catch-up remain unchanged.

Out of scope:
- Replacing deterministic block catch-up replay with network-assembled blocks.
- Splitting miner, validator, and proposer into fully independent role-owned work loops.
- Changing local synthetic job generation, receipt production, attestation production, or block production.

## Iteration 3 Validation Log

- `cargo fmt`: passed.
- `cargo fmt --check`: passed.
- `cargo check -p tensor_vm --all-targets`: passed.
- `cargo test -p tensor_vm --lib node::tests`: passed; 15 node runtime tests passed.
- `cargo test -p tensor_vm --lib network_event`: passed; 5 filtered network-event library tests passed.
- `cargo test -p tensor_vm --bin tvmd network_payload`: passed; 2 binary network-payload tests passed.
- `cargo test -p tensor_vm --bin tvmd network_ingest`: passed; 1 binary network-ingest ordering test
  passed.
- `cargo test -p tensor_vm --bin tvmd network_catchup`: passed; 3 binary network-catch-up tests passed.
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`:
  passed; 1 process-level role command test passed.
- `cargo test -p tensor_vm --test tvmd_cli local_testnet_seed_cli_persists_cpu_chain_for_service_gateway`:
  passed; 1 process-level local-testnet seed persistence test passed.
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`:
  passed; 1 Compose spec-shape test passed.
- `cargo test -p tensor_vm local_testnet --release`: passed; 5 tensor_vm library tests and 1 `tvmd_cli`
  local-testnet seed test passed.
- `cargo tarpaulin --workspace --offline`: passed; 250 instrumented workspace tests passed with 99.21%
  workspace line coverage and 99.96% tensor_vm crate line coverage. The remaining tensor_vm uncovered
  lines are rustfmt-split `P2pMessage` struct-pattern lines for receipt and attestation payload variants;
  the direct applied, pending retry, and invalid payload branches are covered by node runtime tests.

## Iteration 3 Decisions And Notes

- Kept the service-specific libp2p drain point and deterministic block replay in `tvmd`, but moved message
  ordering, decoded payload application, pending retry, and block-header dispatch policy into
  `ingest_network_messages`.
- Adapted `tvmd` through `RuntimeNetworkEventContext`, so reusable node runtime code only requires mutable
  `LocalChain` access and a block-header application callback.
- Added library coverage for producer versus non-producer block-header dispatch, invalid runtime message
  accounting, direct payload application, invalid payload handling, and out-of-order receipt/attestation
  retry after the job payload arrives.
