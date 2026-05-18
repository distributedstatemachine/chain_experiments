# Practical Finite-Size Attack Attempt

This probe tests whether zero-job miners get a practical advantage even when the consensus parameters are
valid:

```text
rank == tile
```

## Attack Attempt

Choose:

```text
A = 0
B = 0
```

For the noisy product:

```text
E = EL * ER
F = FL * FR
```

the cumulative transcript tile is:

```text
C_{i,j}^{(k)} = EL_i * S_k * FR_j
S_k = sum_{t <= k} ER_t * FL_t
```

Two malicious routes are tested:

- `factorized`: compute transcript tiles from `EL_i * S_k * FR_j`
- `no-decode`: materialize the same noisy matrices as the honest solver, compute the same transcript, but
  skip decode because `A * B = 0` is already known

## Run

```bash
cargo run -p pearl_chain --release --example paper_param_practical_break
```

The example:

- benchmarks honest proof attempts against the factorized zero-job route
- mines a block with the factorized route
- validates the block with the full verifier

## Current Result

This is **not** an asymptotic break of Assumption 6.4, because the fastest demonstrated route still computes
the transcript honestly.

The accepted-proof part is stable, but the timing result is not a confirmed break. Isolated runs on this
machine have varied from mild shortcut wins to clear shortcut losses. A recent isolated run:

```text
valid paper params: rank == tile == 4
zero job dimensions: 64x64
honest average proof attempt: 8144 us
shortcut average proof attempt: 29123 us
no-decode average proof attempt: 17681 us
finite-size measured ratio honest/factorized: 0.28x
finite-size measured ratio honest/no-decode: 0.46x
no-decode-mined paper-param block accepted: yes
```

## Interpretation

No asymptotic disproof of Assumption 6.4 has been found here. Confirmed findings:

- invalid `rank < tile` parameterization: broken, now rejected by validation
- valid `rank == tile` with zero jobs: accepted alternative proof-generation routes exist, but no stable
  speed advantage has been demonstrated
