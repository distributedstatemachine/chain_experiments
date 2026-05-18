# Attack Matrix

Current status of attempts to break the cuPoW transcript-hardness claim.

## Confirmed Breaks

### 1. Invalid `rank < tile` Parameterization

Status: fixed in this crate.

If consensus allows the noise rank to be smaller than the transcript tile size, a zero-job miner computes:

```text
C_{i,j}^{(k)} = EL_i * S_k * FR_j
S_k = sum_{t <= k} ER_t * FL_t
```

with less work than honest tiled multiplication. The implementation now rejects `rank != tile`.

Artifact:

- [assumption_break_poc.md](assumption_break_poc.md)
- [break_assumption.rs](../../crates/pearl_chain/examples/break_assumption.rs)

### 2. Small-Field Word-Packing Cost-Model Break

Status: confirmed scoped break.

For `GF(2)`, 64 field elements fit in one `u64`. A bit-packed transcript generator computes the exact same
intermediate transcript hash much faster than scalar field operations.

This breaks small-field instantiations of Assumption 6.4 on real hardware unless the assumption is restated
in word/bit complexity and benchmarked against bit-sliced implementations.

Artifact:

- [gf2_bitpack_break.md](gf2_bitpack_break.md)
- [gf2_bitpack_break.rs](../../crates/pearl_chain/examples/gf2_bitpack_break.rs)

## Attempted But Not A Break

### 1. Factorized Zero-Job Transcript With `rank == tile`

The same factorized formula works algebraically for the paper-shaped setting, but the scalar operation count
is not asymptotically better once `rank == tile`.

Artifact:

- [paper_param_probe.md](paper_param_probe.md)
- [paper_param_probe.rs](../../crates/pearl_chain/examples/paper_param_probe.rs)

### 2. Zero-Job No-Decode Route

A zero-job miner can skip decode because `A * B = 0` is known, but it still computes the transcript. This is
not a transcript-hardness break and benchmark results have not shown a stable advantage.

Artifact:

- [paper_param_practical_break.md](paper_param_practical_break.md)
- [paper_param_practical_break.rs](../../crates/pearl_chain/examples/paper_param_practical_break.rs)

## Still Unbroken Here

No POC in this repo breaks the exact paper-shaped large-field setting:

```text
rank == tile
field elements are full machine-word sized
the prover must compute/hash the full transcript
```

That remains conjectural, but not currently broken by these probes.
