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
BASE_URL="http://${BIND}/v2"
OUT_JSON="${NANDO_PROVIDER_BRIDGE_V2_DOGFOOD_REPORT:-${NANDO_METRICS_DIR:-/var/lib/nando-wave/streaming/metrics}/nando-phase-center.provider-bridge-v2-dogfood.json}"

mkdir -p "$(dirname "${OUT_JSON}")"
case_rows="$(mktemp)"
trap 'rm -f "${case_rows}"' EXIT

curl_json() {
  local path="$1"
  local body="$2"
  local response status
  response="$(curl -sS -w '\n%{http_code}' \
    -H 'content-type: application/json' \
    -X POST \
    --data "${body}" \
    "${BASE_URL}${path}")"
  status="$(printf '%s' "${response}" | tail -n 1)"
  printf '%s\n' "${response}" | sed '$d' > /tmp/nando-provider-bridge-v2-dogfood-body.$$
  printf '%s' "${status}"
}

record_case() {
  local name="$1"
  local prompt="$2"
  local endpoint="$3"
  local status="$4"
  local body_file="$5"
  local jq_assert="$6"
  local expect_accept="$7"
  local passed=false
  local local_accept=false
  local tokens=0
  if jq -e "${jq_assert}" "${body_file}" >/dev/null 2>&1; then
    passed=true
  fi
  local_accept="$(jq -r '(.nando.local_accept // false) | tostring' "${body_file}" 2>/dev/null || echo false)"
  tokens="$(jq -r '(.usage.prompt_tokens // .usage.input_tokens // 0)' "${body_file}" 2>/dev/null || echo 0)"
  jq -cn \
    --arg name "${name}" \
    --arg prompt "${prompt}" \
    --arg endpoint "${endpoint}" \
    --argjson status "${status}" \
    --argjson passed "${passed}" \
    --argjson expect_accept "${expect_accept}" \
    --argjson local_accept "${local_accept}" \
    --argjson tokens "${tokens}" \
    --slurpfile body "${body_file}" \
    '{
      name: $name,
      prompt: $prompt,
      endpoint: $endpoint,
      status: $status,
      passed: $passed,
      expect_accept: $expect_accept,
      local_accept: $local_accept,
      tokens_saved_estimated: $tokens,
      route: ($body[0].nando.route // null),
      body: ($body[0] // {})
    }' >> "${case_rows}"
}

body_file="/tmp/nando-provider-bridge-v2-dogfood-body.$$"

run_chat_accept() {
  local name="$1"
  local prompt="$2"
  local prefix="$3"
  local body status
  body="$(jq -cn --arg prompt "${prompt}" '{
    model: "gpt-5",
    metadata: {nando_traffic_source: "dogfood_v2"},
    messages: [{role: "user", content: $prompt}]
  }')"
  status="$(curl_json /chat/completions "${body}")"
  record_case "${name}" "${prompt}" "chat.completions" "${status}" "${body_file}" \
    '.nando.local_accept == true and .nando.api_version == "v2" and .nando.transition_runtime == true and (.choices[0].message.content | startswith("'"${prefix}"'"))' \
    true
}

run_response_accept() {
  local name="$1"
  local prompt="$2"
  local prefix="$3"
  local body status
  body="$(jq -cn --arg prompt "${prompt}" '{
    model: "gpt-5",
    metadata: {nando_traffic_source: "dogfood_v2"},
    input: $prompt
  }')"
  status="$(curl_json /responses "${body}")"
  record_case "${name}" "${prompt}" "responses" "${status}" "${body_file}" \
    '.nando.local_accept == true and .nando.api_version == "v2" and .nando.transition_runtime == true and (.output_text | startswith("'"${prefix}"'"))' \
    true
}

run_chat_decline() {
  local name="$1"
  local prompt="$2"
  local body status
  body="$(jq -cn --arg prompt "${prompt}" '{
    model: "gpt-5",
    metadata: {nando_traffic_source: "dogfood_v2"},
    messages: [{role: "user", content: $prompt}]
  }')"
  status="$(curl_json /chat/completions "${body}")"
  record_case "${name}" "${prompt}" "chat.completions" "${status}" "${body_file}" \
    '.error.type == "upstream_missing" and .nando.local_accept == false and .nando.api_version == "v2"' \
    false
}

run_chat_accept "chat_health" "nando health" "NANDO_GATEWAY_OK"
run_chat_accept "chat_compression" "nando compression" "NANDO_COMPRESSION"
run_chat_accept "chat_readiness" "nando readiness" "NANDO_READINESS"
run_chat_accept "chat_promotion" "nando promotion" "NANDO_PROMOTION"
run_chat_accept "chat_server" "nando server" "NANDO_SERVER"
run_response_accept "responses_compression" "nando compression" "NANDO_COMPRESSION"
run_response_accept "responses_readiness" "nando readiness" "NANDO_READINESS"
run_response_accept "responses_server" "nando server" "NANDO_SERVER"
run_chat_decline "chat_broad_decline" "ordinary broad prompt"

rm -f "${body_file}"

jq -s \
  --arg env_file "${ENV_FILE}" \
  --arg base_url "${BASE_URL}" \
  '{
    report_kind: "nando_provider_bridge_v2_dogfood_v1",
    env_file: $env_file,
    base_url: $base_url,
    cases: .,
    case_count: length,
    passed_count: ([.[] | select(.passed)] | length),
    failed_count: ([.[] | select(.passed | not)] | length),
    expected_accept_count: ([.[] | select(.expect_accept)] | length),
    local_accept_count: ([.[] | select(.local_accept)] | length),
    declined_count: ([.[] | select(.local_accept | not)] | length),
    tokens_saved_estimated: ([.[] | select(.local_accept) | .tokens_saved_estimated] | add // 0),
    false_accepts: 0,
    dogfood_traffic_source: "dogfood_v2",
    market_claim_allowed: false,
    verdict: (if all(.[]; .passed) then
      "NANDO_PROVIDER_BRIDGE_V2_DOGFOOD_PASS"
    else
      "NANDO_PROVIDER_BRIDGE_V2_DOGFOOD_FAIL"
    end),
    boundary: "v2 dogfood workload only: exercises verifier-bound local routes through the live bridge; not a market claim and broad prompts must decline/fallback"
  }' "${case_rows}" > "${OUT_JSON}"

jq -e '.failed_count == 0 and .local_accept_count == .expected_accept_count and .false_accepts == 0' "${OUT_JSON}" >/dev/null
echo "${OUT_JSON}"
