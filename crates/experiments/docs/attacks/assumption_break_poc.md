# Fixed: Low-Rank Transcript Shortcut Guard

The earlier POC identified a parameterization bug in this Rust prototype, not a break of the paper's exact
Algorithm 6.4.

## The Bug

The implementation originally permitted independent values for:

```text
tile
rank
```

That allowed:

```text
rank < tile
```

For a zero-matrix job:

```text
A = 0
B = 0
```

the noisy matrices are:

```text
E = EL * ER
F = FL * FR
```

The cumulative transcript tile can then be computed from the public low-rank factors:

```text
C_{i,j}^{(k)} = EL_i * S_k * FR_j
S_k = sum_{t <= k} ER_t * FL_t
```

When `rank` is much smaller than `tile`, this computes the same transcript hash with less work than honest
`tile x tile` multiplication. A miner could therefore get more lottery attempts per unit work.

## The Fix

`CuPowParams::validate` now enforces:

```text
rank == tile
```

This matches the paper's Algorithm 6.4, which uses one parameter `r` for both the low-rank noise and the
transcript tile size.

## Regression Example

Run:

```bash
cargo run -p experiments --release --example break_assumption
```

Expected behavior:

- `rank < tile` is rejected before mining or verification
- `rank == tile` still computes and verifies normally

The factorized transcript formula still exists for `rank == tile`, but it is not the earlier break: its scalar
work is no longer asymptotically below the honest tiled multiplication in the way the unsafe `rank < tile`
configuration was.

## Remaining Security Note

This guard only fixes the implementation bug. It does not prove the paper's Assumption 6.4. The paper's
security still relies on the conjecture that, for the valid `rank == tile` setting, correlated low-rank
transcript intermediates cannot be computed with a meaningful shortcut over the honest algorithm.
