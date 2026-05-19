# GF(2) Bit-Packing Break

This is the strongest concrete break found so far. It attacks the **cost model** behind Assumption 6.4 for
small-field instantiations.

## Claim Being Attacked

Assumption 6.4 says there is no algorithm that computes all transcript intermediates for random rank-`r`
matrices faster than the stated matrix-multiplication bound.

The paper's main body counts arithmetic as field operations over `F_q`. If a protocol instantiates `F_q` as
a small field such as `GF(2)`, then a word-RAM implementation can pack many field elements into one
machine word. One word-level XOR/AND operation performs many `GF(2)` field operations at once.

That means the field-operation hardness claim does not directly transfer to real word operations for small
fields.

## POC

Run:

```bash
cargo run -p experiments --release --example gf2_bitpack_break
```

The example samples full-rank low-rank factors:

```text
E = EL * ER
F = FL * FR
```

over `GF(2)` with:

```text
n = 256
rank = tile = 64
```

It computes the same transcript hash two ways:

- scalar GF(2), counting one bit multiply/add as one field operation
- bit-packed GF(2), using one `u64` row word to process 64 field elements at a time

Representative output:

```text
GF(2) bitpacked transcript break
n=256, rank=tile=64, blocks=4
sampled low-rank factors are full rank: yes
same transcript hash: 9edb93c0...
scalar field terms: 16777216
packed row-xor estimate: 131072
estimated operation ratio scalar/packed: 128.00x
scalar elapsed: 3057 us
packed elapsed: 252 us
measured ratio scalar/packed: 12.09x
```

## Why This Matters

This does not break the current Rust crate's large-prime-field implementation. It also does not refute the
paper if the intended model is strictly "field operations over a large field, with no packing."

It does break a natural small-field instantiation of the assumption on real hardware. If someone tried to use
`GF(2)` or another tiny field for efficiency, then Assumption 6.4 would overestimate the work required by a
large factor. In asymptotic word-RAM terms, packing `w = O(log n)` field elements per word gives an
`O(w)` gap between field-operation cost and machine-word cost.

## Mitigation

- Require `q` large enough that each field element consumes a full machine word.
- State the security assumption in bit/word complexity, not only field-operation complexity.
- Benchmark against bit-sliced and SIMD implementations before accepting any small-field parameter set.
