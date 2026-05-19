#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
BUNDLE_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(CDPATH= cd -- "$BUNDLE_DIR/../../.." && pwd)
COMPOSE_FILE="$BUNDLE_DIR/docker-compose.yml"
RPC_PORT="${TENSORVM_LOCAL_CPU_RPC_PORT:-8545}"
AUTH_TOKEN="${TENSORVM_AUTH_TOKEN:-local-cpu-testnet-token}"

EXPECTED_SERVICES="miner-00 miner-01 miner-02 miner-03 miner-04 miner-05 miner-06 miner-07 miner-08 miner-09 validator-00 validator-01 validator-02 validator-03 validator-04"
MINERS="miner-00 miner-01 miner-02 miner-03 miner-04 miner-05 miner-06 miner-07 miner-08 miner-09"
VALIDATORS="validator-00 validator-01 validator-02 validator-03 validator-04"

fail() {
  echo "local CPU testnet check failed: $*" >&2
  exit 1
}

compose() {
  docker compose -f "$COMPOSE_FILE" "$@"
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

contains_line() {
  printf '%s\n' "$1" | grep -qx "$2"
}

unique_count() {
  sort -u "$1" | wc -l | tr -d ' '
}

require_command docker
require_command grep
require_command sort
require_command wc
require_command cargo
require_command curl

cd "$REPO_ROOT"

compose config --quiet

CONFIG_SERVICES=$(compose config --services)
RUNNING_SERVICES=$(compose ps --status running --services)

for service in $EXPECTED_SERVICES; do
  contains_line "$CONFIG_SERVICES" "$service" || fail "compose config is missing $service"
  contains_line "$RUNNING_SERVICES" "$service" || fail "$service is not running"
done

TMP_DIR="${TMPDIR:-/tmp}/tensorvm-local-cpu-check.$$"
mkdir -p "$TMP_DIR"
trap 'rm -rf "$TMP_DIR"' EXIT INT TERM

for service in $EXPECTED_SERVICES; do
  compose exec -T "$service" test -f /var/lib/tensorvm/local-cpu-ready \
    || fail "$service has not written /var/lib/tensorvm/local-cpu-ready"
  compose exec -T "$service" grep -q "operator_name=$service" /var/lib/tensorvm/local-cpu-ready \
    || fail "$service readiness file does not name its operator"
  compose exec -T "$service" grep -q "p2p_runtime=libp2p" /var/lib/tensorvm/local-cpu-ready \
    || fail "$service is missing libp2p runtime readiness"
  compose exec -T "$service" grep -q "node_store_ready=true" /var/lib/tensorvm/local-cpu-ready \
    || fail "$service is missing node store readiness"
  compose exec -T "$service" grep -q "libp2p_ready=true" /var/lib/tensorvm/local-cpu-ready \
    || fail "$service is missing libp2p readiness"
  compose exec -T "$service" grep -q "p2p_identity_seeded=true" /var/lib/tensorvm/local-cpu-ready \
    || fail "$service is missing stable libp2p identity readiness"
  operator_id=$(compose exec -T "$service" printenv TENSORVM_OPERATOR_ID)
  compose exec -T "$service" grep -q "p2p_identity_seed=$operator_id" /var/lib/tensorvm/local-cpu-ready \
    || fail "$service libp2p identity seed does not match its operator ID"
  compose exec -T "$service" grep "^p2p_peer_id=" /var/lib/tensorvm/local-cpu-ready >> "$TMP_DIR/p2p_peer_ids"
  compose exec -T "$service" printenv TENSORVM_OPERATOR_ID >> "$TMP_DIR/operator_ids"
  compose exec -T "$service" printenv TENSORVM_NODE_MULTIADDR >> "$TMP_DIR/node_multiaddrs"
done

for service in $MINERS; do
  compose exec -T "$service" grep -q "role=miner" /var/lib/tensorvm/local-cpu-ready \
    || fail "$service is not marked as a miner"
  compose exec -T "$service" grep -q "device=cpu" /var/lib/tensorvm/local-cpu-ready \
    || fail "$service is not using the CPU backend"
done

for service in $VALIDATORS; do
  compose exec -T "$service" grep -q "role=validator" /var/lib/tensorvm/local-cpu-ready \
    || fail "$service is not marked as a validator"
  compose exec -T "$service" grep -q "reference_verifier_ready=true" /var/lib/tensorvm/local-cpu-ready \
    || fail "$service validator readiness is missing"
done

[ "$(unique_count "$TMP_DIR/operator_ids")" = "15" ] || fail "operator IDs are not distinct"
[ "$(unique_count "$TMP_DIR/p2p_peer_ids")" = "15" ] || fail "libp2p peer IDs are not distinct"
[ "$(unique_count "$TMP_DIR/node_multiaddrs")" = "15" ] || fail "node multiaddrs are not distinct"

compose exec -T miner-00 grep -q "command=local_testnet_seed" /var/lib/tensorvm/local-testnet-seed.out \
  || fail "miner-00 did not seed local testnet chain state"
compose exec -T miner-00 grep -q "settled_receipts=10" /var/lib/tensorvm/local-testnet-seed.out \
  || fail "seeded local testnet did not report settled receipts"
compose exec -T miner-00 grep -q "matmul_settled=true" /var/lib/tensorvm/local-testnet-seed.out \
  || fail "seeded local testnet did not settle matmul work"
compose exec -T miner-00 grep -q "linear_training_settled=true" /var/lib/tensorvm/local-testnet-seed.out \
  || fail "seeded local testnet did not settle linear training work"
compose exec -T miner-00 grep -q "rewarded_miners=9" /var/lib/tensorvm/local-testnet-seed.out \
  || fail "seeded local testnet did not report miner rewards"
compose exec -T miner-00 grep -q "finality_rate_bps=10000" /var/lib/tensorvm/local-testnet-seed.out \
  || fail "seeded local testnet did not report full finality"
compose exec -T miner-00 grep -q "data_availability_bps=10000" /var/lib/tensorvm/local-testnet-seed.out \
  || fail "seeded local testnet did not report full data availability"

for path in /health /rpc/health /chain/head /jobs/current /explorer/health /explorer /faucet/health /faucet/page /telemetry/health /telemetry/dashboard; do
  curl -fsS -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}${path}" >/dev/null \
    || fail "gateway route is not reachable: $path"
done

CHAIN_HEAD=$(curl -fsS -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/chain/head")
printf '%s\n' "$CHAIN_HEAD" | grep -q '"height":2' || fail "gateway chain head does not expose seeded height 2"
printf '%s\n' "$CHAIN_HEAD" | grep -q '"block_count":2' || fail "gateway chain head does not expose 2 seeded blocks"

cargo test -p tensor_vm local_testnet --release

cat <<'STATUS'
local_cpu_testnet_ready=true
ready_miners=10
ready_validators=5
distinct_operator_ids=15
distinct_libp2p_peer_ids=15
distinct_node_multiaddrs=15
libp2p_ready_node_count=15
cpu_ready_miner_count=10
cuda_required_miner_count=0
settled_receipts=10
matmul_settled=true
linear_training_settled=true
rewarded_miners=9
finality_rate_bps=10000
data_availability_bps=10000
public_evidence_full_spec=false
independently_checkable=false
STATUS
