# TensorVM Code Quality Deep Dive

This is a strict implementation-quality review of the current TensorVM codebase, focused on separation of
concerns, SOLID-style ownership boundaries, DRY, idiomatic Rust, and long-term maintainability.

The codebase has a strong technical core: the tensor primitives, verification math, typed chain commands,
and local test coverage show real engineering effort. The main risk is not lack of tests. The main risk is
that large operational surfaces have grown around the core and now own too much behavior through public
state, string contracts, shell scripts, duplicate codecs, and god modules.

## Executive Verdict

The biggest design problem is that TensorVM does not yet have a hard canonical mutation boundary.
`ChainCommand` and `ChainEvent` are the right direction, but `Chain`, `ChainState`, runtime loops,
RPC, tests, and deployment scripts can still bypass or duplicate chain behavior.

That creates four recurring failure modes:

- Consensus invariants are enforced in some paths and bypassed in others.
- Runtime adapters accumulate protocol logic that belongs in `chain/*`.
- Tests certify string outputs and shell contracts instead of typed behavior.
- Manual codecs and parsers drift across p2p, storage, RPC, CLI, and shell.

The repair should be structural, not cosmetic. Splitting files before fixing ownership would only move
spaghetti around.

## Implementation Progress

- Iteration 1 completed the core abstraction rename: the protocol state machine is now `Chain`, with no
  `LocalChain` compatibility alias in the Rust source or public exports. The remaining findings below refer
  to `Chain` as the current type, while the rationale section preserves why the old name was removed.
- Iteration 2 started the encapsulation work by making `Chain`'s top-level `params`, `state`, and `blocks`
  fields crate-private and adding inherent read-only accessors. Internal modules can still mutate
  `ChainState` directly, so the larger command-boundary finding remains open.
- Iteration 3 moved local synthetic block production and the localnet test finality helper onto
  `ChainCommand::ProduceBlock` and `ChainCommand::SubmitBlockVote`. The local CPU round now uses the
  command boundary for jobs, receipts, attestations, settlement, block production, and block votes.
- Iteration 4 removed the silent `apply_transaction` success path for receipt and attestation reference
  submissions. Those transaction variants are now explicitly txpool-only via `Transaction::is_reference_submission`,
  and direct chain application returns an error instead of pretending to mutate state.
- Iteration 5 stopped runtime RPC serving from persisting chain state after read-only requests. The service
  loop now compares chain snapshots around a served RPC request, persists only when the chain changed, and
  still updates served-request runtime status for read-only traffic.
- Iteration 6 routed `LocalTestnet` participant registration, job/receipt/attestation submission,
  settlement, block production, and block votes through `ChainCommand`. Model registration and model
  transition remain direct until the command API grows model-specific variants.
- Iteration 7 added model registration and model transition variants to `ChainCommand`, with matching
  events, and routed local synthetic linear training, block parent-state preparation, and `LocalTestnet`
  model updates through that command path.
- Iteration 8 made model registration duplicate-safe. `register_model` now returns an error for an existing
  model ID, and the `ChainCommand::RegisterModel` path propagates that instead of overwriting model state.
- Iteration 9 added command variants and events for account transfer and reward claim, then routed
  non-reference `apply_transaction` writes through `ChainCommand` instead of direct imperative helpers.
- Iteration 10 stopped validator remote tensor-fetch bookkeeping from persisting unchanged chain state.
  Runtime status still records fetch failures/successes, but snapshot and chain-state files are no longer
  rewritten when no consensus data changed.
- Iteration 11 moved faucet reward credits behind `ChainCommand::CreditReward`. The faucet now owns only
  drip eligibility and faucet balance, while the RPC claim path asks the chain engine to mutate reward state.
- Iteration 12 made `apply_transaction` return the `ChainEvent`s produced by its delegated `ChainCommand`,
  so public transaction writes no longer silently discard the typed mutation effects.
- Iteration 13 added a command/event wrapper for fraud challenge outcomes. Public challenge application now
  enters through `ChainCommand::ApplyChallengeOutcome`, and rejected/slashing outcomes emit typed events.
