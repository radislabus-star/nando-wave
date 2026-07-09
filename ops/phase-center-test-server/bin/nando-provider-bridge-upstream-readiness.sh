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
OUT_JSON="${NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_REPORT:-${NANDO_METRICS_DIR:-/var/lib/nando-wave/streaming/metrics}/nando-phase-center.provider-bridge-upstream-readiness.json}"
BOUNDARY_JSONL="${NANDO_PROVIDER_BRIDGE_BOUNDARY_EVENTS_JSONL:-/var/lib/nando-wave/streaming/nando-provider-bridge.provider-boundary-events.jsonl}"
ALLOW_REAL_CALL="${NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_ALLOW_REAL_CALL:-0}"
PROMPT="${NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_PROMPT:-ordinary broad prompt}"

mkdir -p "$(dirname "${OUT_JSON}")"

body_file="$(mktemp)"
trap 'rm -f "${body_file}"' EXIT

health_ok=false
upstream_configured=false
health_status=0
if curl -fsS "${BASE_URL}/health" > "${body_file}" 2>/dev/null; then
  health_status=200
  if jq -e '.ok == true' "${body_file}" >/dev/null 2>&1; then
    health_ok=true
  fi
  if jq -e '.upstream_configured == true' "${body_file}" >/dev/null 2>&1; then
    upstream_configured=true
  fi
fi

boundary_rows_before=0
if [[ -s "${BOUNDARY_JSONL}" ]]; then
  boundary_rows_before="$(wc -l < "${BOUNDARY_JSONL}" | tr -d ' ')"
fi

real_probe_attempted=false
real_probe_http_status=0
real_probe_upstream_reached=false
real_probe_local_accept=false
real_probe_boundary_rows_after="${boundary_rows_before}"
real_probe_boundary_rows_added=0
real_probe_error_type=""

if [[ "${health_ok}" == "true" && "${upstream_configured}" == "true" && "${ALLOW_REAL_CALL}" == "1" ]]; then
  real_probe_attempted=true
  probe_body="$(jq -cn --arg prompt "${PROMPT}" '{model:"nando-upstream-readiness",messages:[{role:"user",content:$prompt}]}')"
  response="$(curl -sS -w '\n%{http_code}' \
    -H 'content-type: application/json' \
    --data "${probe_body}" \
    "${BASE_URL}/v1/chat/completions" || true)"
  real_probe_http_status="$(printf '%s' "${response}" | tail -n 1)"
  if [[ ! "${real_probe_http_status}" =~ ^[0-9]+$ ]]; then
    real_probe_http_status=0
  fi
  printf '%s\n' "${response}" | sed '$d' > "${body_file}"
  real_probe_error_type="$(jq -r '.error.type // ""' "${body_file}" 2>/dev/null || true)"
  if jq -e '.nando.local_accept == true' "${body_file}" >/dev/null 2>&1; then
    real_probe_local_accept=true
  fi
  if [[ "${real_probe_http_status}" == "200" && "${real_probe_local_accept}" == "false" && "${real_probe_error_type}" != "upstream_missing" ]]; then
    real_probe_upstream_reached=true
  fi
  if [[ -s "${BOUNDARY_JSONL}" ]]; then
    real_probe_boundary_rows_after="$(wc -l < "${BOUNDARY_JSONL}" | tr -d ' ')"
  fi
  if (( real_probe_boundary_rows_after > boundary_rows_before )); then
    real_probe_boundary_rows_added=$((real_probe_boundary_rows_after - boundary_rows_before))
  fi
fi

verdict="NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_WATCH_BRIDGE_HEALTH_MISSING"
ready_for_broad_provider_traffic=false
if [[ "${health_ok}" == "true" && "${upstream_configured}" == "false" ]]; then
  verdict="NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_WATCH_CANARY_ONLY_UPSTREAM_UNSET"
elif [[ "${health_ok}" == "true" && "${upstream_configured}" == "true" && "${ALLOW_REAL_CALL}" != "1" ]]; then
  verdict="NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_WATCH_UPSTREAM_CONFIGURED_NOT_PROBED"
elif [[ "${health_ok}" == "true" && "${upstream_configured}" == "true" && "${real_probe_upstream_reached}" == "true" && "${real_probe_boundary_rows_added}" -gt 0 ]]; then
  verdict="NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_PASS_UPSTREAM_AND_BOUNDARY_CAPTURE"
  ready_for_broad_provider_traffic=true
elif [[ "${health_ok}" == "true" && "${upstream_configured}" == "true" ]]; then
  verdict="NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_WATCH_UPSTREAM_PROBE_FAILED"
fi

jq -n \
  --arg env_file "${ENV_FILE}" \
  --arg base_url "${BASE_URL}" \
  --arg boundary_jsonl "${BOUNDARY_JSONL}" \
  --arg verdict "${verdict}" \
  --arg real_probe_error_type "${real_probe_error_type}" \
  --argjson health_ok "${health_ok}" \
  --argjson health_status "${health_status}" \
  --argjson upstream_configured "${upstream_configured}" \
  --argjson allow_real_call "$(if [[ "${ALLOW_REAL_CALL}" == "1" ]]; then echo true; else echo false; fi)" \
  --argjson real_probe_attempted "${real_probe_attempted}" \
  --argjson real_probe_http_status "${real_probe_http_status}" \
  --argjson real_probe_upstream_reached "${real_probe_upstream_reached}" \
  --argjson real_probe_local_accept "${real_probe_local_accept}" \
  --argjson boundary_rows_before "${boundary_rows_before}" \
  --argjson boundary_rows_after "${real_probe_boundary_rows_after}" \
  --argjson boundary_rows_added "${real_probe_boundary_rows_added}" \
  --argjson ready_for_broad_provider_traffic "${ready_for_broad_provider_traffic}" \
  '{
    report_kind: "nando_provider_bridge_upstream_readiness_v1",
    env_file: $env_file,
    base_url: $base_url,
    provider_bridge_boundary_jsonl: $boundary_jsonl,
    health_ok: $health_ok,
    health_status: $health_status,
    upstream_configured: $upstream_configured,
    real_probe_allowed: $allow_real_call,
    real_probe_attempted: $real_probe_attempted,
    real_probe_http_status: $real_probe_http_status,
    real_probe_upstream_reached: $real_probe_upstream_reached,
    real_probe_local_accept: $real_probe_local_accept,
    real_probe_error_type: $real_probe_error_type,
    boundary_rows_before: $boundary_rows_before,
    boundary_rows_after: $boundary_rows_after,
    boundary_rows_added: $boundary_rows_added,
    ready_for_broad_provider_traffic: $ready_for_broad_provider_traffic,
    local_accept_enabled: false,
    market_money_claim_allowed: false,
    verdict: $verdict,
    boundary: "upstream readiness report only: default mode does not call the real provider; optional real probe checks fail-open broad transport and provider-boundary capture without creating money claims"
  }' > "${OUT_JSON}"

echo "${OUT_JSON}"
