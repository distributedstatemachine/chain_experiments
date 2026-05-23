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

## Iteration 4: Bind Role Runtime Identities

Readiness requirement:
Before moving receipt and attestation production into independent role processes, bind each long-running
`miner run`, `validator run`, and `proposer run` process to the chain address it is configured to operate
as, prove that address is registered in the loaded chain state, and expose the result through role runtime
status and the local checker.

Files likely touched:
- `crates/tensor_vm/src/main.rs`
- `crates/tensor_vm/tests/tvmd_cli.rs`
- `crates/tensor_vm/tests/local_cpu_compose.rs`
- `deploy/tensorvm/local-cpu/docker-compose.yml`
- `deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`
- `docs/tensorvm/local_chain_production_readiness.md`
- `docs/tensorvm/implementation_status.md`
- `docs/tensorvm/local_chain_production_exec_plan.md`

Subagents to run:
- Read-only codebase exploration for role runtime identity binding.
- Read-only test/coverage exploration for role runtime identity binding.
- Diff verification before commit, using the available verifier-style subagent path.

Tests/checkers to add or update:
- Process-level role-run/status coverage proving role wallet addresses and registration status are surfaced.
- Local CPU Compose/checker coverage proving configured wallets map to seeded chain miner/validator
  addresses.
- Keep existing role runtime, network ingest, and local-testnet gates green.

Commands to run before commit:
- `cargo fmt`
- `cargo fmt --check`
- `cargo check -p tensor_vm --all-targets`
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`
- `cargo test -p tensor_vm --bin tvmd role`
- `cargo test -p tensor_vm --bin tvmd network_ingest`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --offline`

Expected observable evidence:
- `role-runtime.status` records a stable role wallet address and whether that address is registered for the
  runtime role in the loaded chain.
- `tvmd service status` exposes the same role identity fields from persisted node status.
- Local Compose wallets match the seeded `LocalTestnet` miner and validator addresses, so future role-owned
  receipt and attestation producers can submit through the shared chain engine without being rejected as
  unknown operators.
- The local checker fails if any counted role runtime is not bound to a registered chain role address.

Out of scope:
- Producing receipts in miner containers.
- Producing attestations in validator containers.
- Replacing deterministic proposer block replay or synthetic local production.
- Treating the missing `docs/tensorvm/codex_5_5_local_chain_workflow.md` requirement as satisfied; the file
  remains absent from tracked files and Git history, and the standing blocker remains active.

## Iteration 4 Validation Log

Result: implemented and locally validated, with the full Docker runtime gate deferred for this narrow
identity-binding slice.

Changes made:
- `miner run`, `validator run`, and `proposer run` now derive a stable role wallet address from the wallet
  argument, check that address against the loaded chain state, and persist the result in
  `role-runtime.status`.
- `tvmd service status` and direct role-run stdout expose `role_wallet_address`,
  `role_wallet_registration`, and `role_wallet_registered`.
- Local CPU Compose now uses the seeded `LocalTestnet` wallet labels (`testnet-miner-*` and
  `testnet-validator-*`) instead of unregistered `local-*.key` labels for counted operators.
- `check-local-testnet.sh` fails unless every counted operator reports a non-`none`, registered role
  wallet, and unless miner services map to miner registration and validator services map to validator
  registration.

Validation evidence:
- `cargo fmt`: passed.
- `cargo fmt --check`: passed.
- `cargo check -p tensor_vm --all-targets`: passed.
- `cargo test -p tensor_vm --bin tvmd role_wallet`: passed; 1 binary unit test.
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`:
  passed; 1 integration test.
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`:
  passed; 1 integration test.
