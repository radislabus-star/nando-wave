#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
UNIT_SOURCE="${ROOT_DIR}/ops/remote-backend/nando-client-connector.service"
BINARY_SOURCE=""
INSTALL_BIN="${HOME}/.local/bin/nando-connector"
INSTALL_UNIT="${HOME}/.config/systemd/user/nando-client-connector.service"
SERVICE="nando-client-connector.service"
METRICS_URL="${NANDO_CONNECTOR_INSTALL_METRICS_URL:-http://127.0.0.1:18786/metrics}"
HEALTH_URL="${NANDO_CONNECTOR_INSTALL_HEALTH_URL:-http://127.0.0.1:8787/health}"
READINESS_ATTEMPTS="${NANDO_CONNECTOR_INSTALL_READINESS_ATTEMPTS:-30}"
READINESS_SLEEP_SECONDS="${NANDO_CONNECTOR_INSTALL_READINESS_SLEEP_SECONDS:-0.25}"
DRAIN_WAIT_SECONDS=0
DRAIN_POLL_SECONDS="${NANDO_CONNECTOR_INSTALL_DRAIN_POLL_SECONDS:-0.25}"
DRAIN_SAMPLE_SLEEP_SECONDS="${NANDO_CONNECTOR_INSTALL_DRAIN_SAMPLE_SLEEP_SECONDS:-0.1}"

usage() {
  cat <<'EOF'
Install a tested Nando connector only when the current connector is drained.

Usage:
  ops/remote-backend/install-client-connector.sh \
    --binary /path/to/nando-connector \
    [--wait-for-drain SECONDS]

Exit 75 means active client connections prevented activation. No installed
file or running process is changed in that case.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary)
      BINARY_SOURCE="${2:-}"
      shift 2
      ;;
    --wait-for-drain)
      DRAIN_WAIT_SECONDS="${2:-}"
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
  printf 'connector binary is not executable: %s\n' "${BINARY_SOURCE}" >&2
  exit 2
fi
if [[ ! "${DRAIN_WAIT_SECONDS}" =~ ^[0-9]+$ ]]; then
  printf 'wait-for-drain must be a non-negative integer number of seconds\n' >&2
  exit 2
fi

work="$(mktemp -d)"
candidate_binary="${work}/nando-connector"
candidate_unit="${work}/nando-client-connector.service"
verify_unit="${work}/nando-client-connector.verify.service"
backup_binary="${work}/previous-binary"
backup_unit="${work}/previous-unit"
had_binary=0
had_unit=0
service_was_active=0
service_was_enabled=0
rollback_armed=0

cleanup() {
  rm -rf "${work}"
}

rollback() {
  local rc="${1:-1}"
  trap - ERR INT TERM EXIT
  set +e
  if [[ "${rollback_armed}" == "1" ]]; then
    systemctl --user stop "${SERVICE}"
    if [[ "${had_binary}" == "1" ]]; then
      install -m 0755 "${backup_binary}" "${INSTALL_BIN}"
    else
      rm -f "${INSTALL_BIN}"
    fi
    if [[ "${had_unit}" == "1" ]]; then
      install -m 0644 "${backup_unit}" "${INSTALL_UNIT}"
    else
      rm -f "${INSTALL_UNIT}"
    fi
    systemctl --user daemon-reload
    if [[ "${service_was_enabled}" == "0" ]]; then
      systemctl --user disable "${SERVICE}"
    fi
    if [[ "${service_was_active}" == "1" ]]; then
      systemctl --user start "${SERVICE}"
    fi
    printf 'connector install failed; previous binary and unit restored\n' >&2
  fi
  cleanup
  exit "${rc}"
}

trap 'rollback $?' ERR
trap 'rollback 130' INT
trap 'rollback 143' TERM
trap cleanup EXIT

active_connection_count() {
  curl -fsS --max-time 2 "${METRICS_URL}" |
    jq -er '.active_connections | select(type == "number" and . >= 0)'
}

wait_for_drain() {
  local active_connections
  local deadline=$((SECONDS + DRAIN_WAIT_SECONDS))
  local _sample

  while true; do
    for _sample in 1 2; do
      active_connections="$(active_connection_count)"
      if [[ "${active_connections}" != "0" ]]; then
        break
      fi
      if [[ "${_sample}" == "1" ]]; then
        sleep "${DRAIN_SAMPLE_SLEEP_SECONDS}"
      fi
    done
    if [[ "${active_connections}" == "0" ]]; then
      return 0
    fi
    if [[ "${DRAIN_WAIT_SECONDS}" == "0" || "${SECONDS}" -ge "${deadline}" ]]; then
      printf 'connector activation deferred: %s active connection(s)\n' \
        "${active_connections}" >&2
      return 75
    fi
    sleep "${DRAIN_POLL_SECONDS}"
  done
}

install -m 0755 "${BINARY_SOURCE}" "${candidate_binary}"
install -m 0644 "${UNIT_SOURCE}" "${candidate_unit}"
sed \
  -e "s#%h/.local/bin/nando-connector#${candidate_binary}#g" \
  "${candidate_unit}" >"${verify_unit}"

"${candidate_binary}" \
  --listen 127.0.0.1:18787 \
  --metrics-listen 127.0.0.1:18785 \
  --upstream 192.168.3.94:8787 \
  --client-fallback \
  --spool-dir "${work}/spool" \
  --route-receipts "${work}/route-receipts-v1.jsonl" \
  --check >/dev/null
systemd-analyze --user verify "${verify_unit}"

if systemctl --user is-active --quiet "${SERVICE}"; then
  service_was_active=1
  if ! wait_for_drain; then
    exit 75
  fi
fi
if systemctl --user is-enabled --quiet "${SERVICE}"; then
  service_was_enabled=1
fi

install -d -m 0755 "$(dirname "${INSTALL_BIN}")"
install -d -m 0700 "$(dirname "${INSTALL_UNIT}")"
if [[ -f "${INSTALL_BIN}" ]]; then
  cp -a "${INSTALL_BIN}" "${backup_binary}"
  had_binary=1
fi
if [[ -f "${INSTALL_UNIT}" ]]; then
  cp -a "${INSTALL_UNIT}" "${backup_unit}"
  had_unit=1
fi

rollback_armed=1
install -m 0755 "${candidate_binary}" "${INSTALL_BIN}"
install -m 0644 "${candidate_unit}" "${INSTALL_UNIT}"
systemctl --user daemon-reload
systemctl --user enable "${SERVICE}" >/dev/null
if [[ "${service_was_active}" == "1" ]]; then
  systemctl --user restart "${SERVICE}"
else
  systemctl --user start "${SERVICE}"
fi

for _attempt in $(seq 1 "${READINESS_ATTEMPTS}"); do
  if systemctl --user is-active --quiet "${SERVICE}" \
    && curl -fsS --max-time 2 "${HEALTH_URL}" |
      jq -e '
        .ok == true
        and .service == "nando-nginx-gateway"
        and .transport == "nginx"
      ' >/dev/null 2>&1 \
    && curl -fsS --max-time 2 "${METRICS_URL}" |
      jq -e '
        .ok == true
        and .service == "nando-connector"
        and (.route_receipts | type) == "number"
        and (.route_receipt_failures | type) == "number"
      ' >/dev/null 2>&1; then
    rollback_armed=0
    printf 'Nando connector installed and route receipts enabled\n'
    exit 0
  fi
  sleep "${READINESS_SLEEP_SECONDS}"
done

printf 'connector readiness timed out\n' >&2
rollback 1
