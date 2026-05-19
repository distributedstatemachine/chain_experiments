# experiments

Research crate for non-TensorVM proof-of-useful-work prototypes.

The current implementation is a Rust prototype for the matrix-multiplication proof-of-useful-work chain
described in [`pearl.pdf`](docs/pearl/pearl.pdf) (`Proofs of Useful Work from Arbitrary Matrix Multiplication`).

The implementation is self-contained and dependency-free:

- finite-field matrix arithmetic over `2^31 - 1`
- SHA-256 based random oracle and deterministic seed expansion
- low-rank-noise cuPoW solve/verify path
- transcript hashing over tiled matrix multiplication intermediates
- minimal block, mining, and chain validation APIs

## Quick Start

From the workspace root:

```bash
cargo test -p experiments --release
cargo run -p experiments --release --example mine
```

## Design Notes

The implemented proof follows the paper's Algorithm 6.4 shape:

1. Derive low-rank matrices `EL`, `ER`, `FL`, `FR` from the block seed and job commitment.
2. Construct `E = EL * ER` and `F = FL * FR`.
3. Multiply `(A + E) * (B + F)` with the canonical tiled algorithm.
4. Hash every cumulative tile intermediate into a compact transcript digest.
5. Decode the useful result with:

```text
C = C' - ((A * FL) * FR + EL * (ER * (B + F)))
```

The chain layer treats the transcript hash as the PoW lottery value. A block is valid only if full
verification recomputes the same useful product and the transcript hash satisfies the configured
leading-zero difficulty.

## Performance Choices

- Matrices are row-major `Vec<u64>` values.
- General matrix multiplication transposes the right matrix for contiguous dot products.
- Tiled transcript multiplication uses a reusable `u128` scratch tile and reduces once per tile cell.
- Low-rank correction work is rectangular and scales as `O(n^2 r)` for square jobs.
- Verification is deliberately full recomputation, matching the paper's unoptimized verifier. A production
  chain would need compact proof delegation, batching, or SNARK-based verification.

## Examples

```bash
cargo run -p experiments --release --example mine
cargo run -p experiments --release --example break_assumption
cargo run -p experiments --release --example paper_param_probe
cargo run -p experiments --release --example paper_param_practical_break
cargo run -p experiments --release --example gf2_bitpack_break
```

## Related Notes

- [Paper critique](docs/pearl/critique.md)
- [AI reproducibility schemes](docs/pearl/ai_reproducibility_schemes.md)
- [Attack matrix](docs/attacks/attack_matrix.md)
- [GF(2) bit-packing break](docs/attacks/gf2_bitpack_break.md)
