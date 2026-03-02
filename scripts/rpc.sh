#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
log_path="${HOME:-/home/sol}/validator.log"

validator_bin="${AGAVE_VALIDATOR_BIN:-./target/release/agave-validator}"

args=(
  --identity /home/sol/validator-keypair.json
  --ledger /mnt/fd-ledger/ledger
  --accounts /mnt/fd-accounts/accounts
  --geyser-plugin-config ./scripts/yellowstone-config.json
  --expected-genesis-hash 5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d
  --entrypoint entrypoint.mainnet-beta.solana.com:8001
  --entrypoint entrypoint2.mainnet-beta.solana.com:8001
  --entrypoint entrypoint3.mainnet-beta.solana.com:8001
  --entrypoint entrypoint4.mainnet-beta.solana.com:8001
  --entrypoint entrypoint5.mainnet-beta.solana.com:8001
  --gossip-port 8001
  --dynamic-port-range 8000-10000
  --rpc-port 8899
  --full-rpc-api
  --no-voting
  --limit-ledger-size
  --minimal-snapshot-download-speed "${MINIMAL_SNAPSHOT_DOWNLOAD_SPEED:-524288000}"
  --rpc-bind-address 127.0.0.1
  --private-rpc
  --experimental-poh-pinned-cpu-core "${AGAVE_POH_PINNED_CPU_CORE:-10}"
  --log "$log_path"
)

# Run with XDP by default; pin to a real NIC unless overridden.
if [[ "${AGAVE_ENABLE_RETRANSMIT_XDP:-1}" == "1" ]]; then
  args+=(--experimental-retransmit-xdp-cpu-cores "${AGAVE_RETRANSMIT_XDP_CPU_CORES:-1}")
  args+=(--experimental-retransmit-xdp-interface "${AGAVE_RETRANSMIT_XDP_INTERFACE:-enp5s0f0}")

  if [[ "${AGAVE_RETRANSMIT_XDP_ZERO_COPY:-0}" == "1" ]]; then
    args+=(--experimental-retransmit-xdp-zero-copy)
  fi
fi

exec "$validator_bin" "${args[@]}"
