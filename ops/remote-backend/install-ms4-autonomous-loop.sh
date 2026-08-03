#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ADMISSION_BINARY_SOURCE=""
INSTALL_BINARY="${NANDO_MS4_ADMISSION_BINARY:-/opt/nando-wave/bin/nando-response-admission}"
GATE_BINARY="${NANDO_MS4_GATE_BINARY:-/opt/nando-wave/bin/nando-live-transition-gate}"
PROJECT_ROOT="${NANDO_MS4_PROJECT_ROOT:-/opt/nando-wave}"
SYSTEMD_DIR="${NANDO_MS4_SYSTEMD_DIR:-/etc/systemd/system}"
STATE_DIR="${NANDO_MS4_STATE_DIR:-/var/lib/nando-wave/transition}"
HOT_HEALTH="${NANDO_MS4_HOT_HEALTH:-http://127.0.0.1:18789/health}"
READINESS_ATTEMPTS="${NANDO_MS4_READINESS_ATTEMPTS:-40}"
READINESS_SLEEP_SECONDS="${NANDO_MS4_READINESS_SLEEP_SECONDS:-0.25}"

unit_names=(
  nando-response-admission.service
  nando-response-admission.path
  nando-response-admission.timer
  nando-live-transition-gate.service
  nando-live-transition-gate.path
  nando-live-transition-gate.timer
)
watch_units=(
  nando-response-admission.path
  nando-response-admission.timer
  nando-live-transition-gate.path
  nando-live-transition-gate.timer
)
state_files=(
  response-registry.json
  response-admission-controller.json
  response-admission-controller-report.json
  response-admission-controller.marker.json
  response-authority-candidate.json
  admission.json
)

usage() {
  cat <<'EOF'
Install the autonomous MS3 -> MS4 admission loop on the private mini-PC.

Usage:
  ops/remote-backend/install-ms4-autonomous-loop.sh \
    --admission-binary /path/to/nando-response-admission

The installer never restarts Nginx or hot serving. It validates the candidate,
backs up all mutable authority files, installs path/timer workers, and rolls the
whole cold-path transaction back if reconciliation or hot health fails.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --admission-binary)
      ADMISSION_BINARY_SOURCE="${2:-}"
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

if [[ ! -x "${ADMISSION_BINARY_SOURCE}" ]]; then
  printf 'response admission binary is not executable: %s\n' "${ADMISSION_BINARY_SOURCE}" >&2
  exit 2
fi
ADMISSION_BINARY_SOURCE="$(realpath --canonicalize-existing -- "${ADMISSION_BINARY_SOURCE}")"
if [[ ! -x "${GATE_BINARY}" ]]; then
  printf 'live transition gate is not executable: %s\n' "${GATE_BINARY}" >&2
  exit 2
fi
if [[ ! -f "${PROJECT_ROOT}/ops/phase-center-test-server/gates/nando-live-transition-gate.profile.json" ]]; then
  printf 'live transition gate profile is missing under %s\n' "${PROJECT_ROOT}" >&2
  exit 2
fi
if [[ ! -s "${STATE_DIR}/response-admission-candidates.cbor" ]]; then
  printf 'response admission candidate bundle is missing\n' >&2
  exit 2
fi
if ! sudo -n true; then
  printf 'passwordless sudo is required\n' >&2
  exit 2
fi

service_user="$(systemctl show -p User --value nando-response-learning.service)"
service_group="$(systemctl show -p Group --value nando-response-learning.service)"
service_user="${service_user:-$(id -un)}"
service_group="${service_group:-$(id -gn "${service_user}")}"

work="$(mktemp -d)"
rendered_units="${work}/units"
backup_units="${work}/previous-units"
backup_state="${work}/previous-state"
backup_binary="${work}/previous-admission"
mkdir -p "${rendered_units}" "${backup_units}" "${backup_state}"

binary_existed=0
rollback_armed=0
hot_pid_before="$(systemctl show -p MainPID --value nando-transition-serving.service)"
gateway_pid_before="$(systemctl show -p MainPID --value nando-transport-gateway.service)"

cleanup() {
  set +e
  sudo -n rm -rf "${work}"
}

