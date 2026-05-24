#!/usr/bin/env sh

LOCAL_CPU_MINERS="miner-00 miner-01 miner-02 miner-03 miner-04 miner-05 miner-06 miner-07 miner-08 miner-09"
LOCAL_CPU_VALIDATORS="validator-00 validator-01 validator-02 validator-03 validator-04"
LOCAL_CPU_EXPECTED_SERVICES="$LOCAL_CPU_MINERS $LOCAL_CPU_VALIDATORS"

local_cpu_count_words() {
  count=0
  for item in "$@"; do
    count=$((count + 1))
  done
  printf '%s\n' "$count"
}

LOCAL_CPU_MINER_COUNT=$(local_cpu_count_words $LOCAL_CPU_MINERS)
LOCAL_CPU_VALIDATOR_COUNT=$(local_cpu_count_words $LOCAL_CPU_VALIDATORS)
LOCAL_CPU_EXPECTED_SERVICE_COUNT=$(local_cpu_count_words $LOCAL_CPU_EXPECTED_SERVICES)
LOCAL_CPU_EXPECTED_SETTLED_RECEIPTS="$LOCAL_CPU_MINER_COUNT"
LOCAL_CPU_CUDA_REQUIRED_MINER_COUNT=0
LOCAL_CPU_BOOTSTRAP_SERVICE=miner-00
LOCAL_CPU_NETWORK_OBSERVER_SERVICE=miner-01
LOCAL_CPU_SEED_HEIGHT=2
LOCAL_CPU_SEED_BLOCKS=2
LOCAL_CPU_FULL_RATE_BPS=10000
