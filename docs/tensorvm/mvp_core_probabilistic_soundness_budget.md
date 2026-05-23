# TensorVM MVP Core Probabilistic Soundness Budget

Status: documentation-only probabilistic proof budget compiled from the current worktree.

Purpose: make the verifier-local false-accept bounds explicit. The current proof docs correctly say
Freivalds and random-linear checks are probabilistic, but a soundness claim also needs a budget: field size,
round counts, composition rule, receipt volume, and the assumptions that let probabilities multiply.

This document does not make the full MVP core sound. It only budgets the verifier-local algebraic checks
inside the current sound kernel.

The receipt-lifecycle seed model required for the non-adaptivity assumption is specified in
[`mvp_core_receipt_lifecycle_seed_model.md`](mvp_core_receipt_lifecycle_seed_model.md).

## Current Parameters

Current Rust evidence:

- Field modulus: `p = 2_147_483_647 = 2^31 - 1`.
- `FreivaldsParams::default().full_rounds = 1`.
- `FreivaldsParams::default().audit_rows = 16`.
- TensorOp verifier uses one full-output Freivalds routine with `max(full_rounds, 1)` rounds.
- LinearTrainingStep verifier uses two Freivalds routines, plus two single random-linear equality checks.
- Row sampling is present as audit evidence and is not counted as block-validity security in this budget.

Useful approximations:

```text
1 / p      ~= 4.657e-10
1 / p^2    ~= 2.168e-19
```

## Assumptions Required For These Bounds

The probability bounds below are valid only under the assumption leaves from
[`mvp_core_theorem_dependency_graph.md`](mvp_core_theorem_dependency_graph.md):

1. Challenges are sampled uniformly enough from the field.
2. Outputs and receipt roots are committed before challenges are derived.
3. Repeated rounds use independent-enough domain-separated challenge material.
4. Hash-to-field sampling matches the random-oracle or PRF model used in the theorem.
5. The Rust verifier semantics match the formal model.
6. Required verifier artifacts are available at verification time.

If any assumption fails, the numerical budget is not meaningful.

## Single-Receipt Bounds

Let:

```text
p = 2_147_483_647
r = max(full_rounds, 1)
```

| Receipt Type | False Relation | Conservative False-Accept Bound | Notes |
| --- | --- | --- | --- |
| TensorOp | `C != A @ B` | `p^-r` | Full-output Freivalds only. Row sampling is not credited unless separately modeled. |
| LinearTrainingStep | Forward matmul wrong | `p^-r` | Freivalds check on `Y = XW`. |
| LinearTrainingStep | Backward matmul wrong | `p^-r` | Freivalds check on `G = X^T dY`. |
| LinearTrainingStep | Error relation wrong | `p^-1` | Single random-linear check on `dY = Y - T`. |
| LinearTrainingStep | Optimizer relation wrong | `p^-1` | Single random-linear check on `W_next = W - lr * G`. |
| LinearTrainingStep | Loss commitment wrong | `0` if modeled exactly | Current check is deterministic finite-field equality, not real-valued loss correctness. |

For LinearTrainingStep, a conservative union-bound budget over the four probabilistic relation checks is:

```text
epsilon_linear <= 2 * p^-r + 2 * p^-1
```

With current defaults:

```text
epsilon_tensorop_default <= 4.657e-10
epsilon_linear_default  <= 1.863e-9
```

This is a verifier-local bound. It is not a v2 consensus-finality bound.

## Volume Budget

For `N` independently budgeted receipt checks, the conservative system-level budget is:

```text
epsilon_total <= N * epsilon_receipt
```

This means default one-round probabilities should not be described as cryptographic-scale soundness without
also stating expected receipt volume. Examples:

| Workload | TensorOp Default Budget | LinearTrainingStep Default Budget |
| --- | --- | --- |
| `N = 1` receipt | `4.657e-10` | `1.863e-9` |
| `N = 1_000` receipts | `4.657e-7` | `1.863e-6` |
| `N = 1_000_000` receipts | `4.657e-4` | `1.863e-3` |

The exact acceptable budget is a product/security decision, not a fact proved by the current code.

## What Must Not Be Multiplied Today

Do not multiply the false-accept bound by these factors unless the listed missing proof exists:

| Tempting Multiplier | Why It Is Invalid Today | Required Upgrade |
| --- | --- | --- |
| `validators_per_job` | Current quorum is syntactic; validators sign statements, and the chain does not prove independent recomputation. | Recomputable/challengeable verifier evidence or an explicit independent-honest-validator theorem. |
| `minimum_validators` | A quorum of `Valid` statements does not prove independent verifier executions. | Same as above, plus seed/domain separation per validator if multiplying checks. |
| Row sampling | TensorOp already has full Freivalds; row sampling is audit evidence and may be correlated through the same receipt seed. | Separate row-corruption model and independence/domain-separation proof. |
| Local testnet repetitions | Repeated local runs are regression evidence, not adversarial probability reduction. | A theorem tying production validation events to independent challenge draws. |
| Public DA observations | Availability evidence does not reduce algebraic false-accept probability. | Keep DA and algebraic soundness budgets separate. |

## Row Sampling Budget

For `t` corrupted rows among `m` total rows and `s` sampled rows without replacement:

```text
P_detect = 1 - choose(m - t, s) / choose(m, s)
```

This is useful audit math and should remain in the proof corpus, but it is not a replacement for full-output
Freivalds unless a target detection probability is explicitly chosen for every relevant job shape and
corruption model.

Bad assumption:

```text
16 sampled rows means sparse corruption is practically impossible.
```

Counterexample shape:

```text
m = 1024, t = 1, s = 16
P_detect ~= 16 / 1024 = 1.5625%
```

That is audit telemetry, not block-validity security.

## Parameter Gates

Before any public claim says "the verifier soundness is at most epsilon," require:

1. A target `epsilon_total` and maximum receipt volume `N`.
2. A chosen `full_rounds = r`.
3. A LinearTrainingStep budget that accounts for the two single random-linear checks.
4. A statement that row sampling is excluded from security budget unless separately modeled.
5. A statement that validator quorum does not reduce algebraic false acceptance under the current syntactic
   quorum model.
6. A receipt-lifecycle seed or equivalent proof that outputs are committed before challenge derivation.
7. Domain tags listed for every probabilistic check.

If current defaults are retained, the honest wording is:

```text
TensorOp and LinearTrainingStep verifier checks have explicit finite-field probabilistic false-accept
bounds under stated randomness and commitment assumptions. The default one-round, 31-bit-field budget is
not a cryptographic-scale consensus soundness claim, and it does not include v2 finality.
```

## Upgrade Pressure

If the target is cryptographic-scale verifier soundness across large receipt volumes, the current verifier
budget is not enough. The likely proof-compatible upgrades are:

1. Larger field or extension-field challenge space.
2. Multiple independent random-linear rounds for LinearTrainingStep error and optimizer checks.
3. Configured `full_rounds` tied to a target `epsilon_total`.
4. Receipt-lifecycle challenge seeds that make non-adaptivity a state theorem.
5. Validator evidence that supports independent recomputation before quorum probabilities are multiplied.

These are implementation changes, so this document only records them as gates.

## Current Judgment

The verifier-local probabilistic proof story is real but easy to overstate. The default budget is acceptable
only if the product accepts roughly `1/p`-scale per-relation false-accept risk and tracks receipt volume.
It does not prove public DA, validator honesty, production authentication, useful-verification PoW, or v2
finality.
