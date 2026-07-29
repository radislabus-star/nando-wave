#!/usr/bin/env bash
set -euo pipefail

BINARY_SOURCE=""
SERVICE="${NANDO_GATEWAY_CONTROL_SERVICE:-nando-gateway-control.service}"
INSTALL_BINARY="${NANDO_GATEWAY_CONTROL_BINARY:-/opt/nando-wave/bin/nando-gateway-control}"
CONTROL_HEALTH="${NANDO_GATEWAY_CONTROL_HEALTH:-http://127.0.0.1:18788/health}"
HOT_HEALTH="${NANDO_GATEWAY_CONTROL_HOT_HEALTH:-http://127.0.0.1:18789/health}"
EDGE_HEALTH="${NANDO_GATEWAY_CONTROL_EDGE_HEALTH:-http://127.0.0.1:8787/health}"
READINESS_ATTEMPTS="${NANDO_GATEWAY_CONTROL_READINESS_ATTEMPTS:-20}"
READINESS_SLEEP_SECONDS="${NANDO_GATEWAY_CONTROL_READINESS_SLEEP_SECONDS:-0.25}"

usage() {
  cat <<'EOF'
Transactionally replace the remote Nando gateway-control binary.

Usage:
  ops/remote-backend/install-gateway-control.sh \
    --binary /path/to/nando-gateway-control

The data-plane Nginx and hot serving services are never restarted.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary)
      BINARY_SOURCE="${2:-}"
      shift 2
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      printf 'unknown argument: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ ! -x "${BINARY_SOURCE}" ]]; then
  printf 'gateway-control binary is not executable: %s\n' "${BINARY_SOURCE}" >&2
  exit 2
fi
if [[ ! -f "${INSTALL_BINARY}" ]]; then
  printf 'installed gateway-control binary is missing: %s\n' "${INSTALL_BINARY}" >&2
  exit 2
fi
if ! sudo -n true; then
  printf 'passwordless sudo is required\n' >&2
  exit 2
fi

work="$(mktemp -d)"
candidate_binary="${work}/nando-gateway-control"
backup_binary="${work}/previous-binary"
control_was_active=0
rollback_armed=0

cleanup() {
  rm -rf "${work}"
}

rollback() {
  local rc="${1:-1}"
  trap - ERR INT TERM EXIT
  set +e
  if [[ "${rollback_armed}" == "1" ]]; then
    sudo -n systemctl stop "${SERVICE}"
    sudo -n install -m 0755 "${backup_binary}" "${INSTALL_BINARY}"
    if [[ "${control_was_active}" == "1" ]]; then
      sudo -n systemctl start "${SERVICE}"
    fi
    printf 'gateway-control install failed; previous binary restored\n' >&2
  fi
  cleanup
  exit "${rc}"
}

trap 'rollback $?' ERR
trap 'rollback 130' INT
trap 'rollback 143' TERM
trap cleanup EXIT

curl -fsS --max-time 2 "${HOT_HEALTH}" | jq -e '.ok == true' >/dev/null
curl -fsS --max-time 2 "${EDGE_HEALTH}" |
  jq -e '.ok == true and .service == "nando-nginx-gateway"' >/dev/null
install -m 0755 "${BINARY_SOURCE}" "${candidate_binary}"
cp -a "${INSTALL_BINARY}" "${backup_binary}"
if systemctl is-active --quiet "${SERVICE}"; then
  control_was_active=1
  sudo -n systemctl stop "${SERVICE}"
fi

rollback_armed=1
sudo -n install -m 0755 "${candidate_binary}" "${INSTALL_BINARY}.candidate.$$"
sudo -n mv -f "${INSTALL_BINARY}.candidate.$$" "${INSTALL_BINARY}"
sudo -n systemctl start "${SERVICE}"

for _attempt in $(seq 1 "${READINESS_ATTEMPTS}"); do
  if curl -fsS --max-time 2 "${CONTROL_HEALTH}" |
    jq -e '.ok == true and .service == "nando-gateway-control"' >/dev/null 2>&1; then
    curl -fsS --max-time 2 "${HOT_HEALTH}" | jq -e '.ok == true' >/dev/null
    curl -fsS --max-time 2 "${EDGE_HEALTH}" |
      jq -e '.ok == true and .service == "nando-nginx-gateway"' >/dev/null
    rollback_armed=0
    printf 'gateway-control ready; data-plane services stayed online\n'
    exit 0
  fi
  sleep "${READINESS_SLEEP_SECONDS}"
done

printf 'gateway-control readiness timed out\n' >&2
rollback 1
