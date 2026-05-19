# Local Chain Production Readiness And Chain-Core Refactor Plan

This document records the current local-chain readiness gaps and the refactor path for making TensorVM's
local chain production-grade while keeping it local-only. It combines the local setup review with an
architecture plan for using one shared chain base across local, testnet, and mainnet profiles.

The target is not public infrastructure. The target is a real local chain where all Docker Compose
participants run the same protocol code paths that a public testnet or mainnet profile would use.

## Architecture Decision

TensorVM should have one chain base. Local CPU, public testnet, and mainnet must share the same deterministic
state-transition engine, validation rules, settlement rules, proposer selection, block application, storage
contract, and libp2p message handling. They may differ only through profile configuration and deployment
adapters.

Accepted profile differences:

```text
chain ID and genesis state
operator set and bootstrap peers
job source policy
block and epoch timing
reward and faucet policy
service exposure and authentication policy
evidence requirements
storage paths and retention policy
```

Rejected differences:

```text
separate local-only chain transition logic
simulation shortcuts in production paths
in-memory propagation instead of the shared node event path
optional libp2p for any counted operator
testnet/mainnet-only validation or settlement code
role processes that bypass the chain engine
```

The repository boundary should stay simple: protocol, runtime, storage, networking, and local deployment
support live in the `tensor_vm` crate and deploy tree; non-protocol experiments, studies, and exploratory
tools live in the `experiments` crate. A feature can be experimental, but it should not require production
chain code to import experiment-only modules.

## Scope

Local production-ready means:

```text
CPU-only default execution remains supported
all 10 miners and 5 validators are real long-running participants
jobs, receipts, attestations, blocks, votes, and tensor fetches move through libp2p/RPC boundaries
all operators persist and sync chain state
the explorer reads live chain data from the node API
restart and rollback behavior is checked locally
the implementation remains explicitly non-public evidence
```

It does not mean:

```text
CUDA requirement
public DNS or TLS
systemd/nginx deployment
external independent operators
7-day public-run evidence
mainnet security claims
```

## Current State

The local bundle is useful and should remain the first operational target:

- `deploy/tensorvm/local-cpu/docker-compose.yml` starts 10 miner containers, 5 validator containers, and
  the standalone explorer.
- Each counted operator has a stable operator ID, stable libp2p identity seed, distinct volume, and
  mandatory libp2p readiness check.
- `miner-00` exposes local RPC, explorer data, faucet, telemetry, and the host-facing WebSocket endpoint.
- The current live producer keeps `/chain/head` advancing past the seeded two-block baseline.
- `check-local-testnet.sh` now fails if live jobs, receipts, settled receipts, height, and block count do
  not advance.
- Every operator now starts from the same deterministic local CPU seed and exposes durable node-store status
  through `tvmd service status`.
- The checker fails unless all 15 operator node stores advance past the seed, report role-specific status
  and live chain counters, report the same first live finalized block hash, and return the same finalized
  common-head block hash through `tvmd service block` before and after restart checks.
- `check-restart-continuity.sh` captures pre/post peer IDs, heights, block counts, and finalized common
  heads around actual Compose restarts, and fails unless restarted services keep identity, avoid rollback,
  preserve the pre-restart finalized common head, and continue finalizing blocks.

That is enough for a useful local demonstration. It is not enough for a production-grade local chain.

## Refactor Progress

The first chain-core cleanup slices are already in the tree:

- `LocalChain` is exposed through a profile-neutral `Chain` alias and `ChainEngine` command/event facade.
- `NodeStore` implements a `ChainStore` boundary for loading and persisting chain state.
- `ChainProfile` and `NodeConfig` let local CPU, public testnet, and future mainnet construct the same
  deterministic chain engine from profile values.
- Local CPU synthetic production moved into the `tensor_vm` library instead of remaining private binary code.
- `JobSource` and `SyntheticLocalJobSource` separate deterministic local job generation from scheduler and
  block-production code.