restore_path() {
  local backup="$1"
  local destination="$2"
  if sudo -n test -e "${backup}"; then
    sudo -n cp -a "${backup}" "${destination}.rollback.$$"
    sudo -n mv -f "${destination}.rollback.$$" "${destination}"
  else
    sudo -n rm -f "${destination}"
  fi
}

rollback() {
  local rc="${1:-1}"
  trap - ERR INT TERM EXIT
  set +e
  if [[ "${rollback_armed}" == "1" ]]; then
    for unit in "${watch_units[@]}"; do
      sudo -n systemctl disable --now "${unit}" >/dev/null 2>&1 || true
    done
    for unit in "${unit_names[@]}"; do
      restore_path "${backup_units}/${unit}" "${SYSTEMD_DIR}/${unit}"
    done
    if [[ "${binary_existed}" == "1" ]]; then
      restore_path "${backup_binary}" "${INSTALL_BINARY}"
    else
      sudo -n rm -f "${INSTALL_BINARY}"
    fi
    for name in "${state_files[@]}"; do
      restore_path "${backup_state}/${name}" "${STATE_DIR}/${name}"
    done
    sudo -n systemctl daemon-reload
    while IFS= read -r unit; do
      [[ -n "${unit}" ]] && sudo -n systemctl enable "${unit}" >/dev/null 2>&1 || true
    done <"${work}/enabled-before"
    while IFS= read -r unit; do
      [[ -n "${unit}" ]] && sudo -n systemctl start "${unit}" >/dev/null 2>&1 || true
    done <"${work}/active-before"
    printf 'MS4 autonomous loop install failed; cold authority state restored\n' >&2
  fi
  cleanup
  exit "${rc}"
}

trap 'rollback $?' ERR
trap 'rollback 130' INT
trap 'rollback 143' TERM
trap cleanup EXIT

curl -fsS --max-time 2 "${HOT_HEALTH}" | jq -e '.ok == true' >/dev/null
candidate_contract="$(${ADMISSION_BINARY_SOURCE} --print-runtime-contract-sha256)"
if [[ ! "${candidate_contract}" =~ ^[0-9a-f]{64}$ ]]; then
  printf 'candidate runtime contract is invalid\n' >&2
  exit 2
fi
current_contract="$(jq -r '.response_authority.runtime_build_sha256 // empty' "${STATE_DIR}/admission.json" 2>/dev/null || true)"
if [[ "${current_contract}" =~ ^[0-9a-f]{64}$ ]] && [[ "${candidate_contract}" != "${current_contract}" ]]; then
  printf 'candidate runtime contract differs from the running hot process\n' >&2
  exit 2
fi

for unit in "${unit_names[@]}"; do
  source_path="${ROOT_DIR}/ops/remote-backend/${unit}"
  if [[ "${unit}" == *.service ]]; then
    source_path="${source_path}.template"
    sed \
      -e "s#@NANDO_SERVICE_USER@#${service_user}#g" \
      -e "s#@NANDO_SERVICE_GROUP@#${service_group}#g" \
      -e "s#@NANDO_PROJECT_ROOT@#${PROJECT_ROOT}#g" \
      -e "s#/opt/nando-wave/bin/nando-response-admission#${ADMISSION_BINARY_SOURCE}#g" \
      "${source_path}" >"${rendered_units}/${unit}"
  else
    cp "${source_path}" "${rendered_units}/${unit}"
  fi
