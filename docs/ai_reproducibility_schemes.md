# Schemes For Reproducible Useful AI Compute

The paper's clean model is finite-field matrix multiplication. Real AI workloads are usually floating-point,
approximate, GPU-kernel specific, and sometimes nondeterministic. This document lays out concrete protocol
schemes that bridge that gap.

## Design Requirements

A usable bridge needs all of these:

- deterministic transcript input: every prover and verifier derives the same matrices, seed, shape, and tile
  order
- deterministic arithmetic: the proof lottery cannot depend on non-reproducible floating-point behavior
- useful output semantics: the result must still be valuable for inference, training, retrieval, simulation, or
  another buyer workload
- hardware-aware performance: the scheme cannot turn a GPU-native workload into a CPU-only audit trail
- explicit acceptance rule: exact equality for finite-field/integer work, or a canonical tolerance rule for
  floating-point work

## Scheme 1: Quantized Field-Native MatMul

This is the cleanest path.

The job owner converts AI tensors into quantized integers before mining:

```text
A_fp -> A_q
B_fp -> B_q
C_int = A_q * B_q
C_fp ~= scale_A * scale_B * C_int
```

The PoUW instance is the exact finite-field or integer product `A_q * B_q`. The useful result is either the
integer accumulator or the dequantized tensor. The job definition commits to:

- dtype: `int4`, `int8`, `fp8-to-int`, or fixed-point
- scale and zero-point rules
- clipping/saturation rules
- accumulator width
- field modulus `p`
- output dequantization rule

To avoid modular wraparound, choose:

```text
p > 2 * k * max_abs(A_q) * max_abs(B_q)
```

or use several CRT moduli and reconstruct the integer accumulator.

Best fit:

- quantized inference
- embedding and retrieval workloads
- models trained with quantization-aware training
- workloads where exact integer accumulators are acceptable

Pros:

- preserves the paper's finite-field security shape
- deterministic across hardware
- easy verifier semantics
- maps well to real int8/fp8 accelerator paths

Cons:

- not a transparent replacement for arbitrary FP16/BF16 training
- job owner must accept quantization error
- field modulus and accumulator range must be protocol parameters

## Scheme 2: Canonical Fixed-Point Kernel ABI

Define a consensus-level deterministic tensor ABI:

```text
Tensor = signed fixed-point integer + scale exponent
MatMul = exact integer multiply + exact deterministic reduction tree
Activation = table, polynomial, or canonical rounding rule
```

The GPU kernel may use any implementation, but the result must equal the canonical integer semantics. The
PoUW transcript is over the canonical tile states, not over GPU-private floating-point states.

Required ABI fields:

- shape and layout
- signedness and bit width
- scale exponent
- rounding mode
- overflow behavior
- reduction order
- tile size
- activation approximation, if any

Best fit:

- inference stacks that can run fixed-point
- deterministic training experiments
- edge models and embedded inference

Pros:

- reproducible by construction
- still accelerator-friendly
- supports more than raw MatMul if activation kernels are standardized

Cons:

- requires model conversion
- can diverge from mainstream BF16/FP16 training numerics
- every supported op needs canonical semantics

## Scheme 3: Dual-Track Floating-Point Plus Canonical Shadow Work

Run the real AI workload in native FP16/BF16/FP32 for the buyer, but bind mining eligibility to a canonical
shadow computation derived from the same tensors:

```text
real path:    C_fp = GPU_GEMM(A_fp, B_fp)
shadow path:  C_q  = Quantize(A_fp) * Quantize(B_fp)
proof hash:   H(transcript(C_q))
```

The block proof uses the exact finite-field shadow transcript. The buyer receives the native floating-point
result. The job commitment binds both paths:

```text
H(A_fp, B_fp, quantization_spec, kernel_profile, C_fp_commitment, C_q_commitment)
```

Best fit:

- workloads where the useful FP result is valuable, but exact reproducibility is not needed on-chain
- AI training/inference jobs that can tolerate an attached quantized audit path

Pros:

- compatible with existing AI kernels
- keeps the consensus lottery deterministic
- avoids pretending FP kernels are globally reproducible

Cons:

- the useful FP work and proof work are adjacent, not identical
- overhead depends on how cheap the shadow quantized path is
- a buyer still needs off-chain validation for FP result quality

Security note:

This is weaker than Scheme 1 as a pure PoUW. It should be presented as "FP work with deterministic proof
attachment," not as a perfect proof that the exact FP kernel ran.

## Scheme 4: Versioned Deterministic GPU Kernel Profiles

Define a registry of deterministic kernel profiles:

