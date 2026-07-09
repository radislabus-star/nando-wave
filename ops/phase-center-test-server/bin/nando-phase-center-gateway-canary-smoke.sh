#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
if [[ -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "${ENV_FILE}"
  set +a
fi

GATEWAY_BIN="${NANDO_GATEWAY_BIN:-/usr/local/bin/nando-llm-gateway}"
OUT_JSON="${NANDO_GATEWAY_CANARY_SMOKE_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.gateway-canary-smoke.json}"

mkdir -p "$(dirname "${OUT_JSON}")"

case_rows="$(mktemp)"
trap 'rm -f "${case_rows}"' EXIT

run_case() {
  local route="$1"
  local request="$2"
  local expected_prefix="$3"
  local expect_fallback="$4"
  local output status passed

  set +e
  output="$(printf '%s' "${request}" | "${GATEWAY_BIN}" "${ENV_FILE}" -- cat 2>&1)"
  status=$?
  set -e

  passed=false
  if [[ "${status}" -eq 0 ]]; then
    if [[ "${expect_fallback}" == "true" ]]; then
      [[ "${output}" == "${request}" ]] && passed=true
    else
      [[ "${output}" == "${expected_prefix}"* ]] && passed=true
    fi
  fi

  jq -cn \
    --arg route "${route}" \
    --arg request "${request}" \
    --arg expected_prefix "${expected_prefix}" \
    --arg output "${output}" \
    --argjson expect_fallback "${expect_fallback}" \
    --argjson status "${status}" \
    --argjson passed "${passed}" \
    '{
      route: $route,
      request: $request,
      expected_prefix: $expected_prefix,
      output: $output,
      expect_fallback: $expect_fallback,
      status: $status,
      passed: $passed
    }' >> "${case_rows}"
}

run_case "nando_gateway_health" "nando health" "NANDO_GATEWAY_OK" false
run_case "nando_compression_status" "nando compression" "NANDO_COMPRESSION" false
run_case "nando_readiness_status" "nando readiness" "NANDO_READINESS" false
run_case "nando_promotion_status" "nando promotion" "NANDO_PROMOTION" false
run_case "nando_server_status" "nando server" "NANDO_SERVER" false
run_case "broad_prompt_fallback" "ordinary broad prompt" "ordinary broad prompt" true

jq -s \
  --arg env_file "${ENV_FILE}" \
  --arg gateway_bin "${GATEWAY_BIN}" \
  '{
    report_kind: "nando_phase_center_gateway_canary_smoke_v1",
    env_file: $env_file,
    gateway_bin: $gateway_bin,
    cases: .,
    case_count: length,
    passed_count: ([.[] | select(.passed)] | length),
    failed_count: ([.[] | select(.passed | not)] | length),
    local_route_count: ([.[] | select(.expect_fallback | not)] | length),
    fallback_route_count: ([.[] | select(.expect_fallback)] | length),
    verdict: (if all(.[]; .passed) then
      "NANDO_GATEWAY_CANARY_SMOKE_PASS"
    else
      "NANDO_GATEWAY_CANARY_SMOKE_FAIL"
    end),
    boundary: "gateway canary smoke only: exact verifier-bound local routes must accept, broad prompt must fallback"
  }' "${case_rows}" > "${OUT_JSON}"

jq -e '.failed_count == 0' "${OUT_JSON}" >/dev/null
echo "${OUT_JSON}"
