#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
if [[ -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "${ENV_FILE}"
  set +a
fi

BIND="${NANDO_PROVIDER_BRIDGE_BIND:-127.0.0.1:8787}"
BASE_URL="http://${BIND}"
OUT_JSON="${NANDO_PROVIDER_BRIDGE_SMOKE_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-smoke.json}"

mkdir -p "$(dirname "${OUT_JSON}")"
case_rows="$(mktemp)"
trap 'rm -f "${case_rows}"' EXIT

curl_json() {
  local method="$1"
  local path="$2"
  local body="${3:-}"
  local response status
  if [[ "${method}" == "GET" ]]; then
    response="$(curl -sS -w '\n%{http_code}' "${BASE_URL}${path}")"
  else
    response="$(curl -sS -w '\n%{http_code}' \
      -H 'content-type: application/json' \
      -X "${method}" \
      --data "${body}" \
      "${BASE_URL}${path}")"
  fi
  status="$(printf '%s' "${response}" | tail -n 1)"
  printf '%s\n' "${response}" | sed '$d' > /tmp/nando-provider-bridge-smoke-body.$$
  printf '%s' "${status}"
}

record_case() {
  local name="$1"
  local status="$2"
  local body_file="$3"
  local jq_assert="$4"
  local passed=false
  if jq -e "${jq_assert}" "${body_file}" >/dev/null 2>&1; then
    passed=true
  fi
  jq -cn \
    --arg name "${name}" \
    --argjson status "${status}" \
    --argjson passed "${passed}" \
    --slurpfile body "${body_file}" \
    '{name: $name, status: $status, passed: $passed, body: ($body[0] // {})}' >> "${case_rows}"
}

body_file="/tmp/nando-provider-bridge-smoke-body.$$"

status="$(curl_json GET /health)"
record_case "health" "${status}" "${body_file}" '.ok == true'

status="$(curl_json GET /v2/health)"
record_case "v2_health" "${status}" "${body_file}" '.ok == true and .default_client_api_version == "v2" and (.supported_api_versions | index("v2"))'

status="$(curl_json POST /v1/chat/completions '{"model":"nando-test","messages":[{"role":"user","content":"nando compression"}]}')"
record_case "chat_local_compression" "${status}" "${body_file}" '.nando.local_accept == true and (.choices[0].message.content | startswith("NANDO_COMPRESSION"))'

status="$(curl_json POST /v1/responses '{"model":"nando-test","input":"nando readiness"}')"
record_case "responses_local_readiness" "${status}" "${body_file}" '.nando.local_accept == true and (.output_text | startswith("NANDO_READINESS"))'

status="$(curl_json POST /v2/chat/completions '{"model":"nando-test","messages":[{"role":"user","content":"nando compression"}]}')"
record_case "v2_chat_local_compression" "${status}" "${body_file}" '.nando.local_accept == true and .nando.api_version == "v2" and .nando.transition_runtime == true and .nando.architecture == "compact_latent_transition_runtime" and (.choices[0].message.content | startswith("NANDO_COMPRESSION"))'

status="$(curl_json POST /v2/responses '{"model":"nando-test","input":"nando readiness"}')"
record_case "v2_responses_local_readiness" "${status}" "${body_file}" '.nando.local_accept == true and .nando.api_version == "v2" and .nando.transition_runtime == true and .nando.architecture == "compact_latent_transition_runtime" and (.output_text | startswith("NANDO_READINESS"))'

status="$(curl_json POST /v1/chat/completions '{"model":"nando-test","messages":[{"role":"user","content":"ordinary broad prompt"}]}')"
if [[ -n "${NANDO_PROVIDER_UPSTREAM_BASE_URL:-}" ]]; then
  record_case "chat_broad_upstream_fallback" "${status}" "${body_file}" '.nando.local_accept != true'
else
  record_case "chat_broad_upstream_missing" "${status}" "${body_file}" '.error.type == "upstream_missing" and .nando.local_accept == false'
fi

status="$(curl_json POST /v2/chat/completions '{"model":"nando-test","messages":[{"role":"user","content":"ordinary broad prompt"}]}')"
if [[ -n "${NANDO_PROVIDER_UPSTREAM_BASE_URL:-}" ]]; then
  record_case "v2_chat_broad_upstream_fallback" "${status}" "${body_file}" '.nando.local_accept != true'
else
  record_case "v2_chat_broad_upstream_missing" "${status}" "${body_file}" '.error.type == "upstream_missing" and .nando.local_accept == false and .nando.api_version == "v2"'
fi

rm -f "${body_file}"

jq -s \
  --arg env_file "${ENV_FILE}" \
  --arg base_url "${BASE_URL}" \
  '{
    report_kind: "nando_provider_bridge_smoke_v1",
    env_file: $env_file,
    base_url: $base_url,
    cases: .,
    case_count: length,
    passed_count: ([.[] | select(.passed)] | length),
    failed_count: ([.[] | select(.passed | not)] | length),
    verdict: (if all(.[]; .passed) then
      "NANDO_PROVIDER_BRIDGE_SMOKE_PASS"
    else
      "NANDO_PROVIDER_BRIDGE_SMOKE_FAIL"
    end),
    boundary: "HTTP provider bridge smoke: v1 compatibility and v2 compact latent transition routes must accept only verifier-bound local routes; broad route must fail-open to upstream or upstream_missing"
  }' "${case_rows}" > "${OUT_JSON}"

jq -e '.failed_count == 0' "${OUT_JSON}" >/dev/null
echo "${OUT_JSON}"
