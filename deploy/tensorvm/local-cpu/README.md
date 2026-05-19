# TensorVM Local CPU Compose Testnet

This bundle is the Docker Compose deployment target for
[`docs/tensorvm/local_cpu_testnet_spec.md`](../../../docs/tensorvm/local_cpu_testnet_spec.md). It is local
only, CPU only, and uses one container per counted operator.

## Services

The default topology defines 10 miners and 5 validators:

```text
miner-00 ... miner-09
validator-00 ... validator-04
```

`miner-00` is also the local bootstrap and host-facing gateway. It publishes:

```text
127.0.0.1:8545 -> local RPC, explorer, faucet, and telemetry HTTP routes
127.0.0.1:4001 -> optional host-visible libp2p bootstrap port
```

Every operator container initializes a durable node store, starts with a stable local operator ID, uses a
distinct data volume, derives a stable libp2p identity seed from that operator ID, runs the mandatory
libp2p readiness path, and starts `tvmd service serve`. Miner containers also run the CPU miner readiness
command with `--device cpu`; validators run the validator readiness command. `miner-00` seeds the local
CPU chain so the gateway exposes settled matmul and LinearTrainingStep work through `/chain/head`.

## Commands

From the repository root:

```bash
cargo test -p tensor_vm local_testnet --release
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml build
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml restart miner-03 validator-02
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml down -v
```

The Docker image builds `tensor_vm` with default features only. It does not enable `cuda-kernels`, mount
GPU devices, or require the NVIDIA container runtime.

## Evidence Boundary

The local Compose run is engineering evidence for the CPU local-testnet milestone. It is not
independently checkable public-testnet evidence and must continue to report:

```text
public_evidence_full_spec=false
independently_checkable=false
```