done
systemd-analyze verify "${rendered_units}"/*

NANDO_TRANSITION_STATE_DIR="${STATE_DIR}" \
NANDO_RESPONSE_REGISTRY="${work}/preflight-registry.json" \
NANDO_RESPONSE_CONTROLLER_ADMISSION_JSON="${work}/preflight-controller.json" \
NANDO_RESPONSE_AUTHORITY_CANDIDATE="${work}/preflight-authority-candidate.json" \
NANDO_RESPONSE_ADMISSION_REPORT="${work}/preflight-controller-report.json" \
NANDO_LIVE_TRANSITION_GATE_BUILD="${GATE_BINARY}" \
  "${ADMISSION_BINARY_SOURCE}"
jq -e '.verdict == "PASS" and .active_packages > 0' \
  "${work}/preflight-controller-report.json" >/dev/null

profile_source="${PROJECT_ROOT}/ops/phase-center-test-server/gates/nando-live-transition-gate.profile.json"
jq --arg registry "${work}/preflight-registry.json" \
  '.response_runtime.registry = $registry' \
  "${profile_source}" >"${work}/preflight-gate.profile.json"
NANDO_RESPONSE_ADMISSION_BUILD="${ADMISSION_BINARY_SOURCE}" \
NANDO_LIVE_GATE_PROFILE="${work}/preflight-gate.profile.json" \
NANDO_TRANSITION_ADMISSION_JSON="${work}/preflight-gate.json" \
  "${GATE_BINARY}" --status-mode --project-root "${PROJECT_ROOT}" >/dev/null
jq -e '.verdict == "PASS" and .eligible_for_local_accept == true' \
  "${work}/preflight-gate.json" >/dev/null

: >"${work}/enabled-before"
: >"${work}/active-before"
for unit in "${unit_names[@]}"; do
  if sudo -n test -e "${SYSTEMD_DIR}/${unit}"; then
    sudo -n cp -a "${SYSTEMD_DIR}/${unit}" "${backup_units}/${unit}"
  fi
done
for unit in "${watch_units[@]}"; do
  systemctl is-enabled --quiet "${unit}" && printf '%s\n' "${unit}" >>"${work}/enabled-before" || true
  systemctl is-active --quiet "${unit}" && printf '%s\n' "${unit}" >>"${work}/active-before" || true
done
if sudo -n test -e "${INSTALL_BINARY}"; then
  sudo -n cp -a "${INSTALL_BINARY}" "${backup_binary}"
  binary_existed=1
fi
for name in "${state_files[@]}"; do
  if sudo -n test -e "${STATE_DIR}/${name}"; then
    sudo -n cp -a "${STATE_DIR}/${name}" "${backup_state}/${name}"
  fi
done

rollback_armed=1
sudo -n install -m 0755 "${ADMISSION_BINARY_SOURCE}" "${INSTALL_BINARY}.candidate.$$"
sudo -n mv -f "${INSTALL_BINARY}.candidate.$$" "${INSTALL_BINARY}"
for unit in "${unit_names[@]}"; do
  if [[ "${unit}" == *.service ]]; then
    sed "s#${ADMISSION_BINARY_SOURCE}#${INSTALL_BINARY}#g" \
      "${rendered_units}/${unit}" >"${work}/${unit}"
    sudo -n install -m 0644 "${work}/${unit}" "${SYSTEMD_DIR}/${unit}"
  else
    sudo -n install -m 0644 "${rendered_units}/${unit}" "${SYSTEMD_DIR}/${unit}"
  fi
done
sudo -n systemctl daemon-reload
for unit in "${watch_units[@]}"; do
  sudo -n systemctl enable --now "${unit}" >/dev/null
done

sudo -n systemctl start nando-response-admission.service
sudo -n systemctl start nando-live-transition-gate.service
jq -e '.verdict == "PASS" and .active_packages > 0' \
  "${STATE_DIR}/response-admission-controller-report.json" >/dev/null
jq -e '.verdict == "PASS" and .eligible_for_local_accept == true' \
  "${STATE_DIR}/admission.json" >/dev/null

ready=0
for _attempt in $(seq 1 "${READINESS_ATTEMPTS}"); do
  if curl -fsS --max-time 2 "${HOT_HEALTH}" | jq -e '
    .ok == true
    and .response_effective_local_accept_enabled == true
    and .response_active_profiles > 0
    and .response_cache_error == ""
  ' >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep "${READINESS_SLEEP_SECONDS}"
done
[[ "${ready}" == "1" ]]

hot_pid_after="$(systemctl show -p MainPID --value nando-transition-serving.service)"
gateway_pid_after="$(systemctl show -p MainPID --value nando-transport-gateway.service)"
if [[ "${hot_pid_before}" == "0" || "${gateway_pid_before}" == "0" \
  || "${hot_pid_before}" != "${hot_pid_after}" \
  || "${gateway_pid_before}" != "${gateway_pid_after}" ]]; then
  printf 'hot serving or gateway PID changed during cold-path install\n' >&2
  exit 1
fi

rollback_armed=0
printf 'MS4 autonomous loop active; hot serving and gateway stayed online\n'
