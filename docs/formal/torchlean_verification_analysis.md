# TensorVM + TorchLean Verification Analysis

## Scope

This document reviews `docs/tensorvm/mvp_spec.md` and analyzes whether
[`lean-dojo/TorchLean`](https://github.com/lean-dojo/TorchLean) can strengthen TensorVM's
verification story.

Sources reviewed:

- Local spec: `docs/tensorvm/mvp_spec.md`
- TorchLean repository: <https://github.com/lean-dojo/TorchLean>
- TorchLean project site/docs: <https://torchlean.org/>
- TorchLean verification docs: <https://torchlean.org/docs/verification/overview>
- TorchLean trust-boundary docs: <https://torchlean.org/docs/governance/trust-boundaries>
- TorchLean arXiv abstract: <https://arxiv.org/abs/2602.22631>

## Executive Verdict

TorchLean should not be used as the online consensus verifier for TensorVM MVP receipts.

It should be used as a **formal specification and proof layer** for TensorVM's deterministic VM semantics,
training-step equations, verifier algorithms, and release gates.

The practical architecture should be:

```text
Lean/TorchLean:
  define semantics
  prove verifier soundness theorems
  generate golden vectors
  certify approved TensorVM programs and verifier logic

Rust node:
  execute compact deterministic validators
  check Merkle openings
  run Freivalds/random-linear checks
  process attestations, blocks, rewards, and finality
```

The core reason is latency and scope. TensorVM validators need cheap, deterministic, bandwidth-bounded
checks per receipt. TorchLean is a theorem-proving and neural-network verification framework. It is valuable
for proving that those checks mean what the protocol says they mean, but it is too heavy and too broad to
run inside every block-validation path.

## What TorchLean Actually Provides

TorchLean is a Lean 4 framework for neural-network specification, execution, and verification. Its public
materials describe:

- typed tensor and model APIs,
- a shared op-tagged graph IR,
- runtime and autograd support,
- finite-precision semantics including IEEE-style Float32 models,
- certificate checkers for IBP, CROWN/LiRPA, PINNs, ODE corridors, and splines,
- PyTorch import/export workflows,
- explicit trust-boundary documentation around CUDA, FFI, external oracles, and executable checkers.

The repository is Lean-heavy and young: the GitHub page currently shows a May 2026 v1 release and only a
small commit history. That is not disqualifying, but it matters for a blockchain consensus dependency.

The most relevant feature for TensorVM is not "verified ML models" in the usual robustness sense. It is
TorchLean's ability to keep tensor semantics, graph semantics, runtime behavior, and verification artifacts
in one formal environment.

## TensorVM Spec Summary

The local TensorVM spec is already a reviewed draft. Its important design choices are:

- deterministic tensor jobs,
- finite-field arithmetic for initial matmul jobs,
- fixed-point or integer-scaled arithmetic for the linear training step,
- Merkle-committed off-chain tensors,
- Freivalds verification for `C = A @ B`,
- random-linear checks for full-tensor elementwise relations,
- full-output Freivalds for block-eligible receipts,
- row sampling only as audit coverage unless soundness is explicitly bounded,
- validator useful-verification PoW over deterministic settled-receipt blockspace, with the older settled
  prior-epoch TensorWork proposer path now treated as superseded reference behavior,
- validation randomness from a finalized beacon or commit-reveal protocol,
- data availability only as verification availability unless durable DA is later added.

That puts the spec on the right track: it is a probabilistic verification protocol, not a claim that every
tensor cell is checked by every validator.

## Where TorchLean Fits

### 1. TensorVM Semantics

TensorVM needs a canonical execution-semantics definition:

```text
shape rules
dtype rules
row-major layout
field arithmetic
fixed-point scale rules
overflow behavior
program hashing
trace-root construction
```

TorchLean can host a formal version of this semantics. The MVP subset should be much smaller than
TorchLean's full neural-network surface:

```text
random_tensor
matmul
transpose
add
sub
mul
reduce_sum
scalar_mul
commit_tensor
hash_tensor
mse_loss
linear_backward
sgd_update
```

The value is that approved TensorVM programs can be defined against one mathematical object rather than
against informal Rust comments and tests.

Critique: TorchLean's public docs emphasize neural-network tensors, graph IR, Float32 semantics, runtime
autograd, and CROWN/LiRPA certificate workflows. TensorVM MVP needs finite-field and integer/fixed-point
consensus semantics. If TorchLean does not already expose a clean finite-field scalar domain for tensors,
TensorVM would need to add one or keep a separate Lean module that imports only the useful TorchLean
tensor/IR infrastructure.

### 2. Freivalds Soundness

The spec's core verifier is Freivalds:

```text
C r = A (B r)
```

TorchLean/Lean can prove the exact theorem TensorVM relies on:

```text
If C = A B, then every Freivalds check accepts.
If C != A B over field F_p and r is uniform, then
Pr[C r = A(B r)] <= 1 / p for one full-output round.
```

For repeated rounds:

```text
Pr[false accept] <= (1 / p)^rounds
```

or a tighter bound depending on the random vector distribution.

This proof should be part of the TensorVM release gate. It is better to mechanize it than leave it as a
hand-waved protocol claim.

Critique: Lean can prove the algebraic theorem, but it does not prove the randomness beacon is unbiasable,
the miner committed before seeing `r`, or the hash/Merkle commitments are collision resistant. Those remain
protocol and cryptographic assumptions.

### 3. Row-Sampled Freivalds Bounds

The local spec correctly warns that row-sampled Freivalds is weak against sparse corruptions.

Lean can formalize the actual detection probability:

```text
P_detect = 1 - C(m - t, s) / C(m, s)
```

where:

- `m` is the number of rows,
- `t` is the number of corrupted rows,
- `s` is the number of sampled rows.

For a one-row corruption:

```text
P_detect = s / m
```

This theorem would prevent future contributors from treating row sampling as equivalent to full Freivalds.

Critique: This is not a neural-network verification theorem. It is protocol probability and combinatorics.
TorchLean may not add much beyond Lean/mathlib here, except providing tensor notation and surrounding
infrastructure.

### 4. LinearTrainingStep Correctness

The spec's training primitive is:

```text
Y = X W_t
dY = Y - T
grad_W = X^T dY
W_{t+1} = W_t - lr * grad_W
```

TorchLean's autograd and model-semantics work is highly relevant here. The clean theorem to mechanize is:

```text
For loss L(W) = 1/2 * ||XW - T||^2,
the gradient is grad_W = X^T (XW - T).
```

Then TensorVM can state that the MVP training step is a valid SGD step for the half-squared-error loss.

The current spec also says `dY = Y - T` is the MVP gradient signal and warns not to claim exact mean-MSE
gradient semantics unless scaling and rounding are defined. That warning should remain. TorchLean can help
make this precise:

```text
loss convention: half squared sum, mean squared error, or scaled fixed-point proxy
gradient scale: exact factor absorbed into lr or explicit divisor
rounding: field arithmetic, rational fixed-point, or integer truncation
overflow: modular, checked, or bounded no-overflow theorem
```

Critique: If TensorVM uses modular field arithmetic, the word "learning" becomes semantic shorthand, not
ordinary real-valued gradient descent. Over a prime field, `W_next = W - lr * grad_W` is algebraically clean
but not necessarily an ML optimizer in the usual real-analysis sense. If TensorVM wants both consensus
determinism and real ML meaning, it needs either:

- rational/fixed-point semantics with no-overflow bounds, or
- a formal bridge theorem connecting field-scaled arithmetic to real arithmetic under bounded ranges.

TorchLean is useful for building that bridge, but it will not come for free.

### 5. Random-Linear Checks for Elementwise Relations

The reviewed spec rejects sampled-only checks for:

```text
dY = Y - T
W_next = W - lr * grad_W
```

and recommends random-linear checks:

```text
<q, dY> = <q, Y> - <q, T>
<q, W_next> = <q, W_t> - lr * <q, grad_W>
```

Lean can prove the same style of soundness theorem:

```text
If the elementwise relation is correct, the random-linear check always accepts.
If the relation is incorrect and q is uniform over F_p^n, false acceptance probability is <= 1 / p.
```

This is a good TorchLean-adjacent target because it links tensor semantics to verifier semantics.

Critique: This check requires validators to compute inner products over full tensors or streamed chunks.
Lean can prove the algebra, but it does not solve validator bandwidth. The spec still needs shape-specific
bandwidth budgets.

### 6. Program Approval and Governance

TorchLean can support a strong governance rule:

```text
No TensorVM primitive or program template becomes consensus-eligible until it has:
  - a Lean semantic definition
  - a Rust implementation
  - equivalence/golden-vector tests
  - verifier soundness theorem or explicit assumption
  - benchmarked validator cost
  - a release manifest pinning the Lean commit and theorem names
```

On-chain receipts already include `program_hash`. TensorVM can extend that idea:

```text
approved_program_id = H(canonical_program || lean_spec_hash || verifier_version)
```

Validators then check that a receipt refers to an approved program and run the corresponding Rust verifier.
They do not need to run Lean per receipt.

Critique: This is release governance, not permissionless arbitrary op deployment. That matches the spec's
non-goals. It does mean TensorVM v0 is not a general TensorVM chain; it is a chain with a small approved
operation set.

### 7. Certificate Checking for Future Model Claims

TorchLean's existing verification stack is focused on certificates such as IBP and CROWN/LiRPA bounds.
That is not required for TensorVM's MVP consensus, but it could be valuable later for:

- model robustness attestations,
- PINN/scientific-model residual bounds,
- certified inference properties,
- model upgrade governance,
- safety constraints on network-provided models.

The right staging is:

```text
MVP:
  use Lean/TorchLean for TensorVM and verifier correctness

MVP+:
  use TorchLean certificate checkers for optional model-quality claims

Later:
  allow proof-carrying model artifacts or formally checked upgrade proposals
```

Critique: Do not conflate "model robustness certificate accepted by TorchLean" with "miner performed the
claimed tensor work". Those are different statements.

## Recommended Architecture

### Repository Layout

Add a separate formal-spec package rather than embedding Lean into the Rust node:

```text
tensorchain/
  crates/
    experiments/ or tensor_vm/
      src/
        tensor/
        verifier/
        chain/
  formal/
    lakefile.lean
    lean-toolchain
    TensorVM/
      Tensor/
      TensorVM/
      Verification/
      Training/
      Commitments/
      TestVectors/
```

The `formal/` package can depend on TorchLean:

```lean
require TorchLean from git "https://github.com/lean-dojo/TorchLean.git" @ "<pinned-commit>"
```

Do not track TorchLean `main` for consensus-critical releases. Pin a commit or release tag.

### Build and Release Flow

```text
1. Lean defines TensorVM execution semantics.
2. Lean proves verifier theorems for approved primitives.
3. Lean emits or checks deterministic test vectors.
4. Rust implementation passes those vectors and adversarial tests.
5. Release manifest records:
   - TorchLean commit
   - TensorVM formal commit
   - theorem names
   - Rust verifier version
   - approved program hashes
6. Nodes only accept receipts for approved program hashes.
```

### Runtime Flow

```text
Miner:
  executes TensorVM job
  commits output root
  serves tensor chunks
  submits receipt

Validator:
  checks approved program hash
  derives randomness from finalized beacon
  verifies Merkle openings
  runs Rust Freivalds/random-linear checks
  submits attestation

Watcher:
  may run full recomputation
  may run Lean artifacts offline
  flags verifier or implementation drift
```

TorchLean is not in the block hot path.

## Concrete Theorems to Prove

### Tensor Algebra

```text
matmul_shape:
  if A has shape [m, k] and B has shape [k, n],
  then A @ B has shape [m, n]

matmul_deterministic:
  canonical matmul over field p is deterministic

transpose_shape:
  transpose([m, n]) = [n, m]

linear_step_shapes:
  X [b, d], W [d, o], T [b, o] imply:
    Y [b, o]
    dY [b, o]
    grad_W [d, o]
    W_next [d, o]
```

### Freivalds

```text
freivalds_complete:
  C = A B -> check(A, B, C, r) = true

freivalds_sound:
  C != A B -> Pr_r[check(A, B, C, r)] <= 1 / p

freivalds_repeated_sound:
  independent rounds multiply false-accept probability
```

### Row Sampling

```text
row_sample_detection:
  corruption in t rows, s rows sampled without replacement:
  P_detect = 1 - choose(m - t, s) / choose(m, s)
```

### Random-Linear Checks

```text
linear_relation_complete:
  D = Y - T -> <q,D> = <q,Y> - <q,T>

linear_relation_sound:
  D != Y - T -> Pr_q[<q,D> = <q,Y> - <q,T>] <= 1 / p

optimizer_relation_complete:
  W_next = W - lr * G -> <q,W_next> = <q,W> - lr * <q,G>

optimizer_relation_sound:
  W_next != W - lr * G -> false accept probability <= 1 / p
```

### Training Step

```text
linear_training_forward:
  Y = XW

half_squared_loss_gradient:
  grad_W = X^T(XW - T)

sgd_update_correct:
  W_next = W - lr * grad_W

training_step_verifier_complete:
  honest receipt passes forward, backward, and update checks

training_step_verifier_sound:
  invalid receipt passes only with bounded probability under beacon and commitment assumptions
```

### Commitments and Encoding

```text
canonical_encoding_injective:
  different tensor descriptors or byte arrays produce different preimages
  before cryptographic hashing

merkle_opening_validity:
  verified opening corresponds to a committed chunk index under the Merkle construction
```

For cryptographic hash collision resistance, Lean should state an assumption rather than pretend to prove SHA-256 security.

## Spec Changes I Would Make

### 1. Add a Formal Verification Layer

The spec has a Verification Layer, but it means runtime receipt verification. Add a distinct formal layer:

```text
Formal Specification Layer
  Lean/TorchLean semantics
  verifier soundness theorems
  approved program manifests
  release-time proof checks
```

This avoids mixing protocol validation with formal-methods validation.

### 2. Define "Verified" More Narrowly

The one-line definition currently says TensorVM is a testnet where probabilistically verified tensor
computation is the native primitive. That is good, but every public-facing claim should keep the word
"probabilistic" unless full recomputation or succinct proofs are used.

TorchLean can prove the probability bound, not remove the probability.

### 3. Make Field Arithmetic the v0 Default Everywhere

The spec still leaves LinearTrainingStep as "fixed-point or integer-scaled". For v0, choose one:

```text
field_element for all consensus checks
```

Then define the "learning" primitive as algebraic SGD over a field. If real-valued ML meaning is required,
make that an MVP+ theorem with bounded fixed-point semantics.

This drastically simplifies Lean proofs and runtime validators.

### 4. Keep Float32 Out of Consensus

TorchLean has valuable IEEE32 semantics, but the spec's non-goal is correct: no floating-point
consensus-critical outputs.

The trust-boundary docs explicitly mention CUDA/FFI paths and nondeterministic float reductions. This
supports the spec's determinism-first stance. Use TorchLean float work for research and non-consensus model
audits, not for v0 block validity.

### 5. Require Program Manifests

Every approved TensorVM program should have:

```text
program_name
program_hash
TensorVM version
dtype/domain
shape constraints
work-unit formula
runtime verifier function
formal spec hash
formal theorem names
test-vector hash
benchmark profile
```

This turns TorchLean output into something operationally usable.

### 6. Treat TorchLean as a Dependency Risk

TorchLean is promising but new. The spec should require:

```text
pinned commit
vendor or mirror strategy
CI lake build
theorem dependency audit
no unreviewed new axioms
explicit trust-boundary diff per upgrade
```

Do not accept upstream changes into consensus releases automatically.

## Major Critiques

### Critique 1: TorchLean Does Not Solve Online Verification Cost

TorchLean can check proofs and certificates, but TensorVM's validators need to validate many receipts
under block-time constraints. Running Lean or CROWN-style certificate checkers per receipt would likely be
too slow and operationally complex.

Use TorchLean to prove the Rust verifier, not to replace the Rust verifier.

### Critique 2: TorchLean's Existing Verification Is Not TensorVM's Main Verification Problem

TorchLean's public verification examples are robustness and certificate workflows: IBP, CROWN/LiRPA, margin
certificates, splines, PINNs, and ODE corridors. TensorVM's MVP problem is different:

```text
Did this miner commit a correct tensor result for this deterministic job?
```

That is algebraic randomized verification, not neural-network robustness verification.

TorchLean is still useful, but mostly as a Lean/tensor semantics foundation.

### Critique 3: The Scalar Domain Gap Is Real

TensorVM wants finite fields and maybe fixed-point integers. TorchLean's public story emphasizes Float32,
IEEE semantics, runtime autograd, and ML verification. If field tensors are not first-class in TorchLean,
TensorVM needs to implement them formally.

This is doable, but it is work.

### Critique 4: Formal Proofs Do Not Cover Incentive Attacks

Lean can prove Freivalds soundness under assumptions. It will not prove:

- validator randomness was unbiasable,
- validators did not collude,
- data was durably available,
- proposer selection was not manipulated,
- identities in redundant agreement were independent,
- rewards cannot be gamed,
- clients can retrieve tensors after settlement.

These are protocol, economics, and networking problems.

### Critique 5: Formalizing Hash Commitments Requires Assumptions

Merkle roots and program hashes are essential to TensorVM. Lean can prove that the encoding is canonical
and that a Merkle proof verifies against a root. It cannot prove SHA-256 collision resistance in the way the
protocol needs. That must remain an explicit cryptographic assumption.

### Critique 6: Fixed-Point Training Needs Range Proofs

If the MVP uses fixed-point, formal correctness requires:

- scale factor,
- rounding rule,
- overflow behavior,
- no-overflow bounds or modular semantics,
- equivalence to intended real update within an error bound.

Without this, "verified training step" means only "verified integer arithmetic step", not necessarily a
meaningful approximation to real SGD.

### Critique 7: CUDA Trust Boundaries Reinforce the Spec's Warnings

TorchLean documents CUDA/FFI as a trust boundary and notes nondeterminism risks in floating reductions.
This supports TensorVM's decision to keep GPU kernels as miner acceleration only. Consensus outputs must
match canonical deterministic semantics.

### Critique 8: Dependency Maturity Is a Consensus Risk

The GitHub page shows TorchLean as a new project with a small public history. Depending on it directly for
consensus-critical node operation would be premature. Depending on it for a formal spec package is much
safer, provided the dependency is pinned and audited.

## Recommended Phased Plan

### Phase 0: Offline Spike

Goal: decide whether TorchLean can comfortably host the MVP semantics.

Deliverables:

```text
formal/TensorVM/FieldTensor.lean
formal/TensorVM/Freivalds.lean
formal/TensorVM/LinearStep.lean
lake build in CI
10 deterministic test vectors cross-checked against Rust
```

Success criterion:

```text
Lean definitions are simple enough that the Rust team can keep them aligned.
```

### Phase 1: Formalize the MVP Verifiers

Deliverables:

```text
Freivalds completeness/soundness
random-linear relation completeness/soundness
row-sampling detection probability
LinearTrainingStep shape and algebraic correctness
Merkle opening semantics, with hash security as assumption
```

Success criterion:

```text
Every verifier acceptance rule in the spec maps to a theorem or explicit assumption.
```

### Phase 2: Program Approval Manifest

Deliverables:

```text
approved_programs.json
program_hash derivation
formal proof manifest
Rust verifier version manifest
test-vector manifest
CI gate checking all manifest entries
```

Success criterion:

```text
No consensus-eligible TensorVM program exists without formal and runtime evidence.
```

### Phase 3: Optional TorchLean Certificate Features

After the MVP verifier is stable, evaluate TorchLean's certificate checkers for:

```text
model robustness claims
PINN/scientific workload claims
model upgrade proposals
non-consensus quality attestations
```

Success criterion:

```text
Certificates are useful product features, not hidden dependencies for block validity.
```

## Bottom Line

TorchLean is a strong fit for making TensorVM's semantics and verifier claims precise. It is not a drop-in
replacement for Freivalds validators, Merkle opening checks, randomness beacons, data availability, slashing,
or consensus finality.

The best use is:

```text
TorchLean/Lean proves what the verifier means.
Rust validators run the verifier.
The chain accepts only approved programs whose semantics and verifier rules have formal evidence.
```

If TensorVM adopts that split, TorchLean could materially improve the MVP by preventing vague claims
around "verified tensor work" from drifting beyond what the protocol actually checks.
