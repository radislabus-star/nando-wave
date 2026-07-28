#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONNECTOR="${ROOT}/nando-connect"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

mkdir -p "${WORK}/bin"

cat >"${WORK}/bin/curl" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' '{"ok":true,"service":"nando-nginx-gateway","transport":"nginx","scope":"private_lan"}'
EOF

cat >"${WORK}/bin/codex-test" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@" | jq -R -s 'split("\n")[:-1]'
EOF

chmod +x "${WORK}/bin/curl" "${WORK}/bin/codex-test"

export PATH="${WORK}/bin:${PATH}"
export NANDO_CONNECT_CODEX_BIN="${WORK}/bin/codex-test"
export NANDO_SERVER_ORIGIN="http://192.168.3.94:8787"

[[ "$("${CONNECTOR}" url)" == "http://192.168.3.94:8787/v1" ]]

health="$("${CONNECTOR}" health)"
jq -e '
  .ok == true
  and .service == "nando-nginx-gateway"
  and .transport == "nginx"
' <<<"${health}" >/dev/null

argv="$("${CONNECTOR}" codex exec --ephemeral probe)"
jq -e '
  index("model_provider=\"nando_remote\"") != null
  and index("model_providers.nando_remote.base_url=\"http://192.168.3.94:8787/v1\"") != null
  and .[-3:] == ["exec", "--ephemeral", "probe"]
' <<<"${argv}" >/dev/null

env_output="$("${CONNECTOR}" env)"
grep -Eq '^export OPENAI_BASE_URL=http://192\.168\.3\.94:8787/v1$' <<<"${env_output}"
grep -Eq '^export OPENAI_API_BASE=http://192\.168\.3\.94:8787/v1$' <<<"${env_output}"

printf '%s\n' "nando-connect tests: PASS"