- Iteration 14 narrowed `RewardState`'s externally visible fields and added read accessors for balances,
  total balance, and treasury so runtime/RPC/testnet reporting no longer reaches through reward internals.
- Iteration 15 made `RewardState` balances and treasury visible only inside the `chain` module, while storage
  decodes rewards through an explicit constructor and encodes them through read accessors.
- Iteration 16 made reward crediting chain-private too, moving storage and telemetry fixtures through
  `ChainCommand::CreditReward` instead of calling `RewardState::credit` directly.
- Iteration 17 moved non-chain test fixture height/epoch writes behind a crate-test-only chain position
  helper, keeping those setup mutations out of scheduler/localnet/profile tests.
- Iteration 18 moved non-chain settled-receipt fixture inserts behind a crate-test-only helper, leaving
  settlement ownership inside `chain/*` instead of scattered storage/RPC/watcher tests.
- Iteration 19 moved malformed/orphan receipt fixture inserts in telemetry and watcher tests behind a
  crate-test-only `Chain` helper, making those consensus bypasses explicit test setup.

## Core Abstraction Correction: `Chain`, Not `LocalChain`

`LocalChain` is the wrong name for the core protocol state machine. It was a reasonable name when the
project was mostly a local harness, but it now leaks a deployment profile into the consensus layer. The
chain itself is not local. Local CPU, public testnet, and mainnet are profiles/configurations of the same
chain engine.

The target model should be:

```text
Chain owns consensus.
Profiles own environment policy.
Runtime owns process and network orchestration.
```

The core type should be named `Chain`. The local profile should be represented by configuration, not by a
separate chain abstraction.

Concrete target shape:

```rust
pub struct Chain {
    params: ChainParams,
    state: ChainState,
    blocks: Vec<TensorBlock>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChainProfile {
    LocalCpu,
    PublicTestnet,
    Mainnet,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainConfig {
    pub profile: ChainProfile,
    pub genesis: GenesisConfig,
    pub params: ChainParams,
    pub job_source: JobSourceConfig,
    pub reward_policy: RewardPolicy,
    pub network_policy: NetworkPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenesisConfig {
    pub chain_id: String,
    pub finalized_randomness: Hash,
    pub initial_miners: Vec<GenesisMiner>,
    pub initial_validators: Vec<GenesisValidator>,
    pub initial_accounts: Vec<GenesisAccount>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JobSourceConfig {
    SyntheticLocalCpu {
        matmul_shape: (usize, usize, usize),
        linear_training_shape: (usize, usize),
    },
    NetworkOnly,
    ExternalProgrammatic,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardPolicy {
    pub miner_reward_pool: u64,
    pub validator_reward_pool: u64,
    pub proposer_reward_bps: u64,
    pub treasury_reward_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkPolicy {
    pub require_libp2p: bool,
    pub allow_synthetic_jobs: bool,
    pub allow_local_block_production: bool,
}
```

The implementation should be a direct replacement, not a compatibility migration:

```rust
pub struct Chain {
    params: ChainParams,
    state: ChainState,
    blocks: Vec<TensorBlock>,
}
```

Do not add `pub type LocalChain = Chain`. Do not keep `LocalChain` as a deprecated alias. Rename the core
type and update call sites, tests, docs, and public exports in the same feature-sized refactor. If the
branch has not shipped, preserving a misleading compatibility name just keeps the wrong abstraction alive.

This rename is not cosmetic. It deletes a misleading concept. The current name encourages local-only
shortcuts to creep into consensus code. A `Chain` configured by `ChainProfile::LocalCpu` makes the boundary
clear: local CPU is a profile and deployment mode, not a different state machine.

Rules that should follow from this:

- `Chain` must not know whether it is running in Docker Compose, a public testnet, or mainnet.
- `local_cpu`, `public_testnet`, and `mainnet` must not fork block validation, receipt settlement, finality,
  codec behavior, or state roots.
- Synthetic local jobs are `JobSourceConfig`, not a separate chain type.
- Local rewards are `RewardPolicy`, not hardcoded constants in node/runtime glue.
- Local deployment readiness is evidence about a profile, not a protocol architecture.

