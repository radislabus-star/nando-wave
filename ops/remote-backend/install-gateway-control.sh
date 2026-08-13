#!/usr/bin/env bash
set -euo pipefail

BINARY_SOURCE=""
SIDECAR_SOURCE=""
SERVICE="${NANDO_GATEWAY_CONTROL_SERVICE:-nando-gateway-control.service}"
INSTALL_BINARY="${NANDO_GATEWAY_CONTROL_BINARY:-/opt/nando-wave/bin/nando-gateway-control}"
INSTALL_SIDECAR="${NANDO_S1C3_OPERATIONAL_STATUS_JSON:-/var/lib/nando-wave/transition/grounded-meaning-v1/s1c3-operational-status-v1.json}"
CONTROL_HEALTH="${NANDO_GATEWAY_CONTROL_HEALTH:-http://127.0.0.1:18788/health}"
CONTROL_BASE="${NANDO_GATEWAY_CONTROL_BASE:-http://127.0.0.1:18788/control}"
CONTROL_DASHBOARD_KEY="${NANDO_GATEWAY_CONTROL_DASHBOARD_KEY:-}"
HOT_HEALTH="${NANDO_GATEWAY_CONTROL_HOT_HEALTH:-http://127.0.0.1:18789/health}"
EDGE_HEALTH="${NANDO_GATEWAY_CONTROL_EDGE_HEALTH:-http://192.168.3.94:8787/health}"
READINESS_ATTEMPTS="${NANDO_GATEWAY_CONTROL_READINESS_ATTEMPTS:-20}"
READINESS_SLEEP_SECONDS="${NANDO_GATEWAY_CONTROL_READINESS_SLEEP_SECONDS:-0.25}"

usage() {
  cat <<'EOF'
Transactionally replace the remote Nando gateway-control binary.

Usage:
  ops/remote-backend/install-gateway-control.sh \
    --binary /path/to/nando-gateway-control \
    [--sidecar /path/to/s1c3-operational-status-v1.json]

The data-plane Nginx and hot serving services are never restarted.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary)
      BINARY_SOURCE="${2:-}"
      shift 2
      ;;
    --sidecar)
      SIDECAR_SOURCE="${2:-}"
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
if [[ -n "${SIDECAR_SOURCE}" && ! -f "${SIDECAR_SOURCE}" ]]; then
  printf 'S1C operational sidecar is missing: %s\n' "${SIDECAR_SOURCE}" >&2
  exit 2
fi
if ! sudo -n true; then
  printf 'passwordless sudo is required\n' >&2
  exit 2
fi

work="$(mktemp -d)"
candidate_binary="${work}/nando-gateway-control"
backup_binary="${work}/previous-binary"
candidate_sidecar="${work}/s1c3-operational-status-v1.json"
backup_sidecar="${work}/previous-sidecar"
sidecar_was_present=0
control_was_active=0
rollback_armed=0

cleanup() {
  rm -rf "${work}"
}

load_dashboard_key() {
  if [[ -n "${CONTROL_DASHBOARD_KEY}" ]]; then
    return
  fi

  local control_pid
  control_pid="$(systemctl show --property MainPID --value "${SERVICE}")"
  if [[ ! "${control_pid}" =~ ^[1-9][0-9]*$ ]]; then
    printf 'cannot identify the running gateway-control process\n' >&2
    exit 2
  fi
  CONTROL_DASHBOARD_KEY="$(
    sudo -n cat "/proc/${control_pid}/environ" |
      tr '\0' '\n' |
      sed -n 's/^NANDO_STATUS_DASHBOARD_KEY=//p' |
      head -n 1
  )"
  if [[ -z "${CONTROL_DASHBOARD_KEY}" ]]; then
    printf 'gateway-control dashboard key is unavailable\n' >&2
    exit 2
  fi
}

