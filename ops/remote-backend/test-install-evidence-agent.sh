#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INSTALLER="${ROOT}/ops/remote-backend/install-evidence-agent.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

BIN="${WORK}/bin"
HOME_DIR="${WORK}/home"
SYSTEMCTL_STATE="${WORK}/systemctl"
CANDIDATE="${WORK}/nando-evidence-agent"
mkdir -p \
  "${BIN}" \
  "${HOME_DIR}/.codex/sessions" \
  "${HOME_DIR}/.config/nando" \
  "${SYSTEMCTL_STATE}"

cat >"${CANDIDATE}" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 0
EOF

cat >"${BIN}/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "--user" ]]
shift
case "$*" in
  "is-active --quiet nando-evidence-agent.service")
    [[ -e "${NANDO_TEST_SYSTEMCTL_STATE}/active" ]]
    ;;
  "is-enabled --quiet nando-evidence-agent.service")
    [[ -e "${NANDO_TEST_SYSTEMCTL_STATE}/enabled" ]]
    ;;
  "daemon-reload")
    ;;
  "enable nando-evidence-agent.service")
    touch "${NANDO_TEST_SYSTEMCTL_STATE}/enabled"
    ;;
  "disable nando-evidence-agent.service")
    rm -f "${NANDO_TEST_SYSTEMCTL_STATE}/enabled"
    ;;
  "stop nando-evidence-agent.service")
    rm -f "${NANDO_TEST_SYSTEMCTL_STATE}/active"
    ;;
  "start nando-evidence-agent.service"|"restart nando-evidence-agent.service")
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
grep -Fq "nando-evidence-agent" "${unit}"
grep -Fq "ReadOnlyPaths=%h/.codex/sessions" "${unit}"
grep -Fq "ReadOnlyPaths=-%t/nando-connector" "${unit}"
grep -Fq "ReadWritePaths=%h/.local/state/nando-evidence-agent" "${unit}"
grep -Fq "MemorySwapMax=0" "${unit}"
grep -Fq "IOSchedulingClass=idle" "${unit}"
exec_path="$(sed -n 's/^ExecStart=\([^ ]*\).*/\1/p' "${unit}")"
[[ -x "${exec_path}" ]]
EOF

chmod +x "${CANDIDATE}" "${BIN}/systemctl" "${BIN}/systemd-analyze"
head -c 32 /dev/zero >"${HOME_DIR}/.config/nando/evidence-agent.key"
chmod 0600 "${HOME_DIR}/.config/nando/evidence-agent.key"

export HOME="${HOME_DIR}"
export PATH="${BIN}:/usr/bin:/bin"
export NANDO_TEST_SYSTEMCTL_STATE="${SYSTEMCTL_STATE}"
export NANDO_EVIDENCE_READINESS_ATTEMPTS=1
export NANDO_EVIDENCE_READINESS_SLEEP_SECONDS=0

"${INSTALLER}" \
  --binary "${CANDIDATE}" \
  --server http://192.168.3.94:8787 >/dev/null

cmp -s "${CANDIDATE}" "${HOME_DIR}/.local/bin/nando-evidence-agent"
grep -Fq "http://192.168.3.94:8787" \
  "${HOME_DIR}/.config/systemd/user/nando-evidence-agent.service"
[[ -e "${SYSTEMCTL_STATE}/active" ]]
[[ -e "${SYSTEMCTL_STATE}/enabled" ]]

printf '%s\n' '# previous binary' \
  >"${HOME_DIR}/.local/bin/nando-evidence-agent"
chmod 0755 "${HOME_DIR}/.local/bin/nando-evidence-agent"
printf '%s\n' '# previous unit' \
  >"${HOME_DIR}/.config/systemd/user/nando-evidence-agent.service"
touch \
  "${SYSTEMCTL_STATE}/active" \
  "${SYSTEMCTL_STATE}/enabled" \
  "${SYSTEMCTL_STATE}/fail-next-start"

if "${INSTALLER}" \
  --binary "${CANDIDATE}" \
  --server http://192.168.3.94:8787 >/dev/null 2>&1; then
  printf '%s\n' "installer accepted a failed service start" >&2
  exit 1
fi

grep -Fxq '# previous binary' \
  "${HOME_DIR}/.local/bin/nando-evidence-agent"
grep -Fxq '# previous unit' \
  "${HOME_DIR}/.config/systemd/user/nando-evidence-agent.service"
[[ -e "${SYSTEMCTL_STATE}/active" ]]
[[ -e "${SYSTEMCTL_STATE}/enabled" ]]

printf '%s\n' "install-evidence-agent transaction tests: PASS"