## Priority Findings

### 1. Chain State Has No Real Encapsulation

`Chain` now hides its top-level fields from external crate users, but `ChainState` still exposes its
internals directly in `crates/tensor_vm/src/chain/state.rs`, and internal runtime modules can still bypass
the command boundary.

```rust
pub struct Chain {
    pub(crate) params: ChainParams,
    pub(crate) state: ChainState,
    pub(crate) blocks: Vec<TensorBlock>,
}
```

This defeats the purpose of `ChainCommand` and `ChainEngine`. Any caller can mutate jobs, receipts,
votes, rewards, finality, or height without going through validation.

Impact:

- Invariants in `chain/validation.rs`, `chain/receipts.rs`, `chain/settlement.rs`, and `chain/blocks.rs`
  are advisory rather than enforced.
- Runtime and tests can create states the chain engine would never admit.
- Future refactors will keep needing defensive cleanup because invalid states are representable everywhere.

Remaining fix:

- Move direct internal mutation of `Chain.state`, `Chain.blocks`, and `Chain.params` behind narrower helpers.
- Expose immutable views through `ChainView` or narrow accessor methods.
- Route production mutations through `ChainEngine::apply_command`.
- Move direct-state test setup into explicit `#[cfg(test)]` builders.

### 2. Dual Mutation APIs Make Events Optional

The command/event facade exists in `crates/tensor_vm/src/chain/engine.rs`, but the codebase still mutates
through imperative methods on `Chain`.

Remaining examples:

- runtime test setup and lower-level model tests still call direct mutation helpers in several places.
- `node.rs` and runtime paths call chain helpers directly.
- network block ingestion still prepares parent state and admits blocks directly so it can preserve
  pending/duplicate/invalid admission results.
- `apply_transaction` now routes non-reference writes through `ChainCommand`, returns command events, and
  rejects txpool-only reference submissions, but the public transaction surface still mixes immediate
  mutations with queued reference announcements.

This violates single responsibility and interface segregation: callers must know which mutation path emits
events, which one finalizes, and which one silently mutates.

Fix:

- Pick the command path as canonical.
- Keep imperative methods private to `chain/*` modules.
- Make public write paths return typed events/effects.
- Remove or implement transaction variants that currently lie about behavior.

### 3. Runtime Adapters Still Own Consensus Behavior

`crates/tensor_vm/src/main.rs` and `crates/tensor_vm/src/node.rs` contain runtime orchestration, network
ingest, role work, status writes, local production, and chain mutation glue.

The recent refactor moved some behavior toward chain-owned helpers, but the structural smell remains:
runtime code still knows too much about assignment, chain state shape, block publication, role counters,
and persistence.

Fix:

- Extract a `service/` or `node_runtime/` module:
  - runtime loop
  - miner worker
  - validator worker
  - proposer worker
  - status snapshot writer
- Keep `main.rs` as a thin binary entrypoint.
- Keep consensus decisions in `chain/*`, not runtime loops.

### 4. Finality And Block Admission Need A Harder Boundary

The code now has a typed `BlockAdmission` direction, which is good. The next step is making finality
fully vote-driven in every runtime path.

Target contract:

```text
valid block payload -> append block
signed block vote payloads -> finality
```

Anything that appends and finalizes in one hidden step should be treated as test-only or removed.

Fix:

- Keep `admit_block` append-only.
- Route all finality through `SubmitBlockVote`.
- Add block-vote gossip/RPC coverage as a first-class network event.
- Keep local auto-finalization only in clearly named pure test helpers.

### 5. Shell Scripts Encode Protocol Policy

`deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh` is a large shell policy engine. It verifies
protocol claims by grepping status strings, parsing JSON with `sed`, and duplicating service lists.

This is not just unpleasant. It is an architecture problem: readiness semantics live outside the Rust
types that produce the state.

Fix:

