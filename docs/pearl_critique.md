# Critique of `pearl.pdf`

## What The Paper Proposes

`pearl.pdf` argues for a proof-of-useful-work blockchain whose mining work is matrix multiplication instead
of hash iteration. The useful task is `C = A * B`; the proof task is based on the transcript of a noisy tiled
matrix multiplication.

The key construction is Algorithm 6.4:

- derive low-rank noise `E = EL * ER` and `F = FL * FR` from an unpredictable seed and the chosen matrices
- compute the tiled transcript of `(A + E) * (B + F)`
- hash the transcript to obtain the PoW lottery value
- recover the useful product by subtracting the cheap low-rank correction

This is an important idea because it shifts hardness from the final matrix product, which low-rank noise
makes shortcuttable, to the transcript of intermediate tile computations.

## Strong Points

- The target workload is economically meaningful. Matrix multiplication is a real bottleneck for AI,
  simulation, graphics, databases, and scientific computing.
- The low-rank-noise construction has the right asymptotic shape for a useful PoW: one dominant matrix
  multiplication plus `O(n^2 r)` encode/decode overhead for square matrices.
- Tying the lottery to intermediate transcript values is the paper's strongest technical move. It blocks the
  obvious attack where a miner chooses easy `A` and `B` and only multiplies low-rank noise.
- The paper correctly separates usefulness, efficiency, and security instead of treating "does work" as enough.
- Appendix B identifies the blockchain-specific Poisson-process requirement, which is often skipped in PoUW
  proposals.

## Main Weaknesses

- The core security claim is conjectural. Assumption 6.4, about computing all correlated low-rank transcript
  intermediates, is new and not yet supported by reductions to standard assumptions.
- Verifier cost is too high for a base-layer chain as written. Full verification repeats the prover's tiled
  transcript computation, which makes ordinary nodes expensive.
- The paper moves between "proof contains the transcript" and "proof contains a hash of the transcript."
  A production protocol needs a precise wire format, data availability rules, and compact verification story.
- The useful-work market is underspecified. A chain needs job selection, payment, result delivery, retries,
  privacy, and dispute handling, not only a PoW primitive.
- Real AI workloads are usually floating-point, approximate, GPU-specific, and sometimes nondeterministic.
  The paper's clean finite-field model does not directly solve reproducible useful AI computation.
- Miner-chosen inputs create economic edge cases. Even if all-zero matrices are cryptographically handled,
  miners may prefer worthless private jobs unless rewards are tied to external demand.
- Difficulty adjustment is only sketched. Matrix sizes, ranks, tile sizes, GPU classes, memory bandwidth, and
  verifier cost all affect effective mining rate.
- Memory pressure is nontrivial. The transcript cannot be stored naively; the implementation should hash it
  streaming and define the exact order of intermediate states.
- Hardware centralization remains likely. If rewards favor high-end GPU matrix throughput, the system shifts
  from ASIC mining centralization to GPU/datacenter centralization.

## Suggested Improvements

- Formalize the exact blockchain proof object: matrix commitments, transcript hash, output commitment,
  tile order, seed derivation, difficulty target, and whether matrices/results are on-chain or off-chain.
- Add a compact verification layer. The practical options are SNARKs for accepted blocks, probabilistic
  fraud proofs, committee verification, or a hybrid where full nodes can choose verification depth.
- Build a job market protocol around the primitive: request format, escrow, result acceptance, job expiry,
  result reuse, and pricing by matrix size and precision.
- Define a deterministic numeric domain bridge for AI workloads. Options include finite-field-native jobs,
  quantized integer matrix multiplication, or reproducible fixed-point kernels.
- Publish parameter guidance for `n`, `r`, tile size, field modulus, transcript hash function, and difficulty.
  This should be backed by CPU and GPU benchmarks rather than only asymptotics.
- Analyze correlated-transcript attacks directly. The paper should include attack attempts using low-rank
  decompositions, rectangular multiplication tricks, preprocessing, and memory-time tradeoffs.
- Specify data availability and privacy. Many useful matrices are proprietary; commitments, encrypted jobs,
  or zk proofs may be needed before the work is economically usable.
- Add a denial-of-service model. Verifiers need cheap rejection paths for malformed dimensions, invalid
  commitments, oversized jobs, and unsupported parameters.
- Explore Appendix A's self-canceling rotation scheme as a practical alternative because it removes decode
  overhead, but only after clarifying its rank and zero-input limitations.

## Implementation Scope In This Repo

This repository implements the conservative core:

- finite-field matrices, not floating point
- streaming transcript hashing, not transcript storage
- low-rank Algorithm 6.4 encode/decode
- full verifier recomputation
- a small block/chain layer that uses the transcript hash as the PoW lottery value

It is a research prototype, not a production L1. The biggest missing production feature is compact
verification; without it, verifier cost is too high for a broad permissionless network.

For concrete schemes that bridge finite-field PoUW with real AI workloads, see
[ai_reproducibility_schemes.md](ai_reproducibility_schemes.md).