These are foundation pieces, not completion. The local runtime still needs role-owned loops and network-visible
state transitions before it satisfies the local CPU spec as a production-grade local chain.

## Highest-Priority Gaps

### 1. Local Production Is Still Single-Process

Current live production still runs inside `miner-00`'s service loop. It now calls a shared library adapter,
but that adapter still directly creates CPU miner objects, validator objects, receipts, attestations,
settlement, block production, and finality votes against one `Chain`.

This conflicts with the local CPU spec and MVP Gate 0 language that rejects simulations, direct in-memory
propagation, local-only networking shims, and single-participant shortcuts.

Required fix:

- Move synthetic job generation into a `JobSource`.
- Broadcast jobs over the same network path used by every profile.
- Have miner containers receive jobs, execute them, serve tensor data, and submit receipts.
- Have validator containers receive assignments, fetch tensor data, validate, and submit attestations.
- Have proposers collect network-visible state before producing blocks.

### 2. Miner And Validator Containers Are Readiness Shells

`tvmd miner start` and `tvmd validator start` currently prove local readiness, then each container execs
`tvmd service serve`. They do not run independent role loops.

Required fix:

- Add long-running commands:
  - `tvmd miner run`
  - `tvmd validator run`
  - `tvmd proposer run` or `tvmd node run --role <role>`
- Make each role loop own only its role responsibilities.
- Keep readiness commands as preflight checks, not the runtime.

### 3. Libp2p Runs But Does Not Drive Chain State

The libp2p control plane subscribes to TensorVM topics and supports request-response protocols, but
production state changes still happen through local memory in the gateway process.

Required fix:

- Implement a node event loop that ingests libp2p messages:
  - `NewJob`
  - `NewReceipt`
  - `NewAttestation`
  - `NewBlock`
  - `PeerInfo`
- Validate message payloads before applying them.
- Persist accepted events through the shared chain engine.
- Publish local events back out through libp2p.

### 4. Non-Bootstrap Operators Do Not Prove Chain Sync

The checker validates that all operators are running and libp2p-ready, and now checks every node store for
role status, live chain counters, the same first live finalized block hash, and the same finalized
common-head block hash through `tvmd service block`. It still does not prove live state was propagated
through the network or that every operator is executing a distinct role loop.

Required fix:

- Extend `tvmd service status` or the local node API to include real connected peer count and role-specific
  work counters sourced from role loops.
- Move the convergence assertion from deterministic same-seed first-live/common-head equality to the
  shared network event path.
- The checker must eventually fail unless all 15 operators converge on the same latest finalized head within
  a bounded time.

### 5. Restart Gate Needs Broader Continuity Assertions

The local spec requires restarted operators to reuse durable state and libp2p identity, rejoin the network,
and avoid chain rollback. The current restart-continuity script now records pre-restart and post-restart
continuity for selected restarts, including `miner-03`, `validator-02`, and `miner-00`.

Required fix:

- Extend continuity evidence from the selected restart targets to every counted operator over a rolling
  restart matrix.
- Add a store checksum or block-log root before and after restart, not only peer ID, height, block count,
  latest hash, and finalized common-head preservation.
- Keep checking that peer IDs are unchanged, height and block count do not decrease, the pre-restart
  finalized common head is preserved, and blocks continue advancing after restart.

### 6. Live Primitive Coverage Needs Stronger Evidence

The seed covers both TensorOp and LinearTrainingStep. Live post-startup production now uses
`SyntheticLocalJobSource` for both matmul and LinearTrainingStep jobs, and the checker can require
`model_count` to advance past the seeded baseline as proof that a live training step settled.

Required fix:

- Keep the deterministic local `JobSource` emitting both:
  - TensorOp matmul jobs
  - LinearTrainingStep jobs
- Add per-block or per-receipt checker evidence that names both live post-startup primitive types, not
  just aggregate model-count advancement.