- Move verification policy into Rust, for example `tvmd local-cpu verify --json`.
- Let shell orchestrate Docker only.
- Make the checker consume typed JSON, not scattered `key=value` strings.
- Remove cargo test execution from deployment scripts; CI should own unit tests.

### 6. Status Surfaces Are Duplicated And Stringly

There are multiple overlapping status emitters:

- reference CLI output in `cli.rs`
- real service handlers in `main.rs`
- `role-runtime.status`
- `local-cpu-ready`
- `service status`
- checker expectations
- CLI integration tests

The same conceptual fields appear with different names, for example prefixed and unprefixed role fields.
This creates brittle tests and makes every new metric require edits across many files.

Fix:

- Introduce one typed `RuntimeStatusSnapshot` / `ServiceStatusV1`.
- Render text and JSON from the same struct.
- Add a schema version.
- Make tests parse the schema instead of checking substring inventories.

### 7. Duplicate Codecs Are A Drift Risk

`p2p.rs` and `storage.rs` both encode domain objects manually, including block payloads. The layouts are
similar enough to suggest shared intent, but separate enough to drift.

Current duplicated domains include:

- `TensorBlock`
- jobs
- receipts
- attestations
- block votes
- tensor payloads

Fix:

- Create a `codec/` module.
- Put canonical encode/decode for consensus payloads there.
- Make p2p and storage call the shared codec.
- Add golden, roundtrip, malformed, trailing-byte, and max-size tests.
- Keep storage-specific wrappers separate from payload encoding.

### 8. Manual Parsing Is Overused

Parsing appears in many forms:

- hand-rolled CLI parsing in `crates/tensor_vm/src/cli.rs`
- shell `grep`/`sed` JSON extraction
- `key=value` status parsing
- hand-written JSON extraction in RPC/tests
- CSV-like manifest parsing with `split(',')`
- whitespace transaction parsing

This is brittle and non-idiomatic Rust for structured data.

Fix:

- Use `clap` for CLI parsing with typed subcommands and `ValueEnum` profile/role values.
- Use `serde_json` for JSON.
- Use typed structs for CLI/RPC status output.
- Use the `csv` crate or a simpler line format that rejects ambiguous characters explicitly.
- Replace `stdout.contains(...)` tests with parsed assertions.

The CLI should not keep growing a giant string-slice match. Replace the current manual parser with a
`clap` command tree:

```rust
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "tvmd")]
pub struct TvmdCli {
    #[command(subcommand)]
    pub command: TvmdCommand,
}

#[derive(Subcommand, Debug)]
pub enum TvmdCommand {
    Miner(MinerCommand),
    Validator(ValidatorCommand),
    Proposer(ProposerCommand),
    Service(ServiceCommand),
    LocalTestnet(LocalTestnetCommand),
    LocalCpu(LocalCpuCommand),
    PublicEvidence(PublicEvidenceCommand),
    PublicTestnet(PublicTestnetCommand),
}

#[derive(Subcommand, Debug)]
pub enum ServiceCommand {
    Init(DataDirArgs),
    Peer(ServicePeerCommand),
    Readiness(ServiceReadinessArgs),
    Serve(ServiceServeArgs),
    Status(DataDirArgs),
    Block(ServiceBlockArgs),
}

#[derive(Args, Debug)]
pub struct DataDirArgs {
    #[arg(long)]
    pub data_dir: String,
}

#[derive(ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChainProfileArg {
    LocalCpu,
    PublicTestnet,
    Mainnet,
}
```

Then convert `TvmdCommand` into the internal command enum if a separate internal representation is still
useful. The parser should be generated by `clap`; validation and execution should live in typed handlers,
not in parser match arms.

### 9. `TvmError` Is Too Stringly

`TvmError::InvalidReceipt(&'static str)` carries many unrelated failures:

- bad signatures
- codec length errors
- duplicate votes
- unknown blocks
- malformed transactions
- state-root mismatches

This prevents callers from making reliable decisions without matching strings.

Fix:

- Split errors by domain:
  - `ChainError`
  - `BlockAdmissionError`
  - `CodecError`
  - `StorageError`
  - `RpcError`
