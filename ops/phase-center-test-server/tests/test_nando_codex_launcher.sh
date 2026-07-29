#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LAUNCHER="${ROOT}/bin/nando-codex"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

mkdir -p "${WORK}/bin" "${WORK}/home"
printf '%s\n' '{"ok":true,"service":"nando-nginx-gateway","transport":"nginx"}' >"${WORK}/health.json"
printf '%s\n' '# test entrypoint consumed by the fake node executable' >"${WORK}/argv.js"
cat >"${WORK}/bin/node" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
shift
printf '%s\n' "$@" | jq -R -s 'split("\n")[:-1]'
EOF
chmod +x "${WORK}/bin/node"

run_launcher() {
  PATH="${WORK}/bin:${PATH}" \
  HOME="${WORK}/home" \
  NANDO_REAL_CODEX="${WORK}/argv.js" \
  NANDO_CODEX_HEALTH_URL="file://${WORK}/health.json" \
  NANDO_GATEWAY_EVENTS_JSONL="${WORK}/events.jsonl" \
    "${LAUNCHER}" "$@"
}

assert_argv() {
  local actual="$1"
  local expected="$2"
  [[ -n "${actual}" ]]
  jq -e --argjson expected "${expected}" '. == $expected' <<<"${actual}" >/dev/null
}

assert_argv \
  "$(run_launcher exec --ephemeral probe)" \
  '["-c","model_provider=\"nando_remote\"","exec","--ephemeral","probe"]'

assert_argv \
  "$(run_launcher resume session-id probe)" \
  '["-c","model_provider=\"nando_remote\"","resume","session-id","probe"]'

assert_argv \
  "$(run_launcher -m gpt-5.6-sol exec probe)" \
  '["-m","gpt-5.6-sol","-c","model_provider=\"nando_remote\"","exec","probe"]'

assert_argv \
  "$(run_launcher probe)" \
  '["-c","model_provider=\"nando_remote\"","probe"]'

assert_argv \
  "$(NANDO_CODEX_FORCE_DIRECT=1 run_launcher exec probe)" \
  '["-c","model_provider=\"openai\"","exec","probe"]'

dry_run="$({ NANDO_CODEX_DRY_RUN=1 run_launcher exec probe; })"
jq -e '
  .route_mode == "nando_nginx"
  and .selected_base_url == "http://127.0.0.1:8787/v1"
  and .config_override_scope == "exec"
  and .config_override_insert_index == 0
  and .config_override_position_verified == true
' <<<"${dry_run}" >/dev/null

remote_dry_run="$({
  NANDO_CODEX_DRY_RUN=1 \
  NANDO_CODEX_GATEWAY_ORIGIN="http://192.168.3.94:8787" \
    run_launcher exec probe
})"
jq -e '
  .route_mode == "nando_nginx"
  and .selected_base_url == "http://192.168.3.94:8787/v1"
' <<<"${remote_dry_run}" >/dev/null

echo "nando-codex launcher tests: PASS"
