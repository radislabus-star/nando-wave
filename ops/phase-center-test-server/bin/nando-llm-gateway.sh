#!/usr/bin/env bash
set -euo pipefail

DEFAULT_ENV_FILE="/etc/nando-wave/phase-center.env"
if [[ ! -f "${DEFAULT_ENV_FILE}" ]]; then
  DEFAULT_ENV_FILE="${HOME}/.config/nando-wave/phase-center.env"
fi
ENV_FILE="${NANDO_PHASE_CENTER_ENV:-${DEFAULT_ENV_FILE}}"
if [[ "${1:-}" != "--" && -n "${1:-}" && -f "${1:-}" ]]; then
  ENV_FILE="$1"
  shift
fi
if [[ "${1:-}" == "--" ]]; then
  shift
fi

if [[ -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "${ENV_FILE}"
  set +a
fi

REQUEST_DIR="${NANDO_GATEWAY_TMP_DIR:-${HOME}/.local/state/nando-wave/tmp}"
EVENTS_JSONL="${NANDO_GATEWAY_EVENTS_JSONL:-${HOME}/.local/state/nando-wave/streaming/nando-llm-gateway.events.jsonl}"
DECISIONS_JSONL="${NANDO_GATEWAY_DECISIONS_JSONL:-${HOME}/.local/state/nando-wave/streaming/nando-llm-gateway.decisions.jsonl}"
TIMEOUT_MS="${NANDO_GATEWAY_TIMEOUT_MS:-200}"
OFFLOAD_ENABLED="${NANDO_OFFLOAD:-1}"
LOCAL_ACCEPT_ENABLED="${NANDO_LOCAL_ACCEPT_ENABLED:-0}"
CAPTURE_RAW="${NANDO_GATEWAY_CAPTURE_RAW:-0}"
CLIENT_ID="${NANDO_CLIENT_ID:-default}"
CLIENT_TIER="${NANDO_CLIENT_TIER:-shadow}"
CLIENT_SAFETY_POLICY="${NANDO_CLIENT_SAFETY_POLICY:-shadow_only}"
CLIENT_ALLOW_LOCAL_ACCEPT="${NANDO_CLIENT_ALLOW_LOCAL_ACCEPT:-0}"
CLIENT_MAX_TIMEOUT_MS="${NANDO_CLIENT_MAX_TIMEOUT_MS:-${TIMEOUT_MS}}"
CLIENT_REQUIRE_VERIFIER="${NANDO_CLIENT_REQUIRE_VERIFIER:-1}"
CLIENT_REQUIRE_FALSE_ACCEPTS_ZERO="${NANDO_CLIENT_REQUIRE_FALSE_ACCEPTS_ZERO:-1}"
CLIENT_KILL_SWITCH="${NANDO_CLIENT_KILL_SWITCH:-0}"

if [[ "${TIMEOUT_MS}" =~ ^[0-9]+$ && "${CLIENT_MAX_TIMEOUT_MS}" =~ ^[0-9]+$ ]]; then
  if (( TIMEOUT_MS > CLIENT_MAX_TIMEOUT_MS )); then
    TIMEOUT_MS="${CLIENT_MAX_TIMEOUT_MS}"
  fi
fi

mkdir -p "${REQUEST_DIR}" "$(dirname "${EVENTS_JSONL}")" "$(dirname "${DECISIONS_JSONL}")"
request_file="$(mktemp "${REQUEST_DIR}/request.XXXXXX")"
local_file="$(mktemp "${REQUEST_DIR}/local.XXXXXX")"
trap 'rm -f "${request_file}" "${local_file}"' EXIT
cat > "${request_file}"

request_bytes="$(wc -c < "${request_file}" | tr -d ' ')"
request_sha256="$(sha256sum "${request_file}" | awk '{print $1}')"
token_estimate=$(( (request_bytes + 3) / 4 ))
started_ns="$(date +%s%N)"
timeout_arg="$(printf '%d.%03ds' "$((TIMEOUT_MS / 1000))" "$((TIMEOUT_MS % 1000))")"

write_gateway_event() {
  local stage="$1"
  local decision="$2"
  local reason="$3"
  local elapsed_ns="${4:-0}"
  jq -cn \
    --arg ts "$(date -Is)" \
    --arg stage "${stage}" \
    --arg decision "${decision}" \
    --arg reason "${reason}" \
    --arg sha256 "${request_sha256}" \
    --arg env_file "${ENV_FILE}" \
    --arg client_id "${CLIENT_ID}" \
    --arg client_tier "${CLIENT_TIER}" \
    --arg safety_policy "${CLIENT_SAFETY_POLICY}" \
    --argjson bytes "${request_bytes}" \
    --argjson token_estimate "${token_estimate}" \
    --argjson elapsed_ns "${elapsed_ns}" \
    --argjson timeout_ms "${TIMEOUT_MS}" \
    --argjson offload_enabled "$(if [[ "${OFFLOAD_ENABLED}" == "0" ]]; then echo false; else echo true; fi)" \
    --argjson local_accept_enabled "$(if [[ "${LOCAL_ACCEPT_ENABLED}" == "1" ]]; then echo true; else echo false; fi)" \
    --argjson client_allow_local_accept "$(if [[ "${CLIENT_ALLOW_LOCAL_ACCEPT}" == "1" ]]; then echo true; else echo false; fi)" \
    --argjson client_require_verifier "$(if [[ "${CLIENT_REQUIRE_VERIFIER}" == "1" ]]; then echo true; else echo false; fi)" \
    --argjson client_require_false_accepts_zero "$(if [[ "${CLIENT_REQUIRE_FALSE_ACCEPTS_ZERO}" == "1" ]]; then echo true; else echo false; fi)" \
    --argjson client_kill_switch "$(if [[ "${CLIENT_KILL_SWITCH}" == "1" ]]; then echo true; else echo false; fi)" \
    --argjson raw_capture_enabled "$(if [[ "${CAPTURE_RAW}" == "1" ]]; then echo true; else echo false; fi)" \
    '{
      schema_version: "nando_llm_gateway_event_v1",
      timestamp: $ts,
      stage: $stage,
      decision: $decision,
      reason: $reason,
      request_sha256: $sha256,
      request_bytes: $bytes,
      input_tokens_estimated: $token_estimate,
      elapsed_ns: $elapsed_ns,
      timeout_ms: $timeout_ms,
      env_file: $env_file,
      client_id: $client_id,
      client_tier: $client_tier,
      safety_policy: $safety_policy,
      offload_enabled: $offload_enabled,
      local_accept_enabled: $local_accept_enabled,
      client_allow_local_accept: $client_allow_local_accept,
      client_require_verifier: $client_require_verifier,
      client_require_false_accepts_zero: $client_require_false_accepts_zero,
      client_kill_switch: $client_kill_switch,
      raw_capture_enabled: $raw_capture_enabled,
      boundary: "fail-open LLM gateway telemetry only unless server policy allows local_accept and a verifier-bound local command returns a response"
    }' >> "${EVENTS_JSONL}" || true
}

write_gateway_event "ingress" "observed" "request_recorded" 0

run_fallback() {
  local reason="$1"
  local before_ns after_ns elapsed_ns status
  before_ns="$(date +%s%N)"
  set +e
  if [[ "$#" -gt 1 ]]; then
    "${@:2}" < "${request_file}"
    status=$?
  elif [[ -n "${NANDO_FALLBACK_CMD:-}" ]]; then
    bash -lc "${NANDO_FALLBACK_CMD}" < "${request_file}"
    status=$?
  else
    echo "nando-llm-gateway: fallback command missing" >&2
    status=127
  fi
  set -e
  after_ns="$(date +%s%N)"
  elapsed_ns=$((after_ns - before_ns))
  write_gateway_event "egress" "fallback" "${reason}" "${elapsed_ns}"
  exit "${status}"
}

if [[ "${OFFLOAD_ENABLED}" == "0" ]]; then
  run_fallback "kill_switch_off" "$@"
fi

if [[ "${CLIENT_KILL_SWITCH}" == "1" ]]; then
  run_fallback "client_kill_switch_on" "$@"
fi

if [[ "${LOCAL_ACCEPT_ENABLED}" != "1" ]]; then
  run_fallback "local_accept_disabled" "$@"
fi

if [[ "${CLIENT_ALLOW_LOCAL_ACCEPT}" != "1" ]]; then
  run_fallback "client_local_accept_not_allowed" "$@"
fi

if [[ -z "${NANDO_GATEWAY_LOCAL_CMD:-}" ]]; then
  run_fallback "local_cmd_missing" "$@"
fi

export NANDO_ENV_FILE="${ENV_FILE}"
set +e
timeout "${timeout_arg}" bash -lc "${NANDO_GATEWAY_LOCAL_CMD}" < "${request_file}" > "${local_file}"
local_status=$?
set -e
elapsed_ns=$(( $(date +%s%N) - started_ns ))

if [[ "${local_status}" -eq 124 ]]; then
  run_fallback "local_cmd_timeout" "$@"
elif [[ "${local_status}" -ne 0 ]]; then
  run_fallback "local_cmd_error" "$@"
fi

local_accept="$(jq -r '(.local_accept // false) | tostring' "${local_file}" 2>/dev/null || echo false)"
verifier_ok="$(jq -r '(.verifier_ok // .verified_safe_accept // false) | tostring' "${local_file}" 2>/dev/null || echo false)"
false_accepts="$(jq -r '(.false_accepts // 0) | tostring' "${local_file}" 2>/dev/null || echo 0)"
local_route="$(jq -r '(.route // empty)' "${local_file}" 2>/dev/null || true)"
response_text="$(jq -r '(.response // .output // .output_text // empty)' "${local_file}" 2>/dev/null || true)"

if [[ "${CLIENT_REQUIRE_VERIFIER}" == "1" && "${verifier_ok}" != "true" ]]; then
  run_fallback "verifier_required_not_ok" "$@"
fi

if [[ "${CLIENT_REQUIRE_FALSE_ACCEPTS_ZERO}" == "1" && "${false_accepts}" != "0" ]]; then
  run_fallback "false_accepts_nonzero" "$@"
fi

if [[ "${local_accept}" == "true" && "${verifier_ok}" == "true" && -n "${response_text}" ]]; then
  printf '%s' "${response_text}"
  jq -cn \
    --arg ts "$(date -Is)" \
    --arg sha256 "${request_sha256}" \
    --arg reason "verifier_bound_local_accept" \
    --arg local_route "${local_route}" \
    --argjson elapsed_ns "${elapsed_ns}" \
    --argjson tokens_saved "${token_estimate}" \
    '{
      schema_version: "nando_llm_gateway_decision_v1",
      timestamp: $ts,
      request_sha256: $sha256,
      decision: "local_accept",
      reason: $reason,
      local_route: $local_route,
      elapsed_ns: $elapsed_ns,
      tokens_saved_estimated: $tokens_saved,
      false_accepts: 0,
      boundary: "local response emitted only because local_accept=true and verifier_ok=true were returned by the configured local command"
    }' >> "${DECISIONS_JSONL}" || true
  write_gateway_event "egress" "local_accept" "verifier_bound_local_accept" "${elapsed_ns}"
  exit 0
fi

run_fallback "local_declined_or_unverified" "$@"
