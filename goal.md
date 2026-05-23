# Goal Operating Contract

Read `docs/tensorvm/codex_5_5_local_chain_workflow.md`, `docs/tensorvm/mvp_spec.md`, and `docs/tensorvm/local_chain_production_readiness.md` fully before editing. Maintain `docs/tensorvm/local_chain_production_exec_plan.md` as the source of truth for progress, decisions, validation commands, and blockers.

## Canonical Architecture Override

The canonical TensorVM architecture is useful-verification PoW with deterministic settled-receipt blockspace. Treat this as the current MVP design, not as an optional v2 side path.

Hard rules:

- Replace v1 behavior outright. Do not preserve backward compatibility with TensorWork-weighted proposer selection, job-rooted blocks, or a separate miner proposer role.
- Do not add compatibility shims, legacy aliases, dual block formats, migration adapters, or runtime branches that keep v1 consensus alive.
- Do not name structs, enums, modules, fields, commands, or docs with `V2`, `v2`, `New`, `Legacy`, or `Compat` just because they implement the current design.
- Use canonical names for the current design, for example `TensorBlock`, `Blockspace`, `SettledReceipt`, `UsefulVerificationPow`, `ChecksRoot`, and `ValidatorProposer`.
- If existing code has v1 names or behavior, rename or replace it directly when touched. Update tests and docs to the new canonical model instead of layering translation code.
- If docs conflict, `docs/tensorvm/mvp_spec.md` wins over older readiness text. Update stale readiness text as part of the feature instead of following it.
- Miners produce work receipts and serve tensor data. Validators verify receipts, build `checks_root`, perform useful-verification PoW, propose blocks, and sign BFT finality.
- TensorWork is only for miner rewards, blockspace capacity, telemetry, and concentration analysis. It must not select proposers.

## Interprocess Node Boundary Override

`tvmd` is a process launcher, operator CLI, and node adapter. It is not allowed to be the hidden consensus orchestrator.

Hard rules:

- Counted miners and validators must be separate long-running node processes with separate durable state, libp2p identities, endpoints, and role loops.
- Jobs, receipts, attestations, tensor fetches, blocks, checks roots, PoW evidence, and finality votes must cross process boundaries through libp2p or node RPC before affecting another node.
- The shared chain engine may be called directly inside one node after that node validates a local or inbound event. It must not be used by one `tvmd` service loop to mutate multiple counted roles in memory.
- Replace `tvmd` paths that synthesize jobs, receipts, attestations, blocks, votes, or role counters for other operators. Do not preserve them behind compatibility flags.
- Do not count single-process helpers, deterministic replay, service-owned local producer loops, or in-memory propagation as local-chain readiness evidence.
- `tvmd miner run` owns only miner behavior. `tvmd validator run` owns validator verification, useful-verification PoW, block proposal, and finality voting. There is no separate miner proposer process.
- `tvmd service serve` may expose local node APIs and start one configured node role. It must not perform multi-role orchestration.
- Tests for pure chain state transitions may call the chain engine directly. Acceptance tests for local production readiness must prove interprocess libp2p/RPC behavior.

## Architecture Shortcut Ban

Hard rules:

- Do not gate inbound P2P ingest on `block_interval`, `local_producer`, profile synthetic jobs, local block production, or RPC serving mode.
- Do not let producer capability disable inbound sync. Producer policy controls outbound block creation only.
- Do not add settlement, model transitions, proposer selection, block validation, block-vote synthesis, reward allocation, or finality in `main.rs`, `node.rs`, p2p glue, RPC glue, checkers, deployment scripts, or other adapters.
- Do not synthesize validator votes except inside clearly named pure test helpers that cannot be reached by runtime code.
- Do not classify consensus outcomes by matching error strings. Add typed outcomes or typed error variants at the chain boundary.
- Do not add consensus transaction variants that do not mutate canonical chain state or explicitly queue into a block body.
- Do not add another codec for `TensorBlock`, jobs, receipts, attestations, block votes, tensors, or consensus payloads without a shared-codec plan and parity tests.
- Do not add unbounded length-prefixed wire reads. Bound before allocation.
- Do not add status/checker fields by copy-pasting format strings. Add or extend a typed status snapshot owner.
- Do not count shell assertions, deterministic replay, local single-process helper state, hardcoded booleans, or checker-only policy as readiness evidence.

Before implementing any feature that touches consensus, P2P, runtime, storage, status, or checker code, answer these in the implementation checkpoint:

