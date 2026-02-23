#!/usr/bin/env bash
#
# Generate a systemd service file for scripts/rpc.sh
#
set -euo pipefail

script_dir="$(readlink -f "$(dirname "$0")")"
repo_root="$(readlink -f "$script_dir/..")"

output_path="${1:-$repo_root/agave-rpc.service}"
service_user="${SERVICE_USER:-$(id -un)}"
service_group="${SERVICE_GROUP:-$(id -gn)}"
env_file="${ENV_FILE:-/etc/default/agave-rpc}"
ledger_mount="${LEDGER_MOUNT:-/mnt/fd-ledger}"
accounts_mount="${ACCOUNTS_MOUNT:-/mnt/fd-accounts}"

cat >"$output_path" <<EOF
[Unit]
Description=Agave RPC Node
After=network-online.target local-fs.target
Wants=network-online.target
RequiresMountsFor=$ledger_mount $accounts_mount

[Service]
Type=simple
User=$service_user
Group=$service_group
WorkingDirectory=$repo_root
EnvironmentFile=-$env_file
Environment=AGAVE_DISABLE_SHRED_SIGVERIFY=1
ExecStart=$repo_root/scripts/rpc.sh
Restart=always
RestartSec=5
LimitNOFILE=2000000
LimitMEMLOCK=infinity
TasksMax=infinity
TimeoutStopSec=120
KillSignal=SIGINT

[Install]
WantedBy=multi-user.target
EOF

echo "Wrote $output_path"
echo
echo "Install (system-wide):"
echo "  sudo cp $output_path /etc/systemd/system/agave-rpc.service"
echo "  sudo systemctl daemon-reload"
echo "  sudo systemctl enable --now agave-rpc.service"
echo
echo "Optional env overrides file:"
echo "  $env_file"
echo "Example keys: IDENTITY_PATH, LEDGER_PATH, ACCOUNTS_PATH, RPC_BIND_ADDRESS, RPC_PORT"
