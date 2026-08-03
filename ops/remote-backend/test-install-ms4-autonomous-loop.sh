#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INSTALLER="${ROOT}/ops/remote-backend/install-ms4-autonomous-loop.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

BIN="${WORK}/bin"
SYSTEMD_DIR="${WORK}/systemd"
STATE_DIR="${WORK}/state"
PROJECT_ROOT="${WORK}/project"
INSTALL_BINARY="${WORK}/opt/nando-response-admission"
INSTALL_GATE_BINARY="${WORK}/opt/nando-live-transition-gate"
mkdir -p \
  "${BIN}" "${SYSTEMD_DIR}" "${STATE_DIR}" "$(dirname "${INSTALL_BINARY}")" \
  "${PROJECT_ROOT}/ops/phase-center-test-server/gates" \
  "${PROJECT_ROOT}/ops/phase-center-test-server/gates/receipts" \
  "${WORK}/systemctl-state"
printf '{}\n' >"${PROJECT_ROOT}/ops/phase-center-test-server/gates/nando-live-transition-gate.profile.json"
printf '{"verdict":"PASS"}\n' \
  >"${PROJECT_ROOT}/ops/phase-center-test-server/gates/receipts/STRUCTURAL_GATE_V2.json"
printf 'candidate\n' >"${STATE_DIR}/response-admission-candidates.cbor"

contract="$(printf 'a%.0s' {1..64})"
cat >"${WORK}/candidate-admission" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--print-runtime-contract-sha256" ]]; then
  printf '%s\n' "${NANDO_TEST_CONTRACT}"
  exit 0
fi
[[ "${NANDO_RESPONSE_ADMISSION_MARKER}" == */preflight-marker.json ]]
cat >"${NANDO_RESPONSE_REGISTRY}" <<JSON
{"schema":"nando.response-registry.v6","revision":1,"packages":[{}]}
JSON
printf '{}\n' >"${NANDO_RESPONSE_CONTROLLER_ADMISSION_JSON}"
printf '{}\n' >"${NANDO_RESPONSE_AUTHORITY_CANDIDATE}"
printf '{}\n' >"${NANDO_RESPONSE_ADMISSION_MARKER}"
cat >"${NANDO_RESPONSE_ADMISSION_REPORT}" <<JSON
{"verdict":"PASS","active_packages":1}
JSON
EOF
chmod +x "${WORK}/candidate-admission"

cat >"${WORK}/gate" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ -n "${NANDO_LIVE_GATE_PROFILE:-}" ]]; then
  jq -e '.response_runtime.registry | endswith("/preflight-registry.json")' \
    "${NANDO_LIVE_GATE_PROFILE}" >/dev/null
  [[ -f "$(dirname "${NANDO_LIVE_GATE_PROFILE}")/receipts/STRUCTURAL_GATE_V2.json" ]]
fi
output="${NANDO_TRANSITION_ADMISSION_JSON:-${NANDO_TEST_STATE_DIR}/admission.json}"
cat >"${output}" <<JSON
{"verdict":"PASS","eligible_for_local_accept":true,"response_authority":{"runtime_build_sha256":"${NANDO_TEST_CONTRACT}"}}
JSON
cat "${output}"
EOF
chmod +x "${WORK}/gate"

cat >"${BIN}/sudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "-n" ]]
shift
while [[ "${1:-}" == "-u" ]]; do shift 2; done
if [[ "${1:-}" == "install" ]]; then
  shift
  arguments=()
  while [[ $# -gt 0 ]]; do
    case "$1" in -o|-g) shift 2 ;; *) arguments+=("$1"); shift ;; esac
  done
  exec /usr/bin/install "${arguments[@]}"
fi
exec "$@"
EOF

cat >"${BIN}/systemd-analyze" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "verify" ]]
shift
if grep -H 'TriggerLimitIntervalSec' "$@"; then
  printf 'unsupported path trigger limit reached systemd verification\n' >&2
  exit 1