```text
Canonical owner:
Adapter callers:
Old shortcut being removed:
Regression test that proves the shortcut is gone:
Behavior with local synthetic block production disabled:
Behavior for producer and non-producer roles:
Structured evidence source:
Finality source:
Wire-size and codec boundary:
```

Every verifier review for these areas must challenge whether the change moved logic into the canonical layer or merely added another adapter branch.

Default to **feature-sized iterations**, not tiny one-function slices. A feature-sized iteration should deliver one coherent readiness capability end to end: production code, tests, checker/docs evidence, and a commit. Only shrink to a smaller slice when the feature crosses unrelated ownership boundaries, the verifier flags high risk, or targeted validation is failing.

Good feature-sized iterations:

- Useful-verification PoW block validity: state types, command path, block checks, focused tests, docs status.
- Settled-receipt blockspace: pool state, deterministic selection, caps, duplicate/spent handling, tests.
- Receipt payload ingestion: p2p decode, runtime queue, `ChainCommand::SubmitReceipt`, counters, checker assertion.
- Validator attestation payload ingestion: p2p decode, runtime apply path, status counters, checker assertion.
- Role-owned miner loop: job subscription, execution, receipt submission, tensor serving, role tests.

Bad iterations:

- Rename one field and stop.
- Add a doc TODO without code.
- Touch every runtime module without a single acceptance capability.
- Run only formatting and commit.

For every feature iteration, write this checkpoint before edits:

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

## Parallelization Rule

Before implementation, launch subagents in parallel and use their results to divide the feature into workstreams:

- `readiness-mapper`: map the target capability to readiness requirements and current gaps.
- `tensorvm-codebase-explorer`: explore implementation paths, symbols, and coupling.
- `tensorvm-test-coverage-explorer`: find existing tests and missing behavior coverage.
- Optional second `tensorvm-codebase-explorer`: focus on checker scripts, Docker, or p2p if the feature spans those areas.

During implementation, parallelize as much as the tooling safely allows:

- Run read-only explorers while the parent plans the implementation boundary.
- Run test discovery in parallel with code-path discovery.
- Use separate implementation subagents or worktrees for independent development workstreams when they will not edit the same files.
- Use `tensorvm-test-runner` for noisy or long validation while the parent reviews the diff.
- Use `tensorvm-verifier` before commit to challenge the whole feature, not just the last small edit.
- Use `tensorvm-goal-supervisor` before resuming after pauses, after several iterations, or whenever scope starts shrinking into busywork.

Do not parallelize two writers against the same files in the same worktree. If parallel implementation would collide, keep the parent as the single writer and use subagents for read-only exploration, test planning, and verification.

## Parallel Development Rule

Actual code development may be parallelized when the feature can be split into independent workstreams with clear file ownership.

Before launching writer subagents, write a workstream map:

```text
Feature:
Integrator/merge owner:

Workstream A:
Owner/subagent:
Files owned:
Allowed edits:
Forbidden files:
Validation:

Workstream B:
Owner/subagent:
Files owned:
Allowed edits:
Forbidden files:
Validation:

Merge order:
Final integrated validation:
```

Use parallel writer subagents for work like:

- chain/state/API boundary while another subagent updates checker-script evidence
- p2p message codec while another subagent writes tests for existing chain validation
- storage recovery tests while another subagent updates docs/status
- explorer/API read surfaces while another subagent works on local checker assertions

Do not use parallel writer subagents for work like:

- two agents editing `crates/tensor_vm/src/main.rs`
- two agents editing the same chain module
- one agent refactoring types while another depends on those unstable type names
- Docker Compose lifecycle work against the same project name
- commits from subagents

When parallel writers are used:

1. Give each writer a narrow ownership contract and forbidden-file list.
2. Prefer isolated worktrees or branches for writer subagents.
3. Require each writer to report changed files, tests run, and known risks.
4. The parent/integrator reviews each diff before merging.
5. The parent/integrator resolves conflicts and runs the final integrated validation.
6. Only the parent/integrator commits.

If workstreams collide, stop parallel writing and switch those subagents to read-only support.

## Slice Size Rule

Prefer the largest slice that can still be reviewed as one coherent feature. A slice is too small if it cannot produce observable behavior or update a meaningful acceptance gate. A slice is too large if it mixes unrelated readiness capabilities or cannot be validated before commit.

The iteration should usually include:

