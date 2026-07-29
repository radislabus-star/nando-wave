#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INSTALLER="${ROOT}/ops/remote-backend/install-client-connector.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

BIN="${WORK}/bin"
HOME_DIR="${WORK}/home"
SYSTEMCTL_STATE="${WORK}/systemctl"
CANDIDATE="${WORK}/nando-connector"
mkdir -p "${BIN}" "${HOME_DIR}/.local/bin" \
  "${HOME_DIR}/.config/systemd/user" "${SYSTEMCTL_STATE}"

cat >"${CANDIDATE}" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ " $* " == *" --check "* ]]
exit 0
EOF

cat >"${BIN}/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "--user" ]]
shift
case "$*" in
  "is-active --quiet nando-client-connector.service")
    [[ -e "${NANDO_TEST_SYSTEMCTL_STATE}/active" ]]
    ;;
  "is-enabled --quiet nando-client-connector.service")
    [[ -e "${NANDO_TEST_SYSTEMCTL_STATE}/enabled" ]]
    ;;
  "daemon-reload")
    ;;
  "enable nando-client-connector.service")
    touch "${NANDO_TEST_SYSTEMCTL_STATE}/enabled"
    ;;
  "disable nando-client-connector.service")
    rm -f "${NANDO_TEST_SYSTEMCTL_STATE}/enabled"
    ;;
  "stop nando-client-connector.service")
    rm -f "${NANDO_TEST_SYSTEMCTL_STATE}/active"
    ;;
  "start nando-client-connector.service"|"restart nando-client-connector.service")
    if [[ -e "${NANDO_TEST_SYSTEMCTL_STATE}/fail-next-start" ]]; then
      rm -f "${NANDO_TEST_SYSTEMCTL_STATE}/fail-next-start"
      rm -f "${NANDO_TEST_SYSTEMCTL_STATE}/active"
    else
      touch "${NANDO_TEST_SYSTEMCTL_STATE}/active"
    fi
    ;;
  *)
    printf 'unexpected systemctl invocation: %s\n' "$*" >&2
    exit 2
    ;;
esac
EOF

cat >"${BIN}/systemd-analyze" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "--user" && "${2:-}" == "verify" ]]
unit="${3}"
grep -Fq -- '--route-receipts %t/nando-connector/route-receipts-v1.jsonl' "${unit}"
grep -Fq 'RuntimeDirectoryPreserve=restart' "${unit}"
EOF

cat >"${BIN}/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
url="${*: -1}"
case "${url}" in
  */metrics)
    active=0
    [[ -e "${NANDO_TEST_SYSTEMCTL_STATE}/busy" ]] && active=2
    printf '{"ok":true,"service":"nando-connector","active_connections":%s,"route_receipts":0,"route_receipt_failures":0}\n' "${active}"
    ;;
  */health)
    printf '%s\n' '{"ok":true,"service":"nando-nginx-gateway","transport":"nginx"}'
    ;;
  *)
    exit 2
    ;;
esac
EOF

chmod +x "${CANDIDATE}" "${BIN}/systemctl" "${BIN}/systemd-analyze" "${BIN}/curl"
printf '%s\n' '# old binary' >"${HOME_DIR}/.local/bin/nando-connector"
chmod 0755 "${HOME_DIR}/.local/bin/nando-connector"
printf '%s\n' '# old unit' >"${HOME_DIR}/.config/systemd/user/nando-client-connector.service"
touch "${SYSTEMCTL_STATE}/active" "${SYSTEMCTL_STATE}/enabled" "${SYSTEMCTL_STATE}/busy"

export HOME="${HOME_DIR}"
export PATH="${BIN}:/usr/bin:/bin"
export NANDO_TEST_SYSTEMCTL_STATE="${SYSTEMCTL_STATE}"
export NANDO_CONNECTOR_INSTALL_READINESS_ATTEMPTS=1
export NANDO_CONNECTOR_INSTALL_READINESS_SLEEP_SECONDS=0
export NANDO_CONNECTOR_INSTALL_DRAIN_POLL_SECONDS=0.01
export NANDO_CONNECTOR_INSTALL_DRAIN_SAMPLE_SLEEP_SECONDS=0.01

set +e
"${INSTALLER}" --binary "${CANDIDATE}" >/dev/null 2>&1
rc=$?
set -e
[[ "${rc}" == "75" ]]
grep -Fxq '# old binary' "${HOME_DIR}/.local/bin/nando-connector"
grep -Fxq '# old unit' "${HOME_DIR}/.config/systemd/user/nando-client-connector.service"
[[ -e "${SYSTEMCTL_STATE}/active" ]]

(
  sleep 0.05
  rm -f "${SYSTEMCTL_STATE}/busy"
) &
drain_release_pid=$!
"${INSTALLER}" --binary "${CANDIDATE}" --wait-for-drain 2 >/dev/null
wait "${drain_release_pid}"
cmp -s "${CANDIDATE}" "${HOME_DIR}/.local/bin/nando-connector"
grep -Fq -- '--route-receipts %t/nando-connector/route-receipts-v1.jsonl' \
  "${HOME_DIR}/.config/systemd/user/nando-client-connector.service"
[[ -e "${SYSTEMCTL_STATE}/active" ]]
[[ -e "${SYSTEMCTL_STATE}/enabled" ]]

printf '%s\n' '# old binary' >"${HOME_DIR}/.local/bin/nando-connector"
chmod 0755 "${HOME_DIR}/.local/bin/nando-connector"
printf '%s\n' '# old unit' >"${HOME_DIR}/.config/systemd/user/nando-client-connector.service"
touch "${SYSTEMCTL_STATE}/active" "${SYSTEMCTL_STATE}/enabled" \
  "${SYSTEMCTL_STATE}/fail-next-start"

if "${INSTALLER}" --binary "${CANDIDATE}" >/dev/null 2>&1; then
  printf '%s\n' "installer accepted a failed connector start" >&2
  exit 1
fi
grep -Fxq '# old binary' "${HOME_DIR}/.local/bin/nando-connector"
grep -Fxq '# old unit' "${HOME_DIR}/.config/systemd/user/nando-client-connector.service"
[[ -e "${SYSTEMCTL_STATE}/active" ]]
[[ -e "${SYSTEMCTL_STATE}/enabled" ]]

printf '%s\n' "install-client-connector transaction tests: PASS"
