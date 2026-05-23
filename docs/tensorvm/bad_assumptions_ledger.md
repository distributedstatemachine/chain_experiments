# TensorVM Bad Assumptions Ledger

Status: documentation-only risk ledger compiled from the current worktree.

Purpose: record assumptions that would make TensorVM soundness claims misleading if left implicit. This is
not a code change and not a mechanized proof. It is a guardrail for future specs, proofs, release notes, and
implementation work.

## Severity Legend

| Severity | Meaning |
| --- | --- |
| Critical | Makes the reviewed MVP consensus claim false if asserted today. |
| High | Can make verifier, reward, or availability claims materially misleading. |
| Medium | Can produce wrong expectations or weak local evidence if not qualified. |

## Ledger

| ID | Bad Assumption | Severity | Current Evidence | Why It Is Unsound | Allowed Claim Today | Proof/Implementation Gate |
| --- | --- | --- | --- | --- | --- | --- |
| BA-001 | "The current chain already implements useful-verification PoW." | Critical | Current `TensorBlock` has `job_root`, `receipt_root`, and `randomness`, but no `settled_receipt_set_root`, `checks_root`, `difficulty_target`, or `nonce`. `chain::blocks::produce` has no PoW predicate. | The reviewed v2 MVP requires validators to verify canonical settled-receipt blockspace and mine over the verification commitment. The current block object cannot express that theorem. | The current chain implements local reference block production, not v2 useful-verification PoW. | Add v2 block fields, canonical receipt selector, block-level checks root, PoW predicate, and finality validation over that predicate. |
| BA-002 | "Settled TensorWork proposer selection is close enough to the reviewed MVP." | Critical | `chain::proposer` selects miners by `settled_tensor_work` when total work is nonzero. | The reviewed spec explicitly says TensorWork no longer selects proposers. This is a different consensus resource. | Settled TensorWork proposer selection is superseded reference behavior. | Replace normal block production with registered-validator useful-verification PoW; keep TensorWork for miner rewards/blockspace metrics only. |
| BA-003 | "Finality implies useful-PoW block validity." | Critical | `submit_block_vote` checks validator stake, vote signature, known block hash, and duplicates. | Stake signatures over a known block hash do not prove the block used canonical receipt blockspace, recomputed checks, or met PoW target. | Current finality is stake-threshold finality for current reference blocks. | Require `validate_block_v2` before accepting votes or counting finality. |
| BA-004 | "A receipt map root is canonical blockspace." | Critical | Current blocks commit `receipt_root` over the whole receipt map. | A global map root does not define which settled receipts are eligible, selected, spent, expired, or carried over under caps. | Receipt roots commit state content, not deterministic v2 blockspace. | Add settled-receipt pool metadata, deterministic ordering, byte/TWU/count caps, spent/carry-over state, and root over the selected set. |
| BA-005 | "Per-receipt checks roots prove the block proposer verified work." | Critical | Attestations contain `checks_root`, but current blocks do not commit to an aggregate block-level checks root. | A proposer can produce the current block without proving it verified the canonical receipt set. | Per-receipt checks roots are validator attestation evidence only. | Define check leaves, aggregate `checks_root`, recomputation rules, and challenge openings. |
| BA-006 | "Freivalds proves every output cell." | High | `full_freivalds` is randomized and returns a boolean from sampled vectors. | Freivalds is probabilistic; false acceptance is bounded, not impossible. | Freivalds bounds false acceptance under hidden uniform challenges. | Mechanize Freivalds soundness and state sampling/randomness assumptions. |
| BA-007 | "Row sampling is good enough for block eligibility." | High | `row_sample_detection_probability` shows sparse-row detection can be low. | Sparse corruptions can evade small samples with high probability. | Row sampling is audit telemetry unless explicit bounds meet the target. | Require full-output Freivalds or a documented parameterized row-sampling bound per job shape. |
| BA-008 | "Hash-derived randomness is automatically unbiasable." | High | Validation seed uses finalized randomness and receipt id; assignment admission currently recomputes from current finalized randomness. | Hashing a biased or late-changing source does not make it unbiasable. Delayed attestations can be evaluated against the wrong lifecycle seed if the seed is not receipt-locked. | Randomness is deterministic from current chain state, with explicit beacon/commitment assumptions. | Store or derive receipt-lifecycle validation seed at receipt admission; prove miners commit before challenge derivation. |
| BA-009 | "Reference signatures prove production authentication." | High | `types::sign` is `hash(address, message)` and verification recomputes the hash. | This is a test helper, not a real signature scheme or key-ownership proof. | The reference core tests signature-flow plumbing. | Use a production signature scheme and state unforgeability, key custody, and replay-domain assumptions. |
| BA-010 | "LinearTrainingStep proves real ML training." | High | Current verifier checks finite-field algebraic relations. | Field arithmetic can model a training-shaped transition without proving real-valued SGD approximation or convergence. | LinearTrainingStep proves deterministic field-algebra transition consistency. | Define fixed-point scale, rounding, overflow/range bounds, and bridge theorem to real-valued SGD if that claim is desired. |
| BA-011 | "Local tensor serving proves public data availability." | High | Local tests/checkers can fetch descriptors, rows, chunks, and openings. | Local fetches do not prove independent retention or public availability over active/retention windows. | Local serving proves the code path and verification-time availability in local runs. | Add public DA measurement evidence with signed observations over retention windows. |
| BA-012 | "Local Compose operators are independent." | High | Compose starts multiple containers with separate identities and roles. | Separate local participants are not independent economic/security principals. | Compose proves multi-participant local shape, not public operator independence. | Require external operator attestations, disjoint ownership evidence, and public run evidence. |
| BA-013 | "Synthetic tensor jobs prove useful work." | Medium | Local synthetic job source emits deterministic matmul and LinearTrainingStep jobs. | Synthetic jobs prove verifiable compute rails, not externally demanded usefulness. | Synthetic jobs prove verifiable deterministic workload handling. | Separate synthetic-work metrics from user-valued workload metrics. |
| BA-014 | "High line coverage proves protocol soundness." | Medium | Tarpaulin reports strong line coverage for reference code. | Coverage proves lines executed, not that threat-model properties or consensus theorems hold. | Coverage is regression evidence for implemented behavior. | Tie every soundness claim to a theorem, assumption, adversarial test, or external evidence item. |
| BA-015 | "A clean local testnet gate means the MVP is complete." | Medium | Gate 0 exercises CPU local multi-participant paths. | Gate 0 is necessary local evidence, but not public-run, useful-PoW, DA, slashing, or independent-operator evidence. | Gate 0 is the first local acceptance gate. | Complete v2 consensus proof surface and public evidence gates before claiming full MVP completion. |
| BA-016 | "Produced blocks imply proposer eligibility." | Critical | `produce_block` accepts a supplied address and `submit_block_vote` checks voters, not the block proposer. | A block can be appended and finalized without the chain transition proving the proposer is a registered validator or useful-PoW winner. | Current block production is a reference append path with caller-supplied proposer. | Make block production/admission fallible and require v2 proposer eligibility before votes or finality count. |
| BA-017 | "A Valid attestation means the verifier actually ran." | High | `submit_attestation` accepts an assigned validator's signed `Valid` statement and `checks_root`; it does not recompute tensor verification. | Quorum is syntactic unless verifier evidence is independently bound, recomputed, or challengeable. | Current quorum proves assigned validators signed matching Valid/DataAvailable statements. | Bind attestations to recomputable check leaves, block-level `checks_root`, and challenge openings or direct verification evidence. |
| BA-018 | "Remote tensor fetch proves public data availability." | High | Current worktree adds root-addressed request-response fetches and validator remote-fetch counters. | A successful fetch proves one runtime retrieved a matching tensor at one time; it does not prove durable retention, public reachability, or independent hosting. | Remote fetch is verification-time artifact availability evidence. | Add signed public DA measurements across active/challenge windows and prove enough independent operators serve required artifacts. |
| BA-019 | "One Freivalds round over the current 31-bit field is cryptographic-scale soundness." | High | The current field is `2^31 - 1`, default `full_rounds` is `1`, and LinearTrainingStep has two single random-linear equality checks. | The default per-relation false-accept budget is about `1 / 2^31`, and receipt-volume union bounds can make that too weak for broad consensus claims. Validator quorum cannot be multiplied in while quorum is syntactic. | Current verifier checks have explicit finite-field probabilistic bounds under assumptions. | Set a target soundness budget, receipt-volume limit, round count, and random-linear repetition story; see `mvp_core_probabilistic_soundness_budget.md`. |