### 7. The Checker Does Not Prove All Local-Spec Acceptance Items

The local spec requires validator attestations, rewards, data availability, telemetry, and tensor-server
availability evidence. The checker currently verifies some seed strings and aggregate live counters.

Required fix:

- Query live receipt details and prove at least one new post-startup receipt has validator attestations.
- Query miner and validator rewards after live jobs, not only from seed output.
- Perform a live tensor row/chunk/opening fetch through the local tensor-server path.
- Assert telemetry counters advance with the live chain.
- Record exact observed values in checker output.

## Shared Chain-Core Refactor

The core architectural goal is:

```text
local, testnet, and mainnet use the same deterministic chain engine
```

The profiles should differ by configuration, adapters, and launch topology, not by separate chain logic.

### Current Coupling To Reduce

`LocalChain` currently contains state, parameters, blocks, registration, transaction application, receipt
submission, attestation validation, settlement, rewards, proposer selection, block production, and finality
helpers in one type.

That is practical for a reference core, but it makes it easy for local/testnet helpers to bypass real
runtime boundaries.

### Target Module Shape

Refactor toward these boundaries:

```text
chain::state
  ChainState, ChainParams, account/miner/validator/job/receipt/block state types

chain::engine
  ChainEngine, deterministic state transitions, command application, event emission

chain::validation
  receipt, attestation, block-vote, and transaction validation

chain::settlement
  epoch settlement, reward accounting, model-state transition settlement

chain::proposer
  proposer eligibility and settled-prior-epoch TensorWork selection

node::runtime
  event loop joining network, store, txpool, chain engine, clock, and role services

node::roles
  miner, validator, proposer, watcher

node::profiles
  local, testnet, mainnet runtime profiles

network
  libp2p adapter, message codec, gossip/request-response routing

storage
  ChainStore trait, NodeStore implementation, recovery and consistency checks
```

### Shared Profile Model

Use a single profile type instead of environment-specific branches:

```rust
pub enum ChainProfile {
    Local(LocalProfile),
    Testnet(TestnetProfile),
    Mainnet(MainnetProfile),
}

pub struct NodeConfig {
    pub chain: ChainParams,
    pub profile: ChainProfile,
    pub role: NodeRole,
    pub network: NetworkConfig,
    pub storage: StorageConfig,
}
```

Local/testnet/mainnet should select different values for:

- genesis state
- chain ID
- job source policy
- block interval
- peer discovery/bootstrap
- auth/exposure policy
- reward caps
- persistence paths
- telemetry/evidence requirements

They should not select different state-transition code.

### Engine API Direction

The chain engine should expose a small command/event boundary:

```rust
pub enum ChainCommand {
    RegisterMiner(...),
    RegisterValidator(...),
    SubmitJob(JobState),
    SubmitReceipt(ReceiptState),
    SubmitAttestation(ValidatorAttestation),
    SubmitBlock(TensorBlock),
    SubmitBlockVote(BlockVote),
    SettleEpoch,
}

pub enum ChainEvent {
    JobAccepted(Hash),
    ReceiptAccepted(Hash),
    AttestationAccepted(Hash),
    ReceiptSettled(Hash),
    BlockAccepted(Hash),
    BlockFinalized(Hash),
    RewardCredited { address: Address, amount: u64 },
}

pub trait ChainEngine {
    fn apply(&mut self, command: ChainCommand) -> Result<Vec<ChainEvent>>;
    fn view(&self) -> &ChainState;
}
```

This makes tests, local Compose, public testnet, and future mainnet run the same transition path while
still allowing different runtimes to drive it.

### Traits To Introduce

Keep traits narrow and role-specific:

