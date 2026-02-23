#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

jet_bin="${JET_BIN:-./yellowstone-jet/target/release/jet}"
jet_config="${JET_CONFIG:-./scripts/yellowstone-jet-config.yml}"
jet_prometheus_bind="${JET_PROMETHEUS_BIND:-127.0.0.1:9464}"

if [[ ! -x "$jet_bin" && -x ./yellowstone-jet/target/release/yellowstone-jet ]]; then
  jet_bin=./yellowstone-jet/target/release/yellowstone-jet
fi

if [[ ! -x "$jet_bin" ]]; then
  echo "yellowstone-jet binary not found or not executable: $jet_bin" >&2
  echo "Build it with: cargo build --release -p yellowstone-jet --manifest-path yellowstone-jet/Cargo.toml" >&2
  exit 1
fi

if [[ ! -f "$jet_config" ]]; then
  echo "yellowstone-jet config not found: $jet_config" >&2
  exit 1
fi

exec "$jet_bin" \
  --config "$jet_config" \
  --prometheus "$jet_prometheus_bind" \
  "$@"
