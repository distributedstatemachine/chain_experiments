# TensorVM Tarpaulin Report

Generated on May 23, 2026 from the workspace root with:

```bash
cargo tarpaulin --workspace --offline
```

The root [`tarpaulin.toml`](../../tarpaulin.toml) expands that to workspace library coverage,
LLVM instrumentation, stdout output, and a force-clean build.

Host notes:

- `cargo-tarpaulin` must be at least `0.35.4` for the current Rust toolchain.
- `--engine Llvm` is used by the root `tarpaulin.toml` for stable instrumentation on this host.
- The older `cargo-tarpaulin 0.30.0` failed to parse Rust `1.94.1` / LLVM `21.1.8` profile data with `consistency check for reading counts failed`.

Result:

```text
260 tests passed under instrumentation:
- 14 experiments library tests
- 245 tensor_vm library tests
- 1 tensor_vm_explorer library test

98.14% workspace line coverage
11495/11713 workspace lines covered

98.74% tensor_vm crate line coverage
10650/10786 tensor_vm lines covered
100.00% tensor_vm_explorer crate line coverage
277/277 tensor_vm_explorer lines covered
```

The remaining uncovered `tensor_vm` lines are concentrated in block-admission rejection branches, pending
block-payload retry edges, and p2p request/response unhappy paths. Focused node and p2p tests cover the
main block-payload happy path, malformed payload rejection, invalid signature/root rejection, and duplicate
admission behavior.

The optional CUDA kernel feature is verified separately because the standard Tarpaulin configuration keeps
the portable default feature set:

```text
cargo test -p tensor_vm --features cuda-kernels --release
182 tensor_vm tests passed, including native CUDA field-matmul and linear-step tensor-op checks against
canonical CPU output
```

Tarpaulin reports line coverage here. Its branch coverage flag is currently listed as not implemented by the installed tool.