```rust
pub trait ChainStore {
    fn load_chain(&self) -> Result<ChainSnapshot>;
    fn persist_events(&self, events: &[ChainEvent]) -> Result<()>;
    fn persist_snapshot(&self, state: &ChainState) -> Result<()>;
}

pub trait Network {
    fn publish(&self, message: P2pMessage) -> Result<()>;
    fn recv(&mut self) -> Result<NetworkEvent>;
    fn request(&self, peer: PeerId, request: P2pMessage) -> Result<P2pMessage>;
}

pub trait JobSource {
    fn next_job(&mut self, state: &ChainState) -> Option<JobState>;
}

pub trait MinerExecutor {
    fn execute(&mut self, job: &JobState, context: &ExecutionContext) -> Result<ReceiptBundle>;
}

pub trait ReceiptVerifier {
    fn verify(&self, receipt: &ReceiptState, context: &ValidationContext) -> Result<ValidatorAttestation>;
}
```

Concrete implementations:

- `NodeStore` implements `ChainStore`.
- `Libp2pNetwork` implements `Network`.
- `SyntheticLocalJobSource` implements `JobSource`.
- `CpuReferenceMiner` implements `MinerExecutor`.
- `TensorVmReceiptVerifier` implements `ReceiptVerifier`.

### SOLID/Rust Guidelines

Use SOLID as a practical constraint, not as ceremony:

- Single responsibility: chain transition logic should not know Docker, HTTP, CLI, or libp2p details.
- Open/closed: adding `MainnetProfile` should not require editing settlement or validation internals.
- Liskov substitution: tests should run against the same `ChainEngine` trait as local Compose.
- Interface segregation: miners should not depend on proposer APIs; validators should not depend on faucet APIs.
- Dependency inversion: `node::runtime` depends on `Network` and `ChainStore` traits, not concrete libp2p or file-store types.

Rust-specific practices:

- Prefer explicit domain types over `String`/`usize` plumbing at module boundaries.
- Keep `Result<T, TvmError>` for fallible domain paths and avoid stringly errors in core logic.
- Make command application deterministic and side-effect-free except through returned events.
- Keep IO at adapter edges: storage, network, CLI, RPC.
- Avoid large `impl` blocks that mix registration, execution, settlement, and API concerns.
- Prefer small structs with explicit ownership over shared mutable globals.
- Use `#[cfg(test)]` helpers only for tests; do not let production code call testnet-only shortcuts.

## Role Runtime Design

### Miner Loop

Responsibilities:

```text
subscribe to jobs
check assignment
execute with CPU reference backend
serve tensor rows/chunks/openings
submit receipts
gossip receipt announcements
track local work metrics
```

### Validator Loop

Responsibilities:

```text
subscribe to jobs and receipts
check validation assignment
request tensor data from assigned miner
verify TensorOp and LinearTrainingStep receipts
submit attestations
gossip attestation announcements
vote on valid blocks
track validation metrics
```

### Proposer Loop

Responsibilities:

```text
watch settled receipts and prior-epoch TensorWork
select eligible proposer with finalized randomness
assemble blocks from accepted state
publish blocks
collect block votes
track finality metrics
```

In local mode, `miner-00` may be the first proposer for simplicity, but it must still consume network-visible
jobs, receipts, attestations, and votes.

## Proposed Implementation Phases

### Phase 1: Document And Harden The Gate

- Add this document.
- Update the local checker to emit exact live counters.
- Update `coverage_matrix.md` so it describes live post-startup jobs, not only seeded state.
- Add checker assertions for live rewards, live attestations, live tensor data fetch, and all-operator
  finalized-head convergence.

Status: partially complete. The document exists and the checker gates live post-startup height, blocks,
jobs, model-count advancement, attestation-count growth, reward-balance growth, receipts, and settled
receipts, per-receipt validator-attestation details, live tensor descriptor/row/chunk/opening fetches, all
15 operator node stores reporting role status, live chain counters, the same first live finalized block
hash, and the same finalized common-head block hash through `tvmd service block`. The restart-continuity
script also captures pre/post peer IDs, heights, block counts, and finalized common heads for the selected
restart gates. Latest-head convergence via the shared network event path still needs hard checker
assertions.