```text
1 production capability
1 focused test cluster
1 checker/docs evidence update when applicable
1 targeted validation run
1 commit
```

If a feature needs multiple commits, keep the commits feature-subdivided, not microscopic:

```text
commit 1: data/state/API boundary
commit 2: runtime/network integration
commit 3: checker/docs evidence
```

Do not stop after commit 1 if the goal feature needs commits 2 and 3 and the targeted validation is still incomplete. Continue until the feature capability is done or a real blocker is documented.

## Commit And Push After Every Iteration

Every successful feature iteration must end with a git commit and push. Do not start the next feature iteration until the current iteration has either:

- a commit hash and pushed branch/remote recorded in `docs/tensorvm/local_chain_production_exec_plan.md`, or
- an explicit blocker recorded with the exact failing command/output and reason commit or push was not completed.

Before committing:

```text
1. Run the targeted validation for the feature.
2. Run `tensorvm-verifier` against the integrated diff.
3. Review `git status --short` and `git diff`.
4. Update the exec plan with validation evidence.
5. Compact the exec plan if required by the compaction rule.
6. Commit only the files related to the iteration.
7. Push the commit to the configured upstream branch.
8. Record the commit hash, remote, branch, and push result in the exec plan.
```

Commit/push rule: commit and push after every successful iteration, but never commit a known-broken targeted gate. If a full Docker gate is environmentally blocked, document exact command/output in the exec plan before committing and pushing the narrower passing slice. Only the parent/integrator commits and pushes; subagents must not commit or push.

If push fails because no upstream exists, credentials are unavailable, the network is blocked, or policy forbids publishing the branch, record the exact blocker and do not start the next feature iteration until the user resolves it or explicitly approves continuing without push.

## Exec Plan Compaction Rule

Keep `docs/tensorvm/local_chain_production_exec_plan.md` useful as durable state, not as an ever-growing transcript. Compact it after every feature-sized iteration, after every 3 commits, or whenever it exceeds roughly 300 lines.

The exec plan should keep:

- Current goal and active feature capability.
- Current blocker list.
- Current readiness matrix with status, evidence path, and next action.
- Last 2 feature iterations in detail.
- Validation evidence for the latest successful commit and any current blocker.
- Decision log entries that still affect future implementation.
- Compact archive summaries for older iterations.

The exec plan should remove or compress:

- Full terminal transcripts after the pass/fail result and key error lines are recorded.
- Repeated command lists already captured in `goal.md` or the readiness doc.
- Stale file lists from completed iterations.
- Old subagent chatter once decisions and evidence have been summarized.
- Superseded plans that no longer affect future work.

Use this structure:

```text
# Local Chain Production Execution Plan

## Current State
- Active feature:
- Current status:
- Current blockers:
- Next action:

## Readiness Matrix
| Capability | Status | Evidence | Next action |

## Active Feature Iteration
<full current checkpoint, workstream map, validation target>

## Recent Iterations
<last 1-2 completed feature iterations with concise evidence>

## Decision Log
<durable decisions only>

## Validation Evidence
<latest command results and current blocker outputs only>

## Archive
<one-paragraph summaries of older completed iterations with commit hashes>
```

When compacting, preserve facts, commands, evidence, blockers, and commit hashes. Do not delete unresolved blockers or decisions that still constrain the implementation. If unsure whether to delete something, summarize it in the archive instead of keeping the full text.

## Definition Of Done

- Every readiness gap in `docs/tensorvm/local_chain_production_readiness.md` is implemented or explicitly reclassified with rationale.
- v1 consensus assumptions are removed or rewritten wherever they affect active code, tests, checkers, or current docs.
- Role-owned miner and validator paths do not bypass the shared chain engine; block proposal is validator-owned useful-verification PoW, not a separate miner proposer path.
- `tvmd` no longer performs hidden multi-role consensus orchestration; counted operators interact through interprocess libp2p/RPC boundaries.
- Libp2p or the shared node event path drives jobs, receipts, attestations, and blocks.
- Blocks are built from deterministic settled-receipt blockspace and validated through `checks_root`, useful-verification PoW, and BFT finality.
- The local checker proves live post-startup receipt, attestation, reward, tensor fetch, telemetry, restart, and all-operator convergence evidence.
- Unit/integration tests cover changed chain/runtime/storage/network behavior.
- The full local acceptance gate from the readiness doc passes or any environmental blocker is documented with exact failing command/output.