- Keep `TvmError` as a top-level wrapper.
- Avoid string matching in retry/admission logic.

### 10. Error Paths Have Hidden Side Effects

`validation.rs::submit_attestation` can penalize a validator while returning `Err`.

That means `Err` does not imply no state change. This is surprising and makes retries dangerous.

Fix:

- Split validation from effects:
  - `validate_attestation(...) -> AttestationDecision`
  - `apply_attestation_decision(...)`
- If rejected attestations slash or penalize, return a typed event/effect.
- Make side-effecting rejection explicit in `ChainEvent`.

### 11. `Chain::clone()` Is Too Easy

`Chain` is deeply cloneable. That encourages speculative mutation by cloning the whole chain and
replacing it on success.

This is convenient but not scalable or conceptually clean.

Fix:

- Replace clone-based admission with dry-run validation or a small rollback journal.
- Avoid cloning `ChainState` to reconstruct parent views.
- Consider removing or narrowing `Clone` for `Chain` outside tests.

### 12. God Modules Hide Ownership

Several files are far past a healthy size boundary:

| File | Problem |
| --- | --- |
| `crates/tensor_vm/src/cli.rs` | CLI parsing, validation, public evidence, docs tests, string output |
| `crates/tensor_vm/src/testnet.rs` | testnet orchestration, manifests, public evidence, validation |
| `crates/tensor_vm/src/p2p.rs` | libp2p service, gossip, request-response, peer book, codecs, tests |
| `crates/tensor_vm/src/main.rs` | binary dispatch, runtime loop, role logic, status, network glue |
| `crates/tensor_vm/src/rpc.rs` | HTTP parsing, routing, explorer, websocket, chain reads |
| `crates/tensor_vm/src/storage.rs` | snapshots, block log, state codec, recovery |
| `crates/tensor_vm/src/node.rs` | network ingest, payload apply, pending queues, runtime counters |

Fix after boundaries are cleaner:

- `p2p/{service,codec,peer_book,request_response}.rs`
- `service/{runtime,roles,status}.rs`
- `rpc/{server,routes,explorer,websocket}.rs`
- `storage/{snapshot,block_log,chain_state,codec}.rs`
- `cli/{parse,reference,evidence,commands}.rs`

Do not split files first. Split after the canonical owners are clear.

## SOLID Review

### Single Responsibility

Weak in:

- `main.rs`
- `cli.rs`
- `p2p.rs`
- `rpc.rs`
- `storage.rs`
- `testnet.rs`

Strong in:

- `tensor.rs`
- `field.rs`
- `merkle.rs`
- parts of `chain/*`
- `miner.rs`
- `validator.rs`

Recommendation: keep core math and chain modules focused; extract operational code aggressively.

### Open/Closed

The system is not open for extension without editing many places. Adding a new payload/status field often
touches:

- API enum
- p2p codec
- lib re-exports
- node ingest
- main status strings
- checker script
- CLI tests
- compose artifact tests

Recommendation: route extensions through typed snapshots and shared codecs.

### Liskov Substitution

This is less relevant because there is little trait hierarchy. The notable issue is that the
`ChainEngine` trait promises a mutation boundary, but `Chain` exposes bypasses. Implementations can
not be substituted if callers rely on public fields.

Recommendation: make `ChainEngine` meaningful by hiding direct mutation.

### Interface Segregation

`lib.rs` re-exports too much. Consumers get chain, p2p, CLI, testnet, telemetry, and deployment-adjacent
types from one crate surface.

Recommendation:

- expose `tensor_vm::core`
- expose `tensor_vm::node`
- expose `tensor_vm::ops`
- or split into `tensor_vm_core` and `tensor_vm_node` later.

### Dependency Inversion

Runtime logic depends directly on concrete `RpcHttpServer`, `TensorVmLibp2pService`, `Chain`,
`NodeStore`, and status files.

Recommendation:

- introduce narrow traits for runtime dependencies:
  - `ChainMutator`
  - `TensorStore`
  - `NetworkPublisher`
  - `RuntimeStatusSink`
- use concrete types at the binary boundary.

## DRY Findings