## Non-Negotiable Wording Rules

Use these rules in specs, README text, release notes, and investor-facing summaries:

1. Say **probabilistically verified** unless every relevant tensor output is fully recomputed or succinctly
   proven.
2. Say **local reference block production** for the current block path, not useful-verification PoW.
3. Say **superseded TensorWork proposer path** for `chain::proposer` until v2 block production replaces it.
4. Say **verification-time availability** for local tensor serving, not durable DA.
5. Say **field-algebra training step**, not real-valued training, unless fixed-point bridge theorems exist.
6. Say **reference signature helper**, not production authentication, until production crypto is wired.
7. Say **local multi-participant shape**, not independent operators, for Compose.
8. Say **signed Valid/DataAvailable statements**, not verified work, when referring only to current
   attestation quorum.
9. Say **verification-time remote fetch**, not public DA, for root-addressed request-response tensor
   retrieval.

## Claims We Can Make Today

These are defensible with current docs/code evidence:

- The verifier layer has proof-ready algebraic obligations for TensorOp and LinearTrainingStep.
- Freivalds and random-linear checks provide probabilistic soundness under explicit randomness and
  commitment assumptions.
- Row sampling is documented as weak for sparse corruption and should be treated as audit coverage.
- Chain attestation admission now enforces registered stake, signature validity, receipt metadata,
  deterministic assignment, and duplicate prevention.