projection_is_exact() {
  printf 'url = "%s/%s/api/v1/dashboard"\n' \
    "${CONTROL_BASE}" "${CONTROL_DASHBOARD_KEY}" |
    curl --config - --fail --silent --show-error --max-time 2 |
    jq -e '
      .available == true and
      .dashboard_build == "2026.08.13-control-v19" and
      .s1c3_operational.stage == "S1C-3H" and
      .s1c3_operational.verdict == "S1C3H_DEPLOYMENT_PASS" and
      .s1c3_operational.capture_installed == true and
      .s1c3_operational.natural_record_count == 0 and
      .s1c3_operational.s1c4_state == "COLLECTING" and
      .s1c3_operational.authority_ready == false and
      .s1c3_operational.scientific_authority == false and
      .s1c3_operational.model_training_allowed == false and
      .s1c3_operational.phase_mutation_allowed == false
    ' >/dev/null 2>&1
}

rollback() {
  local rc="${1:-1}"
  trap - ERR INT TERM EXIT
  set +e
  if [[ "${rollback_armed}" == "1" ]]; then
    sudo -n systemctl stop "${SERVICE}"
    sudo -n install -m 0755 "${backup_binary}" "${INSTALL_BINARY}"
    if [[ "${sidecar_was_present}" == "1" ]]; then
      sudo -n install -m 0644 "${backup_sidecar}" "${INSTALL_SIDECAR}"
    elif [[ -n "${SIDECAR_SOURCE}" ]]; then
      sudo -n rm -f "${INSTALL_SIDECAR}"
    fi
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
if [[ -n "${SIDECAR_SOURCE}" ]]; then
  load_dashboard_key
fi
install -m 0755 "${BINARY_SOURCE}" "${candidate_binary}"
cp -a "${INSTALL_BINARY}" "${backup_binary}"
if [[ -n "${SIDECAR_SOURCE}" ]]; then
  install -m 0644 "${SIDECAR_SOURCE}" "${candidate_sidecar}"
  if [[ -f "${INSTALL_SIDECAR}" ]]; then
    cp -a "${INSTALL_SIDECAR}" "${backup_sidecar}"
    sidecar_was_present=1
  fi
fi
if systemctl is-active --quiet "${SERVICE}"; then
  control_was_active=1
  sudo -n systemctl stop "${SERVICE}"
fi

rollback_armed=1
if [[ -n "${SIDECAR_SOURCE}" ]]; then
  sudo -n install -m 0644 "${candidate_sidecar}" "${INSTALL_SIDECAR}.candidate.$$"
  sudo -n mv -f "${INSTALL_SIDECAR}.candidate.$$" "${INSTALL_SIDECAR}"
fi
sudo -n install -m 0755 "${candidate_binary}" "${INSTALL_BINARY}.candidate.$$"
sudo -n mv -f "${INSTALL_BINARY}.candidate.$$" "${INSTALL_BINARY}"
sudo -n systemctl start "${SERVICE}"

for _attempt in $(seq 1 "${READINESS_ATTEMPTS}"); do
  if curl -fsS --max-time 2 "${CONTROL_HEALTH}" |
    jq -e '.ok == true and .service == "nando-gateway-control"' >/dev/null 2>&1; then
    curl -fsS --max-time 2 "${HOT_HEALTH}" | jq -e '.ok == true' >/dev/null
    curl -fsS --max-time 2 "${EDGE_HEALTH}" |
      jq -e '.ok == true and .service == "nando-nginx-gateway"' >/dev/null
    if [[ -n "${SIDECAR_SOURCE}" ]] && ! projection_is_exact; then
      printf 'gateway-control S1C-3H API projection mismatch\n' >&2
      rollback 1
    fi
    rollback_armed=0
    printf 'gateway-control ready; data-plane services stayed online\n'
    exit 0
  fi
  sleep "${READINESS_SLEEP_SECONDS}"
done

printf 'gateway-control readiness timed out\n' >&2
rollback 1
