#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  echo "Run as root (example: sudo ./scripts/apply-agave-system-tuning.sh)" >&2
  exit 1
fi

sysctl_file="/etc/sysctl.d/21-agave-validator.conf"
limits_file="/etc/security/limits.d/90-solana-nofiles.conf"
systemd_manager_dir="/etc/systemd/system.conf.d"
systemd_manager_file="${systemd_manager_dir}/99-agave-limits.conf"

cat >"${sysctl_file}" <<'EOF'
# Increase max UDP buffer sizes
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728

# Increase memory mapped files limit
vm.max_map_count = 1000000

# Increase number of allowed open file descriptors
fs.nr_open = 1000000
EOF

cat >"${limits_file}" <<'EOF'
* - nofile 1000000
* - memlock 2000000
EOF

mkdir -p "${systemd_manager_dir}"
cat >"${systemd_manager_file}" <<'EOF'
[Manager]
DefaultLimitNOFILE=1000000
DefaultLimitMEMLOCK=2000000000
EOF

echo "Applying kernel settings from ${sysctl_file}..."
sysctl -p "${sysctl_file}" >/dev/null

echo "Reloading systemd manager..."
systemctl daemon-reexec
systemctl daemon-reload

echo
echo "Applied tuning files:"
echo "  ${sysctl_file}"
echo "  ${limits_file}"
echo "  ${systemd_manager_file}"
echo
echo "Current values:"
sysctl net.core.rmem_max net.core.wmem_max vm.max_map_count fs.nr_open

service_name="${1:-}"
if [[ -n "${service_name}" ]]; then
  echo
  echo "Restarting ${service_name}..."
  systemctl restart "${service_name}"
  systemctl --no-pager --full status "${service_name}" || true
else
  echo
  echo "Next step:"
  echo "  sudo systemctl restart agave-rpc.service"
fi
