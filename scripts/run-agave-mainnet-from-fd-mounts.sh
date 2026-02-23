#!/usr/bin/env bash
#
# Run this Agave fork against the existing Firedancer-backed mountpoints.
# Intended for RPC-only mode reusing:
#   - /mnt/fd-ledger/ledger
#   - /mnt/fd-accounts/accounts
#
set -euo pipefail

script_dir="$(readlink -f "$(dirname "$0")")"
cd "$script_dir/.."

profile="${CARGO_BUILD_PROFILE:-release}"
validator_bin="${AGAVE_VALIDATOR_BIN:-$PWD/target/$profile/agave-validator}"
if [[ ! -x "$validator_bin" ]]; then
  if command -v agave-validator >/dev/null 2>&1; then
    validator_bin="$(command -v agave-validator)"
  else
    echo "agave-validator binary not found. Build first, for example:" >&2
    echo "  cargo build --release -p agave-validator" >&2
    exit 1
  fi
fi

ledger_mount="${LEDGER_MOUNT:-/mnt/fd-ledger}"
accounts_mount="${ACCOUNTS_MOUNT:-/mnt/fd-accounts}"
ledger_path="${LEDGER_PATH:-$ledger_mount/ledger}"
accounts_path="${ACCOUNTS_PATH:-$accounts_mount/accounts}"

identity_path="${IDENTITY_PATH:-/home/sol/validator-keypair.json}"
expected_genesis_hash="${EXPECTED_GENESIS_HASH:-5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d}"
rpc_bind_address="${RPC_BIND_ADDRESS:-0.0.0.0}"
rpc_port="${RPC_PORT:-8899}"
gossip_port="${GOSSIP_PORT:-8001}"
dynamic_port_range="${DYNAMIC_PORT_RANGE:-8000-8020}"
log_path="${LOG_PATH:--}"

entrypoints=(
  "${ENTRYPOINT_1:-entrypoint.mainnet-beta.solana.com:8001}"
  "${ENTRYPOINT_2:-entrypoint2.mainnet-beta.solana.com:8001}"
  "${ENTRYPOINT_3:-entrypoint3.mainnet-beta.solana.com:8001}"
  "${ENTRYPOINT_4:-entrypoint4.mainnet-beta.solana.com:8001}"
  "${ENTRYPOINT_5:-entrypoint5.mainnet-beta.solana.com:8001}"
)

require_mountpoint() {
  local mp="$1"
  if command -v mountpoint >/dev/null 2>&1; then
    if ! mountpoint -q "$mp"; then
      echo "$mp is not mounted. Refusing to run to avoid writing to the root disk." >&2
      exit 1
    fi
    return
  fi
  if ! awk -v mount="$mp" '$2==mount{found=1} END{exit !found}' /proc/mounts; then
    echo "$mp is not mounted. Refusing to run to avoid writing to the root disk." >&2
    exit 1
  fi
}

require_mountpoint "$ledger_mount"
require_mountpoint "$accounts_mount"

mkdir -p "$ledger_path" "$accounts_path"
[[ -d "$ledger_path" && -w "$ledger_path" ]] || {
  echo "Ledger path is not writable: $ledger_path" >&2
  exit 1
}
[[ -d "$accounts_path" && -w "$accounts_path" ]] || {
  echo "Accounts path is not writable: $accounts_path" >&2
  exit 1
}
[[ -f "$identity_path" ]] || {
  echo "Validator identity keypair not found: $identity_path" >&2
  exit 1
}

if ps -eo cmd | grep -Eq "[a]gave-validator|[s]olana-validator|[f]dctl run|[f]iredancer-dev"; then
  echo "A validator/Firedancer process appears to already be running. Stop it first." >&2
  ps -eo pid,cmd | grep -E "[a]gave-validator|[s]olana-validator|[f]dctl run|[f]iredancer-dev" >&2 || true
  exit 1
fi

# Remove stale Firedancer admin socket from prior runs, if present.
if [[ -S "$ledger_path/admin.rpc" ]]; then
  rm -f "$ledger_path/admin.rpc"
fi

args=(
  --identity "$identity_path"
  --ledger "$ledger_path"
  --accounts "$accounts_path"
  --expected-genesis-hash "$expected_genesis_hash"
  --gossip-port "$gossip_port"
  --dynamic-port-range "$dynamic_port_range"
  --rpc-bind-address "$rpc_bind_address"
  --rpc-port "$rpc_port"
  --full-rpc-api
  --enable-rpc-transaction-history
  --enable-extended-tx-metadata-storage
  --no-voting
  --log "$log_path"
)

for ep in "${entrypoints[@]}"; do
  args+=(--entrypoint "$ep")
done

if [[ "${PRIVATE_RPC:-0}" == "1" ]]; then
  args+=(--private-rpc)
fi
if [[ -n "${WAL_RECOVERY_MODE:-}" ]]; then
  args+=(--wal-recovery-mode "$WAL_RECOVERY_MODE")
fi
if [[ "${SKIP_STARTUP_LEDGER_VERIFICATION:-0}" == "1" ]]; then
  args+=(--skip-startup-ledger-verification)
fi
if [[ "${NO_PORT_CHECK:-0}" == "1" ]]; then
  args+=(--no-port-check)
fi

echo "Starting Agave with:"
echo "  binary:     $validator_bin"
echo "  ledger:     $ledger_path"
echo "  accountsdb: $accounts_path"
echo "  rpc:        $rpc_bind_address:$rpc_port"

exec "$validator_bin" "${args[@]}" "$@"