### Phase 2: Extract Chain Engine Boundaries

- Rename `LocalChain` to a profile-neutral `Chain` or wrap it behind `ChainEngine`.
- Move validation, settlement, proposer selection, and state views into separate modules.
- Preserve all existing behavior and tests.
- Keep `LocalChain` as a compatibility type alias temporarily if needed.

Status: started. `Chain`, `ChainEngine`, `ChainCommand`, and `ChainEvent` exist. Validation, settlement,
proposer selection, and state views still live mostly in one large `chain.rs` implementation.

### Phase 3: Add Role Loops Without Changing Consensus Semantics

- Add long-running miner, validator, and proposer/node commands.
- Initially run them against the existing RPC endpoints.
- Then move gossip/request-response ingestion into the node runtime.

Status: not complete. Miner and validator commands are readiness paths today, not independent long-running
role runtimes.

### Phase 4: Make Compose Participants Actually Participate

- `miner-*` containers run miner role loops.
- `validator-*` containers run validator role loops.
- `miner-00` runs gateway/proposer duties, but no longer creates all receipts and attestations locally.
- The checker requires all operators to converge on the same finalized head.

### Phase 5: Shared Profiles

- Introduce `NodeConfig` and `ChainProfile`.
- Express local, testnet, and future mainnet as config profiles.
- Remove profile-specific chain transition branches.
- Ensure all profile tests instantiate the same engine.

Status: started. `ChainProfile` and `NodeConfig` exist and tests prove all profiles build the same engine.
The runtime still needs to consume those profiles end to end.

### Phase 6: Restart And Recovery

- Restart miner, validator, and proposer/gateway roles independently.
- Verify no rollback.
- Verify catch-up from persisted block log and peer state.
- Verify block production continues after restart.

Status: partially complete. `check-restart-continuity.sh` covers selected miner, validator, and gateway
restarts and proves stable libp2p peer IDs, nondecreasing height/block count, preservation of the
pre-restart finalized common head on every operator, and continued finalization. It still needs a rolling
all-operator restart matrix and store-checksum or block-log-root evidence.

## Local Production-Ready Acceptance Gate

The local chain should not be called production-ready until this command sequence passes:

```bash
cargo test -p tensor_vm local_testnet --release
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml build
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
deploy/tensorvm/local-cpu/scripts/check-restart-continuity.sh miner-03 validator-02
deploy/tensorvm/local-cpu/scripts/check-restart-continuity.sh miner-00
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml down -v
```

And the checker must prove:

```text
all 15 counted operators are running real role loops
all 15 operators have stable identities after restart
all 15 operators converge on the same finalized head
blocks continue after restarts
jobs are delivered through libp2p or the shared node event path
receipts are produced by miner containers
attestations are produced by validator containers
blocks are produced from network-visible receipts and attestations
TensorOp and LinearTrainingStep live jobs both settle after startup
tensor rows/chunks/openings are fetched through the local tensor-server path
live rewards accrue to miners and validators
telemetry reflects live post-startup work
local evidence remains explicitly non-public
```

## Recommended Next Commit Sequence

Keep this incremental:

1. Strengthen the checker from deterministic common-head convergence to latest-head convergence through
   the shared network event path.
2. Split `chain.rs` into state, engine, validation, settlement, and proposer modules while preserving the
   `ChainEngine` facade.
3. Add role-loop commands while keeping current tests green.
4. Wire miner receipt production through role processes.
5. Wire validator attestation production through role processes.
6. Wire proposer/block production through network-visible state.
7. Make `SyntheticLocalJobSource` profile-configured and expose per-block evidence for both live primitive
   types after startup.
8. Expand restart continuity checks from `miner-00`, `miner-03`, and `validator-02` to a rolling
   all-operator restart matrix with store-root evidence.

This sequence keeps the local chain usable at every step while moving it toward the same base runtime that
testnet and mainnet profiles should use.
