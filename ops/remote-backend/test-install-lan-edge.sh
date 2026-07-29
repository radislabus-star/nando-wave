#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INSTALLER="${ROOT}/ops/remote-backend/install-lan-edge.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

BIN="${WORK}/bin"
NGINX_DIR="${WORK}/nginx"
SYSTEMD_DIR="${WORK}/systemd"
STATE="${WORK}/state"
mkdir -p "${BIN}" "${NGINX_DIR}" "${SYSTEMD_DIR}" "${STATE}"

cat >"${BIN}/sudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
while [[ $# -gt 0 ]]; do
  case "$1" in
    -n)
      shift
      ;;
    -u)
      shift 2
      ;;
    *)
      break
      ;;
  esac
done
exec "$@"
EOF

cat >"${BIN}/install" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
for argument in "$@"; do
  case "${argument}" in
    /run/nando-gateway|/var/lib/nando-gateway*)
      exit 0
      ;;
  esac
done
arguments=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    -o|-g)
      shift 2
      ;;
    *)
      arguments+=("$1")
      shift
      ;;
  esac
done
exec /usr/bin/install "${arguments[@]}"
EOF

cat >"${BIN}/touch" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF

cat >"${BIN}/chown" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF

cat >"${BIN}/chmod" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF

cat >"${BIN}/nginx" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
config=""
while [[ $# -gt 0 ]]; do
  if [[ "$1" == "-c" ]]; then
    config="$2"
    break
  fi
  shift
done
grep -Fq "resolver ${NANDO_TEST_EXPECTED_RESOLVERS} " "${config}"
EOF

cat >"${BIN}/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  "is-active --quiet nando-transport-gateway.service")
    [[ -e "${NANDO_TEST_STATE}/active" ]]
    ;;
  "is-enabled --quiet nando-transport-gateway.service")
    [[ -e "${NANDO_TEST_STATE}/enabled" ]]
    ;;
  "daemon-reload")
    ;;
  "reload nando-transport-gateway.service")
    printf '%s\n' reload >>"${NANDO_TEST_STATE}/reloads"
    ;;
  "enable --now nando-transport-gateway.service")
    touch "${NANDO_TEST_STATE}/active" "${NANDO_TEST_STATE}/enabled"
    ;;
  "stop nando-transport-gateway.service")
    rm -f "${NANDO_TEST_STATE}/active"
    ;;
  "disable nando-transport-gateway.service")
    rm -f "${NANDO_TEST_STATE}/enabled"
    ;;
  *)
    printf 'unexpected systemctl invocation: %s\n' "$*" >&2
    exit 2
    ;;
esac
EOF

cat >"${BIN}/dig" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' 192.0.2.10 192.0.2.11
EOF

cat >"${BIN}/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
url="${*: -1}"
if [[ "${NANDO_TEST_FAIL_CONTROL:-0}" == "1" && "${url}" == */control-health ]]; then
  exit 22
fi
exit 0
EOF

chmod +x "${BIN}"/*

export PATH="${BIN}:/usr/bin:/bin"
export NANDO_REMOTE_NGINX_DIR="${NGINX_DIR}"
export NANDO_REMOTE_SYSTEMD_DIR="${SYSTEMD_DIR}"
export NANDO_TEST_STATE="${STATE}"
export NANDO_TEST_EXPECTED_RESOLVERS="127.0.0.53"

printf '%s\n' "old config" >"${NGINX_DIR}/nginx.conf"
printf '%s\n' "old unit" >"${SYSTEMD_DIR}/nando-transport-gateway.service"
/usr/bin/touch "${STATE}/active" "${STATE}/enabled"

"${INSTALLER}" \
  --bind 192.168.3.94:8787 \
  --allow 192.168.3.0/24 \
  --resolver 127.0.0.53 >/dev/null

grep -Fq "resolver 127.0.0.53 " "${NGINX_DIR}/nginx.conf"
[[ "$(wc -l <"${STATE}/reloads")" == "1" ]]
if compgen -G "${NGINX_DIR}/.nginx.conf.rollback.*" >/dev/null; then
  printf '%s\n' "installer left a rollback file after success" >&2
  exit 1
fi

printf '%s\n' "rollback config" >"${NGINX_DIR}/nginx.conf"
printf '%s\n' "rollback unit" >"${SYSTEMD_DIR}/nando-transport-gateway.service"
: >"${STATE}/reloads"

if NANDO_TEST_FAIL_CONTROL=1 "${INSTALLER}" \
  --bind 192.168.3.94:8787 \
  --allow 192.168.3.0/24 \
  --resolver 127.0.0.53 >/dev/null 2>&1; then
  printf '%s\n' "installer unexpectedly accepted a failed health check" >&2
  exit 1
fi

[[ "$(<"${NGINX_DIR}/nginx.conf")" == "rollback config" ]]
[[ "$(<"${SYSTEMD_DIR}/nando-transport-gateway.service")" == "rollback unit" ]]
[[ "$(wc -l <"${STATE}/reloads")" == "2" ]]
[[ -e "${STATE}/active" ]]
[[ -e "${STATE}/enabled" ]]

printf '%s\n' "install-lan-edge transaction tests: PASS"
