# Paper-Parameter Attack Probe

After fixing the invalid `rank < tile` configuration, the remaining direct shortcut to test is the
factorized zero-job transcript formula under the paper's valid parameter coupling:

```text
rank == tile == r
```

For `A = 0` and `B = 0`, the transcript can be written as:

```text
C_{i,j}^{(k)} = EL_i * S_k * FR_j
S_k = sum_{t <= k} ER_t * FL_t
```

This is algebraically equivalent to the honest transcript. The question is whether it gives a malicious
prover a real advantage when `rank == tile`.

## Result

The probe does **not** break the paper's Assumption 6.4.

The shortcut advantage that existed for `rank < tile` disappears when the parameters are coupled. The
scalar-term estimate is slightly worse than honest tiled multiplication for the tested valid parameters, and
the measured wall-clock advantage seen at small sizes fades at larger sizes.

Run:

```bash
cargo run -p pearl_chain --release --example paper_param_probe
```

Representative output:

```text
n=64  r=tile=4  rounds=5: measured_ratio=2.396x scalar_ratio=0.938x
n=128 r=tile=4  rounds=4: measured_ratio=1.347x scalar_ratio=0.969x
n=128 r=tile=8  rounds=3: measured_ratio=1.248x scalar_ratio=0.938x
n=256 r=tile=8  rounds=2: measured_ratio=1.039x scalar_ratio=0.969x
n=256 r=tile=16 rounds=1: measured_ratio=0.962x scalar_ratio=0.938x
```

The small-size wall-clock win is an implementation artifact from skipping encode/decode work for the zero
product and from different constant factors. It is not an asymptotic attack on Assumption 6.4.

## Current Status

No valid break of the exact paper-shaped `rank == tile` assumption has been demonstrated in this repo.
The earlier POC remains a useful warning: a production consensus protocol must not allow independent
`rank` and `tile` parameters unless it has a separate proof for that setting.
