#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INSTALLER="${ROOT}/ops/remote-backend/install-remote-evidence-spool.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

BIN="${WORK}/bin"
SYSTEMCTL_STATE="${WORK}/systemctl"
INSTALL_BINARY="${WORK}/opt/nando-transition-serving"
ROLE_ENV="${WORK}/etc/response-learning.env"
KEY_DIRECTORY="${WORK}/etc/evidence-clients"
LEARNING_STATE="${WORK}/state/multi-source-live-v2"
SPOOL_DIRECTORY="${LEARNING_STATE}/remote-evidence-spool-v1"
CLIENT_KEY="${WORK}/client.key"
CANDIDATE_ONE="${WORK}/candidate-one"
CANDIDATE_TWO="${WORK}/candidate-two"
mkdir -p \
  "${BIN}" \
  "${SYSTEMCTL_STATE}" \
  "$(dirname "${INSTALL_BINARY}")" \
  "$(dirname "${ROLE_ENV}")" \
  "${LEARNING_STATE}"

cat >"${BIN}/sudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "-n" ]]
shift
if [[ "${1:-}" == "install" ]]; then
  shift
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
fi
if [[ "${1:-}" == "chown" ]]; then
  exit 0
fi
exec "$@"
EOF

cat >"${BIN}/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  "show -p User --value nando-response-learning.service")
    id -un
    ;;
  "show -p Group --value nando-response-learning.service")
    id -gn
    ;;
  "is-active --quiet nando-response-learning.service")
    [[ -e "${NANDO_TEST_SYSTEMCTL_STATE}/active" ]]
    ;;
  "stop nando-response-learning.service")
    rm -f "${NANDO_TEST_SYSTEMCTL_STATE}/active"
    ;;
  "start nando-response-learning.service")
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
url="${*: -1}"
if [[ "${url}" == *":18790/health" ]]; then
  if [[ "${NANDO_TEST_FAIL_LEARNING:-0}" == "1" ]]; then
    printf '%s\n' '{"ok":false}'
  else
    printf '%s\n' \
      '{"ok":true,"remote_evidence":{"enabled":true,"transport_ready":true,"configured_clients":1,"learning_closed_loop_ready":false},"learning_health":{"serving_healthy":true,"authority_ready":false,"phase_mutation_allowed":false}}'
  fi
elif [[ "${url}" == *":18789/health" ]]; then
  printf '%s\n' '{"ok":true}'
else
  printf 'unexpected curl URL: %s\n' "${url}" >&2
  exit 2
fi
EOF

chmod +x "${BIN}/sudo" "${BIN}/systemctl" "${BIN}/curl"
printf '%s\n' '# old binary' >"${INSTALL_BINARY}"
printf '%s\n' '# candidate one' >"${CANDIDATE_ONE}"
printf '%s\n' '# candidate two' >"${CANDIDATE_TWO}"
chmod 0755 "${INSTALL_BINARY}" "${CANDIDATE_ONE}" "${CANDIDATE_TWO}"
printf '%s\n' 'NANDO_REMOTE_EVIDENCE_SPOOL_ENABLED=0' >"${ROLE_ENV}"
printf '%s\n' 'state marker' >"${LEARNING_STATE}/marker"
head -c 32 /dev/zero >"${CLIENT_KEY}"
touch "${SYSTEMCTL_STATE}/active"

export PATH="${BIN}:/usr/bin:/bin"
export NANDO_TEST_SYSTEMCTL_STATE="${SYSTEMCTL_STATE}"
export NANDO_REMOTE_SERVING_BINARY="${INSTALL_BINARY}"
export NANDO_REMOTE_LEARNING_ENV="${ROLE_ENV}"
export NANDO_REMOTE_EVIDENCE_KEYS="${KEY_DIRECTORY}"
export NANDO_REMOTE_EVIDENCE_SPOOL="${SPOOL_DIRECTORY}"
export NANDO_REMOTE_LEARNING_STATE="${LEARNING_STATE}"
export NANDO_REMOTE_EVIDENCE_READINESS_ATTEMPTS=1
export NANDO_REMOTE_EVIDENCE_READINESS_SLEEP_SECONDS=0

"${INSTALLER}" \
  --binary "${CANDIDATE_ONE}" \
  --client-key "${CLIENT_KEY}" >/dev/null

cmp -s "${CANDIDATE_ONE}" "${INSTALL_BINARY}"
grep -Fxq 'NANDO_REMOTE_EVIDENCE_SPOOL_ENABLED=1' "${ROLE_ENV}"
grep -Fxq "NANDO_REMOTE_EVIDENCE_SPOOL=${SPOOL_DIRECTORY}" "${ROLE_ENV}"
grep -Fxq "NANDO_REMOTE_EVIDENCE_CLIENT_KEYS=${KEY_DIRECTORY}" "${ROLE_ENV}"
grep -Fxq 'state marker' "${LEARNING_STATE}/marker"
[[ -e "${SYSTEMCTL_STATE}/active" ]]
client_id="$(sha256sum "${CLIENT_KEY}")"
client_id="${client_id%% *}"
[[ "$(stat -c '%a' "${KEY_DIRECTORY}/${client_id}.key")" == "600" ]]

cp -a "${INSTALL_BINARY}" "${WORK}/expected-binary"
cp -a "${ROLE_ENV}" "${WORK}/expected-env"
cp -a "${KEY_DIRECTORY}/${client_id}.key" "${WORK}/expected-key"

if NANDO_TEST_FAIL_LEARNING=1 "${INSTALLER}" \
  --binary "${CANDIDATE_TWO}" \
  --client-key "${CLIENT_KEY}" >/dev/null 2>&1; then
  printf '%s\n' "installer accepted a failed learner health check" >&2
  exit 1
fi

cmp -s "${WORK}/expected-binary" "${INSTALL_BINARY}"
cmp -s "${WORK}/expected-env" "${ROLE_ENV}"
cmp -s "${WORK}/expected-key" "${KEY_DIRECTORY}/${client_id}.key"
grep -Fxq 'state marker' "${LEARNING_STATE}/marker"
[[ -e "${SYSTEMCTL_STATE}/active" ]]
if compgen -G "${LEARNING_STATE}.rollback.*" >/dev/null; then
  printf '%s\n' "installer left a state rollback directory" >&2
  exit 1
fi

printf '%s\n' "install-remote-evidence-spool transaction tests: PASS"
