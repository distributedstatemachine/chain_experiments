# Chain Workspace

Cargo workspace for experimenting with multiple chain designs and proof-of-useful-work consensus
variants.

## Crates

| Crate | Purpose |
| --- | --- |
| [pearl_chain](crates/pearl_chain/README.md) | Matrix-multiplication proof-of-useful-work chain prototype based on [`pearl.pdf`](docs/pearl.pdf). |

New chain designs should live under `crates/<name>/` and include their own `README.md`.

## Workspace Commands

```bash
cargo test --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check --all
```

Run a crate example:

```bash
cargo run -p pearl_chain --release --example mine
```

## Research Notes

- [pearl.pdf](docs/pearl.pdf)
- [Ambient litepaper](docs/Ambient_Litepaper_V1.pdf)
- [Paper critique](docs/pearl_critique.md)
- [Pearl vs Ambient protocol review](docs/pearl_vs_ambient_protocol_review.md)
- [AI reproducibility schemes](docs/ai_reproducibility_schemes.md)
- [Attack matrix](docs/attack_matrix.md)
- [GF(2) bit-packing break](docs/gf2_bitpack_break.md)
