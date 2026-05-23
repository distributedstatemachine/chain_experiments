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
