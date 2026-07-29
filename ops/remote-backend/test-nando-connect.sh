#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONNECTOR="${ROOT}/nando-connect"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

mkdir -p "${WORK}/bin"

cat >"${WORK}/bin/curl" <<'EOF'
#!/usr/bin/env bash
case "${*: -1}" in
  */metrics)
    printf '%s\n' '{"ok":true,"service":"nando-connector","uptime_seconds":30,"active_connections":2,"accepted_connections":7,"completed_connections":5,"rejected_connections":0,"accept_failures":0,"relay_failures":0,"upload_bytes":123,"download_bytes":456}'
    ;;
  */cpu-health)
    printf '%s\n' '{"ok":true,"mode":"CPU","admission_verdict":"PASS","response_active_profiles":2,"ordinary_response_local_accepts":78,"ordinary_response_local_accept_input_tokens":16227152,"requests":1011,"fallbacks":983,"transition_false_accepts":0}'
    ;;
  *)
    printf '%s\n' '{"ok":true,"service":"nando-nginx-gateway","transport":"nginx","scope":"private_lan"}'
    ;;
esac
EOF

cat >"${WORK}/bin/systemctl-test" <<'EOF'
#!/usr/bin/env bash
case "$*" in
  *is-active*) printf '%s\n' active ;;
esac
EOF

chmod +x "${WORK}/bin/curl" "${WORK}/bin/systemctl-test"

export PATH="${WORK}/bin:${PATH}"
export NANDO_CONNECT_SYSTEMCTL_BIN="${WORK}/bin/systemctl-test"
export NANDO_SERVER_ORIGIN="http://192.168.3.94:8787"
export NANDO_CONNECTOR_ORIGIN="http://127.0.0.1:8787"

[[ "$("${CONNECTOR}" url)" == "http://127.0.0.1:8787/v1" ]]
[[ "$("${CONNECTOR}" server-url)" == "http://192.168.3.94:8787/v1" ]]

health="$("${CONNECTOR}" health)"
jq -e '
  .ok == true
  and .service == "nando-nginx-gateway"
  and .transport == "nginx"
' <<<"${health}" >/dev/null

status="$("${CONNECTOR}" status)"
grep -Fq 'LOCAL CONNECTOR  active' <<<"${status}"
grep -Fq 'CPU / ADMISSION  CPU / PASS' <<<"${status}"
grep -Fq 'ACTIVE PROFILES  2' <<<"${status}"
grep -Fq 'ORDINARY ACCEPTS 78' <<<"${status}"
grep -Fq 'ACTIVE CONNECTIONS 2' <<<"${status}"
grep -Fq 'CONNECTIONS TOTAL  7' <<<"${status}"
grep -Fq 'UPLOAD BYTES       123' <<<"${status}"

watch="$(NANDO_CONNECT_WATCH_ONCE=1 "${CONNECTOR}")"
grep -Fq 'Ctrl+C closes this monitor' <<<"${watch}"

restart="$(NANDO_CONNECT_WATCH_ONCE=1 "${CONNECTOR}" restart)"
grep -Fq 'LOCAL CONNECTOR  active' <<<"${restart}"
grep -Fq 'Ctrl+C closes this monitor' <<<"${restart}"

if "${CONNECTOR}" codex >/dev/null 2>&1; then
  printf '%s\n' "nando-connect unexpectedly launched Codex" >&2
  exit 1
fi

env_output="$("${CONNECTOR}" env)"
grep -Eq '^export OPENAI_BASE_URL=http://127\.0\.0\.1:8787/v1$' <<<"${env_output}"
grep -Eq '^export OPENAI_API_BASE=http://127\.0\.0\.1:8787/v1$' <<<"${env_output}"

printf '%s\n' "nando-connect tests: PASS"