- Current documentation clearly records that v2 useful-verification PoW consensus is not complete.
- Current attestation quorum proves syntactic assigned-validator agreement, not independent recomputation of
  verifier transcripts.
- Current root-addressed remote tensor fetch can prove payload/root matching for verifier use, not public DA.
- Current default probabilistic verifier bounds are finite-field proof budgets, not cryptographic-scale
  consensus soundness across unbounded receipt volume.

## Claims We Must Not Make Today

These would overstate the current MVP:

- "The MVP core is formally proven sound."
- "The chain implements useful-verification proof of work."
- "Finalized blocks prove validators verified the canonical receipt set."
- "TensorWork no longer affects proposer eligibility in implementation."
- "Local Compose evidence proves public independent operators."
- "Local tensor fetches prove durable data availability."
- "LinearTrainingStep proves real SGD correctness."
- "A finalized current block proves its proposer was eligible under v2 rules."
- "A quorum of Valid attestations proves validators actually executed the verifier."
- "Remote tensor fetch counters prove public data availability."
- "One default Freivalds round over the 31-bit field is enough for all production consensus volume."

## Proof Hygiene Checklist

Before upgrading any `implementation-blocked` item to `local-proof-ready`, require all of:

1. The theorem statement is written in `formal_proof_manifest_v0.md`.
2. The exact Rust transition or verifier surface exists.
3. Tests cover honest and adversarial cases for that surface.
4. Every cryptographic/randomness/economic dependency is listed as an assumption.
5. The relevant docs stop using superseded v1 evidence as v2 proof.
6. The claim is phrased with probabilistic bounds when randomness is involved.

## Current Judgment

The current proof docs are useful and materially more honest than the earlier MVP wording. The core is still
not sound enough for the full reviewed v2 MVP because block production and finality are still tied to the old
consensus object.

This ledger should remain open until useful-verification PoW, canonical settled-receipt blockspace, and v2
block finality validation are implemented and mapped back into the formal proof manifest.

Assumptions should be discharged only through the category-specific gates in
[`mvp_core_assumption_discharge_plan.md`](mvp_core_assumption_discharge_plan.md); tests, local runs, and
wording changes do not discharge cryptographic, public-evidence, or missing-implementation assumptions by
themselves.
