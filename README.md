# TensorVM Workspace

Cargo workspace for TensorVM (TVM) and related proof-of-useful-work chain experiments.

## Crates

| Crate | Purpose |
| --- | --- |
| [pearl_chain](crates/pearl_chain/README.md) | Matrix-multiplication proof-of-useful-work chain prototype based on [`pearl.pdf`](docs/pearl/pearl.pdf). |
| [tensor_vm](crates/tensor_vm/README.md) | Reference implementation of the reviewed TensorVM MVP spec. |

New chain designs should live under `crates/<name>/` and include their own `README.md`.
Protocol papers and research notes live in [docs](docs/README.md).

## Workspace Commands

```bash
cargo test --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check --all
cargo tarpaulin
```

Run a crate example:

```bash
cargo run -p pearl_chain --release --example mine
cargo test -p tensor_vm --release
```

## Documentation

See [docs](docs/README.md) for papers, protocol reviews, implementation notes, and attack writeups.
