#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
log_path="${HOME:-/home/sol}/validator.log"

exec "${AGAVE_VALIDATOR_BIN:-./target/release/agave-validator}" \
  --identity /home/sol/validator-keypair.json \
  --ledger /mnt/fd-ledger/ledger \
  --accounts /mnt/fd-accounts/accounts \
  --geyser-plugin-config ./scripts/yellowstone-config.json \
  --expected-genesis-hash 5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d \
  --entrypoint entrypoint.mainnet-beta.solana.com:8001 \
  --entrypoint entrypoint2.mainnet-beta.solana.com:8001 \
  --entrypoint entrypoint3.mainnet-beta.solana.com:8001 \
  --entrypoint entrypoint4.mainnet-beta.solana.com:8001 \
  --entrypoint entrypoint5.mainnet-beta.solana.com:8001 \
  --gossip-port 8001 \
  --dynamic-port-range 8000-8025 \
  --rpc-bind-address 0.0.0.0 \
  --rpc-port 8899 \
  --full-rpc-api \
  --no-voting \
  --log "${log_path}" \
  "$@"
