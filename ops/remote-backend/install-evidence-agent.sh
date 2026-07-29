#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
UNIT_SOURCE="${ROOT_DIR}/ops/remote-backend/nando-evidence-agent.service"
BINARY_SOURCE=""
REMOTE_ORIGIN="${NANDO_EVIDENCE_REMOTE_ORIGIN:-http://192.168.3.94:8787}"
INSTALL_BIN="${HOME}/.local/bin/nando-evidence-agent"
INSTALL_UNIT="${HOME}/.config/systemd/user/nando-evidence-agent.service"
KEY_FILE="${HOME}/.config/nando/evidence-agent.key"
STATE_DIR="${HOME}/.local/state/nando-evidence-agent"
SESSIONS_DIR="${HOME}/.codex/sessions"
READINESS_ATTEMPTS="${NANDO_EVIDENCE_READINESS_ATTEMPTS:-30}"
READINESS_SLEEP_SECONDS="${NANDO_EVIDENCE_READINESS_SLEEP_SECONDS:-0.25}"

usage() {
  cat <<'EOF'
Install the local compact evidence agent without touching the Nando connector.

Usage:
  ops/remote-backend/install-evidence-agent.sh \
    --binary /path/to/nando-evidence-agent \
    [--server http://192.168.3.94:8787]

The 32-byte client key must already exist at:
  ~/.config/nando/evidence-agent.key
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary)
      BINARY_SOURCE="${2:-}"
      shift 2
      ;;
    --server)
      REMOTE_ORIGIN="${2:-}"
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
  printf 'evidence agent binary is not executable: %s\n' "${BINARY_SOURCE}" >&2
  exit 2
fi
if [[ ! "${REMOTE_ORIGIN}" =~ ^http://[A-Za-z0-9.-]+:[0-9]{2,5}$ ]]; then
  printf 'server must be a private-LAN HTTP origin\n' >&2
  exit 2
fi
if [[ ! -f "${KEY_FILE}" ]]; then
  printf 'client key is missing: %s\n' "${KEY_FILE}" >&2
  exit 2
fi
if [[ "$(stat -c '%a' "${KEY_FILE}")" != "600" ]]; then
  printf 'client key must have mode 0600: %s\n' "${KEY_FILE}" >&2
  exit 2
fi
if [[ ! -d "${SESSIONS_DIR}" ]]; then
  printf 'Codex sessions directory is missing: %s\n' "${SESSIONS_DIR}" >&2
  exit 2
fi

install -d -m 0700 \
  "$(dirname "${KEY_FILE}")" \
  "${STATE_DIR}" \
  "$(dirname "${INSTALL_UNIT}")"
install -d -m 0755 "$(dirname "${INSTALL_BIN}")"

work="$(mktemp -d)"
candidate_binary="${work}/nando-evidence-agent"
candidate_unit="${work}/nando-evidence-agent.service"
verify_unit="${work}/nando-evidence-agent.verify.service"
check_state="${work}/check-state"
backup_binary="${work}/previous-binary"
backup_unit="${work}/previous-unit"
had_binary=0
had_unit=0
service_was_active=0
service_was_enabled=0
rollback_armed=0

# Invoked through EXIT/rollback traps.
# shellcheck disable=SC2329
cleanup() {
  rm -rf "${work}"
}

# Invoked through ERR/INT/TERM traps.
# shellcheck disable=SC2329
rollback() {
  local rc="${1:-1}"
  trap - ERR INT TERM EXIT
  set +e
  if [[ "${rollback_armed}" == "1" ]]; then
    systemctl --user stop nando-evidence-agent.service
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
      systemctl --user disable nando-evidence-agent.service
    fi
    if [[ "${service_was_active}" == "1" ]]; then
      systemctl --user start nando-evidence-agent.service
    fi
    printf 'evidence agent install failed; previous version restored\n' >&2
  fi
  cleanup
  exit "${rc}"
}

trap 'rollback $?' ERR
trap 'rollback 130' INT
trap 'rollback 143' TERM
trap cleanup EXIT

install -m 0755 "${BINARY_SOURCE}" "${candidate_binary}"
sed \
  -e "s#http://192\\.168\\.3\\.94:8787#${REMOTE_ORIGIN}#g" \
  "${UNIT_SOURCE}" > "${candidate_unit}"
sed \
  -e "s#%h/.local/bin/nando-evidence-agent#${candidate_binary}#g" \
  "${candidate_unit}" > "${verify_unit}"

mkdir -m 0700 "${check_state}"
"${candidate_binary}" \
  --server "${REMOTE_ORIGIN}" \
  --sessions-dir "${SESSIONS_DIR}" \
  --key-file "${KEY_FILE}" \
  --state-dir "${check_state}" \
  --route-receipts "${work}/route-receipts-v1.jsonl" \
  --check >/dev/null
systemd-analyze --user verify "${verify_unit}"

if [[ -f "${INSTALL_BIN}" ]]; then
  cp -a "${INSTALL_BIN}" "${backup_binary}"
  had_binary=1
fi
if [[ -f "${INSTALL_UNIT}" ]]; then
  cp -a "${INSTALL_UNIT}" "${backup_unit}"
  had_unit=1
fi
if systemctl --user is-active --quiet nando-evidence-agent.service; then
  service_was_active=1
fi
if systemctl --user is-enabled --quiet nando-evidence-agent.service; then
  service_was_enabled=1
fi

rollback_armed=1
install -m 0755 "${candidate_binary}" "${INSTALL_BIN}"
install -m 0644 "${candidate_unit}" "${INSTALL_UNIT}"
systemctl --user daemon-reload
systemctl --user enable nando-evidence-agent.service >/dev/null
if [[ "${service_was_active}" == "1" ]]; then
  systemctl --user restart nando-evidence-agent.service
else
  systemctl --user start nando-evidence-agent.service
fi

for _attempt in $(seq 1 "${READINESS_ATTEMPTS}"); do
  if systemctl --user is-active --quiet nando-evidence-agent.service; then
    sleep "${READINESS_SLEEP_SECONDS}"
    if systemctl --user is-active --quiet nando-evidence-agent.service; then
      rollback_armed=0
      printf 'Nando evidence agent active; connector unchanged\n'
      exit 0
    fi
  fi
  sleep "${READINESS_SLEEP_SECONDS}"
done

printf 'evidence agent readiness timed out\n' >&2
rollback 1