- `cargo test -p tensor_vm --bin tvmd role`: passed; 2 binary unit tests.
- `cargo test -p tensor_vm --bin tvmd network_ingest`: passed; 1 binary unit test.
- `cargo test -p tensor_vm local_testnet --release`: passed; 5 library tests plus the local CPU seed CLI
  test.
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`: passed.
- `cargo tarpaulin --workspace --offline`: passed; 250 instrumented tests, 99.21% workspace coverage
  (`10814/10900` lines), 99.96% `tensor_vm` coverage (`9969/9973` lines).

Not run:
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml build`
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait`
- `deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`
- `deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh`

Decision notes:
- This iteration deliberately binds and checks runtime identities before moving receipt and attestation
  production into separate role processes.
- `miner-00` still runs `proposer_run` for local gateway/proposer duties, but its wallet registration is
  checked as a seeded miner address because the Compose service remains a miner operator.
- The missing workflow document remains the standing blocker and is not treated as satisfied by this
  checkpoint.

## Iteration 5: Extract Role Runtime Loop Boundary

Readiness requirement:
Split the current `tvmd miner run`, `tvmd validator run`, and `tvmd proposer run` internals into an
explicit role-runtime loop boundary without changing consensus semantics. This should make the role command
path own the loop structure and role configuration while preserving the existing shared node runtime,
network ingest, persistence, deterministic catch-up, and local synthetic production behavior.

Files likely touched:
- `crates/tensor_vm/src/main.rs`
- `crates/tensor_vm/tests/tvmd_cli.rs`
- `docs/tensorvm/local_chain_production_readiness.md`
- `docs/tensorvm/implementation_status.md`
- `docs/tensorvm/local_chain_production_exec_plan.md`

Subagents to run:
- Goal-supervisor-style read-only check before editing.
- Read-only codebase exploration for role runtime loop boundary extraction.
- Read-only test/coverage exploration for role runtime loop boundary extraction.
- Diff verification before commit, using the available verifier-style subagent path.

Tests/checkers to add or update:
- Focused binary coverage proving role loop configuration selects the expected runtime command, runtime
  role, wallet binding, and local-producer policy.
- Process-level role-run/status coverage proving the externally visible command/status contract remains
  unchanged after the loop extraction.
- Keep Compose/checker behavior unchanged unless a new surfaced status field is intentionally added.

Commands to run before commit:
- `cargo fmt`
- `cargo fmt --check`
- `cargo check -p tensor_vm --all-targets`
- `cargo test -p tensor_vm --bin tvmd role`
- `cargo test -p tensor_vm --bin tvmd role_loop`
- `cargo test -p tensor_vm --bin tvmd network_ingest`
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`
- `cargo test -p tensor_vm local_testnet --release`
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`
- `cargo tarpaulin --workspace --offline`

Expected observable evidence:
- `miner run`, `validator run`, and `proposer run` construct role-owned loop wrappers instead of directly
  entering the generic service runtime function.
- The extracted loop boundary has named methods for status writes, RPC serving, network ingestion, and
  optional local production while preserving existing persistence points and counters.
- Existing `role-runtime.status`, direct role-run stdout, and `tvmd service status` fields remain compatible
  with the local checker.

Out of scope:
- Producing receipts in miner containers.
- Producing attestations in validator containers.
- Replacing deterministic proposer block replay or synthetic local production.
- Changing Compose runtime commands or the local checker contract.
- Treating the missing `docs/tensorvm/codex_5_5_local_chain_workflow.md` requirement as satisfied; the file
  remains absent from tracked files and Git history, and the standing blocker remains active.

## Iteration 5 Validation Log

Result: implemented and locally validated, with the full Docker runtime gate deferred for this structural
loop-boundary slice.

Changes made:
- Added role-run loop wrappers for `miner run`, `validator run`, and `proposer run`, so each role command
  owns its runtime command, runtime role, wallet binding, and role-specific readiness report before entering
  the shared node runtime.
- Extracted the shared runtime loop into a `RoleRuntimeLoop` boundary with named steps for status writes,
  RPC serving, network ingestion, optional local production, and final report generation.
- Preserved existing consensus behavior, persistence points, role status fields, local-producer policy,
  network ingestion, deterministic catch-up, and synthetic local production.
- Added focused binary tests for role-loop runtime config selection and role-specific readiness report
  preservation.

Validation evidence:
- `cargo fmt`: passed.
- `cargo fmt --check`: passed.
- `cargo check -p tensor_vm --all-targets`: passed.
- `cargo test -p tensor_vm --bin tvmd role`: passed; 4 binary unit tests.
- `cargo test -p tensor_vm --bin tvmd role_loop`: passed; 2 binary unit tests.
- `cargo test -p tensor_vm --bin tvmd network_ingest`: passed; 1 binary unit test.
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`:
  passed; 1 integration test.
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`:
  passed; 1 integration test.
- `cargo test -p tensor_vm local_testnet --release`: passed; 5 library tests plus the local CPU seed CLI
  test.
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`: passed.
- `cargo tarpaulin --workspace --offline`: passed; 250 instrumented tests, 99.21% workspace coverage
  (`10814/10900` lines), 99.96% `tensor_vm` coverage (`9969/9973` lines).

Not run:
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml build`
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait`
- `deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`
- `deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh`

Decision notes:
- Kept this iteration behavior-preserving so later commits can move miner receipt production and validator
  attestation production into the role-loop boundary without also changing loop mechanics.
- Kept Compose and checker fields unchanged; existing role runtime command, loop readiness, wallet
  registration, network counter, and local-producer checks continue to apply.
- The missing workflow document remains the standing blocker and is not treated as satisfied by this
  checkpoint.