fi
for unit in "$@"; do
  while IFS= read -r exec_start; do
    executable="${exec_start#ExecStart=}"
    [[ "${executable}" == /* ]] || {
      printf 'relative ExecStart reached systemd verification: %s\n' "${executable}" >&2
      exit 1
    }
  done < <(grep '^ExecStart=' "${unit}" || true)
done
EOF

cat >"${BIN}/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
state="${NANDO_TEST_SYSTEMCTL_STATE}"
case "$*" in
  "show -p User --value nando-response-learning.service") id -un ;;
  "show -p Group --value nando-response-learning.service") id -gn ;;
  "show -p MainPID --value nando-transition-serving.service") printf '111\n' ;;
  "show -p MainPID --value nando-transport-gateway.service") printf '222\n' ;;
  "is-enabled --quiet "*) [[ -e "${state}/enabled-${*: -1}" ]] ;;
  "is-active --quiet "*) [[ -e "${state}/active-${*: -1}" ]] ;;
  "enable --now "*) unit="${*: -1}"; touch "${state}/enabled-${unit}" "${state}/active-${unit}" ;;
  "disable --now "*) unit="${*: -1}"; rm -f "${state}/enabled-${unit}" "${state}/active-${unit}" ;;
  "enable "*) touch "${state}/enabled-${*: -1}" ;;
  "start nando-response-admission.service")
    cat >"${NANDO_TEST_STATE_DIR}/response-admission-controller-report.json" <<JSON
{"verdict":"PASS","active_packages":1}
JSON
    ;;
  "start nando-live-transition-gate.service")
    if [[ "${NANDO_TEST_FAIL_LIVE_GATE:-0}" == "1" ]]; then exit 1; fi
    NANDO_TRANSITION_ADMISSION_JSON="${NANDO_TEST_STATE_DIR}/admission.json" \
      "${NANDO_TEST_GATE}" --status-mode >/dev/null
    ;;
  "start "*) touch "${state}/active-${*: -1}" ;;
  "daemon-reload") ;;
  *) printf 'unexpected systemctl invocation: %s\n' "$*" >&2; exit 2 ;;
esac
EOF

cat >"${BIN}/curl" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' '{"ok":true,"response_effective_local_accept_enabled":true,"response_active_profiles":2,"response_cache_error":""}'
EOF
chmod +x "${BIN}"/*

printf 'old admission binary\n' >"${INSTALL_BINARY}"
chmod +x "${INSTALL_BINARY}"
for name in response-registry.json response-admission-controller.json response-admission-controller-report.json response-admission-controller.marker.json response-authority-candidate.json; do
  printf 'old-%s\n' "${name}" >"${STATE_DIR}/${name}"
done
cat >"${STATE_DIR}/admission.json" <<JSON
{"response_authority":{"runtime_build_sha256":"${contract}"}}
JSON

export PATH="${BIN}:/usr/bin:/bin"
export NANDO_TEST_CONTRACT="${contract}"
export NANDO_TEST_STATE_DIR="${STATE_DIR}"
export NANDO_TEST_SYSTEMCTL_STATE="${WORK}/systemctl-state"
export NANDO_TEST_GATE="${WORK}/gate"
export NANDO_MS4_ADMISSION_BINARY="${INSTALL_BINARY}"
export NANDO_MS4_GATE_BINARY="${WORK}/gate"
export NANDO_MS4_GATE_INSTALL_BINARY="${INSTALL_GATE_BINARY}"
export NANDO_MS4_PROJECT_ROOT="${PROJECT_ROOT}"
export NANDO_MS4_SYSTEMD_DIR="${SYSTEMD_DIR}"
export NANDO_MS4_STATE_DIR="${STATE_DIR}"
export NANDO_MS4_READINESS_ATTEMPTS=1
export NANDO_MS4_READINESS_SLEEP_SECONDS=0

(
  cd "${WORK}"
  "${INSTALLER}" --admission-binary candidate-admission \
    --gate-binary "${WORK}/gate" >/dev/null
)
cmp -s "${WORK}/candidate-admission" "${INSTALL_BINARY}"
cmp -s "${WORK}/gate" "${INSTALL_GATE_BINARY}"
for unit in nando-response-admission.path nando-response-admission.timer nando-live-transition-gate.path nando-live-transition-gate.timer; do
  [[ -e "${NANDO_TEST_SYSTEMCTL_STATE}/enabled-${unit}" ]]
  [[ -e "${NANDO_TEST_SYSTEMCTL_STATE}/active-${unit}" ]]
done
grep -Fq "User=$(id -un)" "${SYSTEMD_DIR}/nando-response-admission.service"
grep -Fq "ExecStart=${INSTALL_BINARY}" "${SYSTEMD_DIR}/nando-response-admission.service"
grep -Fq 'OnUnitInactiveSec=10s' "${SYSTEMD_DIR}/nando-live-transition-gate.timer"

printf 'rollback admission binary\n' >"${INSTALL_BINARY}"
chmod +x "${INSTALL_BINARY}"
printf 'rollback gate binary\n' >"${INSTALL_GATE_BINARY}"
chmod +x "${INSTALL_GATE_BINARY}"
printf 'rollback-registry\n' >"${STATE_DIR}/response-registry.json"
cp "${INSTALL_BINARY}" "${WORK}/expected-binary"
cp "${INSTALL_GATE_BINARY}" "${WORK}/expected-gate-binary"
cp "${STATE_DIR}/response-registry.json" "${WORK}/expected-registry"

if NANDO_TEST_FAIL_LIVE_GATE=1 "${INSTALLER}" \
  --admission-binary "${WORK}/candidate-admission" \
  --gate-binary "${WORK}/gate" >/dev/null 2>&1; then
  printf 'installer accepted a failed live gate reconciliation\n' >&2
  exit 1
fi
cmp -s "${WORK}/expected-binary" "${INSTALL_BINARY}"
cmp -s "${WORK}/expected-gate-binary" "${INSTALL_GATE_BINARY}"
cmp -s "${WORK}/expected-registry" "${STATE_DIR}/response-registry.json"

printf '%s\n' 'install-ms4-autonomous-loop transaction tests: PASS'
