# TensorVM

TensorVM is a proof-of-useful-work chain for deterministic tensor programs. Miners run useful matrix and
training jobs, validators check receipts and data availability, and the chain turns settled tensor work into
rewards and proposer weight.

The repo has two crates:

- [tensor_vm](crates/tensor_vm/README.md): the node, chain, CPU/CUDA runtimes, libp2p service, RPC surfaces,
  evidence tooling, and local CPU testnet.
- [experiments](crates/experiments/README.md): non-TensorVM research prototypes, paper notes, and attack
  probes.

TensorVM docs live in [docs/tensorvm](docs/tensorvm/README.md). Deployment assets live in
[deploy/tensorvm](deploy/tensorvm/README.md).

## Try It

```bash
cargo test -p tensor_vm local_testnet --release
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
```

## Check Everything

```bash
cargo fmt --check --all
cargo test --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
cargo tarpaulin
```
