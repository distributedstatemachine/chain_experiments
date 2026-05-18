# TensorVM Tarpaulin Report

Generated on May 18, 2026 from the workspace root with:

```bash
cargo tarpaulin
```

The root [`tarpaulin.toml`](../../tarpaulin.toml) expands that to workspace library coverage,
LLVM instrumentation, stdout output, and a force-clean build.

Host notes:

- `cargo-tarpaulin` must be at least `0.35.4` for the current Rust toolchain.
- `--engine Llvm` is required on this macOS/aarch64 host because Tarpaulin's ptrace backend is not supported.
- The older `cargo-tarpaulin 0.30.0` failed to parse Rust `1.94.1` / LLVM `21.1.8` profile data with `consistency check for reading counts failed`.

Result:

```text
175 tests passed under instrumentation:
- 14 pearl_chain library tests
- 161 tensor_vm library tests

98.45% workspace line coverage
5200/5282 workspace lines covered

100.00% tensor_vm crate line coverage
```

Tarpaulin reports line coverage here. Its branch coverage flag is currently listed as not implemented by the installed tool.
