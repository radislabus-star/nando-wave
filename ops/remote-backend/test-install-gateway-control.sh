#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INSTALLER="${ROOT}/ops/remote-backend/install-gateway-control.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

BIN="${WORK}/bin"
SYSTEMCTL_STATE="${WORK}/systemctl"
INSTALL_BINARY="${WORK}/opt/nando-gateway-control"
CANDIDATE_ONE="${WORK}/candidate-one"
CANDIDATE_TWO="${WORK}/candidate-two"
INSTALL_SIDECAR="${WORK}/var/s1c3-operational-status-v1.json"
SIDECAR_ONE="${WORK}/sidecar-one.json"
SIDECAR_TWO="${WORK}/sidecar-two.json"
mkdir -p "${BIN}" "${SYSTEMCTL_STATE}" "$(dirname "${INSTALL_BINARY}")"
mkdir -p "$(dirname "${INSTALL_SIDECAR}")"

cat >"${BIN}/sudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "-n" ]]
shift
exec "$@"
EOF

cat >"${BIN}/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  "is-active --quiet nando-gateway-control.service")
    [[ -e "${NANDO_TEST_SYSTEMCTL_STATE}/active" ]]
    ;;
  "stop nando-gateway-control.service")
    rm -f "${NANDO_TEST_SYSTEMCTL_STATE}/active"
    ;;
  "start nando-gateway-control.service")
    touch "${NANDO_TEST_SYSTEMCTL_STATE}/active"
    ;;
  *)
    printf 'unexpected systemctl invocation: %s\n' "$*" >&2
    exit 2
    ;;
esac
EOF

cat >"${BIN}/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ " $* " == *" --config - "* ]]; then
  IFS= read -r config_line
  url="${config_line#url = \"}"
  url="${url%\"}"
else
  url="${*: -1}"
fi
case "${url}" in
  *":18788/health")
    if [[ "${NANDO_TEST_FAIL_CONTROL:-0}" == "1" ]]; then
      printf '%s\n' '{"ok":false}'
    else
      printf '%s\n' '{"ok":true,"service":"nando-gateway-control"}'
    fi
    ;;
  *"/api/v1/dashboard")
    if [[ "${NANDO_TEST_FAIL_PROJECTION:-0}" == "1" ]]; then
      natural_record_count=1
    else
      natural_record_count=0
    fi
    printf '%s\n' "{\"available\":true,\"dashboard_build\":\"2026.08.13-control-v20\",\"s1c3_operational\":{\"stage\":\"S1C-3H\",\"verdict\":\"S1C3H_DEPLOYMENT_PASS\",\"capture_installed\":true,\"natural_record_count\":${natural_record_count},\"s1c4_state\":\"COLLECTING\",\"authority_ready\":false,\"scientific_authority\":false,\"model_training_allowed\":false,\"phase_mutation_allowed\":false}}"
    ;;
  *":18789/health")
    printf '%s\n' '{"ok":true}'
    ;;
  "http://192.168.3.94:8787/health")
    printf '%s\n' '{"ok":true,"service":"nando-nginx-gateway"}'
    ;;
  *)
    printf 'unexpected curl URL: %s\n' "${url}" >&2
    exit 2
    ;;
esac
EOF

chmod +x "${BIN}/sudo" "${BIN}/systemctl" "${BIN}/curl"
printf '%s\n' '# old binary' >"${INSTALL_BINARY}"
printf '%s\n' '# candidate one' >"${CANDIDATE_ONE}"
printf '%s\n' '# candidate two' >"${CANDIDATE_TWO}"
printf '%s\n' '{"status":"old"}' >"${INSTALL_SIDECAR}"
printf '%s\n' '{"status":"one"}' >"${SIDECAR_ONE}"
printf '%s\n' '{"status":"two"}' >"${SIDECAR_TWO}"
chmod 0755 "${INSTALL_BINARY}" "${CANDIDATE_ONE}" "${CANDIDATE_TWO}"
touch "${SYSTEMCTL_STATE}/active"

export PATH="${BIN}:/usr/bin:/bin"
export NANDO_TEST_SYSTEMCTL_STATE="${SYSTEMCTL_STATE}"
export NANDO_GATEWAY_CONTROL_BINARY="${INSTALL_BINARY}"
export NANDO_S1C3_OPERATIONAL_STATUS_JSON="${INSTALL_SIDECAR}"
export NANDO_GATEWAY_CONTROL_DASHBOARD_KEY="test-dashboard-key"
export NANDO_GATEWAY_CONTROL_READINESS_ATTEMPTS=1
export NANDO_GATEWAY_CONTROL_READINESS_SLEEP_SECONDS=0

"${INSTALLER}" --binary "${CANDIDATE_ONE}" --sidecar "${SIDECAR_ONE}" >/dev/null
cmp -s "${CANDIDATE_ONE}" "${INSTALL_BINARY}"
cmp -s "${SIDECAR_ONE}" "${INSTALL_SIDECAR}"
[[ -e "${SYSTEMCTL_STATE}/active" ]]

cp -a "${INSTALL_BINARY}" "${WORK}/expected-binary"
cp -a "${INSTALL_SIDECAR}" "${WORK}/expected-sidecar"
if NANDO_TEST_FAIL_PROJECTION=1 "${INSTALLER}" \
  --binary "${CANDIDATE_TWO}" --sidecar "${SIDECAR_TWO}" >/dev/null 2>&1; then
  printf '%s\n' "installer accepted an invalid S1C-3H API projection" >&2
  exit 1
fi

cmp -s "${WORK}/expected-binary" "${INSTALL_BINARY}"
cmp -s "${WORK}/expected-sidecar" "${INSTALL_SIDECAR}"
[[ -e "${SYSTEMCTL_STATE}/active" ]]

if NANDO_TEST_FAIL_CONTROL=1 "${INSTALLER}" \
  --binary "${CANDIDATE_TWO}" --sidecar "${SIDECAR_TWO}" >/dev/null 2>&1; then
  printf '%s\n' "installer accepted a failed control health check" >&2
  exit 1
fi

cmp -s "${WORK}/expected-binary" "${INSTALL_BINARY}"
cmp -s "${WORK}/expected-sidecar" "${INSTALL_SIDECAR}"
[[ -e "${SYSTEMCTL_STATE}/active" ]]
printf '%s\n' "install-gateway-control transaction tests: PASS"