```text
profile_id = H(gpu_arch, driver, CUDA/cuBLAS/cuDNN version, kernel, workspace, stream policy, math mode)
```

A job can require a profile. Miners must run with:

- one stream or fixed per-stream workspace
- fixed cuBLAS/cuDNN versions
- fixed math mode and tensor-core mode
- atomics disabled when they affect reproducibility
- deterministic framework flags enabled

Verification can be done by:

- re-execution on matching hardware
- validator committees with the same profile
- spot checks on random tiles
- TEEs/remote attestation as an optional operational layer

Best fit:

- short-term deployments
- private or semi-permissioned markets
- cases where the hardware fleet is known

Pros:

- closest to current AI infrastructure
- can support BF16/FP16 native kernels
- practical for controlled fleets

Cons:

- not fully permissionless
- brittle across driver/toolkit/hardware changes
- deterministic settings can reduce performance
- verifier availability depends on matching hardware

## Scheme 5: Tolerance-Band Floating-Point Commitments

Instead of demanding bitwise equality, define a canonical acceptance band:

```text
Accept C if |C - reference(A, B)| <= error_bound(A, B, dtype, kernel_profile)
```

The protocol should not hash raw floating-point outputs directly. It should hash a canonical quantized
commitment:

```text
Q(C_fp, tolerance_spec)
```

Verification can use randomized checks:

```text
A * (B * r) ~= C * r
```

with interval arithmetic or high-precision spot checks. The PoW lottery remains tied to a deterministic
finite-field transcript or a canonical bucketed representation.

Best fit:

- approximate inference
- training checkpoints where small numeric drift is acceptable
- outputs too large for full deterministic re-execution

Pros:

- acknowledges real FP behavior
- cheaper verification is possible
- useful for buyer-facing correctness disputes

Cons:

- tolerance can become an attack surface
- not enough by itself for Nakamoto-style mining fairness
- must be paired with deterministic proof work

## Scheme 6: Checkpointed Deterministic Training Segments

Training is harder than inference because small numeric differences compound. Break training into short,
canonical segments:

```text
input checkpoint
microbatch
deterministic forward/backward segment
optimizer update
output checkpoint
```

Each segment uses Scheme 1 or Scheme 2 arithmetic. The chain commits to checkpoints, not entire open-ended
training runs. Randomness such as dropout and data order is derived from the block/job seed:

```text
rng_seed = H(job_id, segment_id, global_seed)
```

Best fit:

- reproducible research training
- quantized/fixed-point training
- fine-tuning jobs where checkpoint validation matters

Pros:

- contains nondeterminism
- enables retry/dispute at segment granularity
- maps to job-market payment milestones

Cons:

- stricter than normal training stacks
- may reduce model quality unless training is designed for it
- more protocol machinery

## Scheme 7: Useful-Work Market With Result Classes

Do not pretend all AI work has one verification model. Define result classes:

```text
Class A: exact finite-field/integer result
Class B: deterministic fixed-point model result
Class C: native FP result plus deterministic shadow proof
Class D: native FP result with tolerance-band dispute process
```

Consensus should only depend on Class A/B deterministic transcripts. Class C/D can receive market payments
but should not be the sole mining security basis.

Best fit:

- production marketplace
- mixed buyer workloads
- gradual adoption

Pros:

- honest about guarantees
- lets high-value FP workloads participate without weakening consensus
- gives users clear semantics

Cons:

- more complex UX
- requires pricing different verification strengths
- consensus and marketplace accounting must be separated carefully

## Recommended Path

Use a layered design:

1. Consensus mining uses Scheme 1 or Scheme 2 only.
2. Native FP AI jobs use Scheme 3 or Scheme 5 as marketplace attachments, not as the core consensus proof.
3. Training jobs use Scheme 6 with deterministic checkpoints.
4. Deployment starts with Scheme 4 profiles for practicality, but the long-term security target is profile
   independence through exact integer/fixed-point semantics.

The core rule is:

```text
Consensus proof inputs must be deterministic. Buyer-facing FP outputs may be approximate, but they need a
separate acceptance/dispute layer.
```

## Sources Checked

- PyTorch Reproducibility notes: https://docs.pytorch.org/docs/stable/notes/randomness.html
- PyTorch Numerical Accuracy notes: https://docs.pytorch.org/docs/2.9/notes/numerical_accuracy.html
- PyTorch deterministic algorithms API: https://docs.pytorch.org/docs/stable/generated/torch.use_deterministic_algorithms.html
- NVIDIA cuBLAS reproducibility notes: https://docs.nvidia.com/cuda/cublas/index.html#results-reproducibility
- NVIDIA cuDNN reproducibility notes: https://docs.nvidia.com/deeplearning/cudnn/