### Duplicated Domain Logic

High-priority duplicates:

- block commit path in `blocks::produce` and `blocks::admit`
- p2p and storage payload codecs
- public endpoint validation across CLI/testnet code
- attestation validation in write path, quorum checks, and watcher logic
- receipt submission for TensorOp and LinearTrainingStep
- status string rendering across CLI/main/checker/tests
- identity seed reports in CLI/main
- CLI argument parsing and command descriptions in `cli.rs`
- JSON/key-value parsing in shell/tests/RPC

Fix one domain at a time. The highest leverage is shared status snapshot and shared codecs.

## Idiomatic Rust Findings

### Prefer Private Fields And Smart Constructors

Many domain structs have all-public fields. That is reasonable for plain data transfer, but not for
consensus state.

Make these private or crate-private:

- `Chain`
- `ChainState`
- possibly `TensorBlock`
- possibly `BlockVote`

Keep DTOs public only at serialization boundaries.

### Prefer Typed Results Over Strings

Replace:

```rust
Result<T, String>
InvalidReceipt(&'static str)
```

with typed errors that callers can match without string parsing.

### Prefer Shared Parsers Over Hand-Rolled Splits

Manual parsing is acceptable for small, consensus-critical binary codecs if centralized and tested. It is
not appropriate for every JSON/status/CSV surface.

### Avoid Deep Clone As Control Flow

Cloning a whole chain to attempt admission is easy but expensive and hides which changes are speculative.
Prefer dry-run validation or a journal.

### Avoid Massive Match Functions

`execute_reference_cli_command`, CLI parsing, and RPC route dispatch should be split into typed command
handlers.

## Tests Review

Strengths:

- high number of focused unit tests
- good coverage of malformed p2p payloads
- good coverage of chain validation edges
- integration tests exercise CLI flows

Weaknesses:

- too many substring tests
- huge inline test modules in already-large source files
- checker script tests assert script text rather than behavior
- no property/fuzz tests for manual codecs
- repeated doc/manifest tests across multiple files

Fix:

- add test support helpers in `crates/tensor_vm/tests/support`
- parse structured outputs instead of using `contains`
- move binary-runtime tests out of `main.rs`
- add proptest or table-driven malformed codec tests
- reduce duplicate manifest tests to one integration surface

## Recommended Refactor Order

1. Encapsulate `Chain` and `ChainState`.
2. Enforce `ChainEngine` / `ChainTransition` as the only production mutation path.
3. Split block append from finality everywhere.
4. Consolidate p2p/storage codecs.
5. Introduce typed status snapshots and JSON outputs.
6. Move local CPU checker policy into Rust.
7. Replace hand-rolled CLI parsing with `clap`.
8. Stop persisting chain state on read-only runtime activity. Read-only RPC was completed in Iteration 5,
   and validator remote tensor-fetch status bookkeeping was completed in Iteration 10.
9. Split `main.rs`, `cli.rs`, `p2p.rs`, `rpc.rs`, and `storage.rs` by ownership.
10. Replace stringly errors with typed domain errors.
11. Move large inline tests into focused module or integration test files.

## Positive Notes

The following pieces are directionally good and should be preserved:

- `ChainCommand` / `ChainEvent` is the right abstraction; it needs enforcement.
- `BlockAdmission` is the right shape; carry that pattern further.
- `Tensor`, `TxPool`, and `NodeRuntimeState` show better encapsulation than chain state.
- `tensor.rs`, `field.rs`, `merkle.rs`, `miner.rs`, and `validator.rs` are comparatively focused.
- The verification math is relatively isolated from I/O.
- Test density is high; the problem is test shape and duplication, not total lack of tests.

## Approval Bar For Future Changes

Do not accept future changes that:

- add consensus behavior to runtime adapters
- add more `key=value` fields without a typed snapshot owner
- add another codec for an existing domain type
- expose more mutable state publicly
- match consensus errors by string
- extend the shell checker as the protocol authority
- push an already-large file further past a clear split boundary

The codebase needs fewer branches in adapters and stronger ownership in the core.
