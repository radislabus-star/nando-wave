#!/usr/bin/env bash
set -euo pipefail

request="$(cat)"
normalized="$(
  printf '%s' "${request}" |
    tr '[:upper:]' '[:lower:]' |
    tr -d '\r' |
    sed -E 's/[[:space:]]+/ /g; s/^ //; s/ $//'
)"

METRICS_JSON="${NANDO_METRICS_SNAPSHOT_JSON:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.metrics.json}"
READINESS_JSON="${NANDO_READINESS_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.readiness.json}"
PROMOTION_JSON="${NANDO_LOCAL_ACCEPT_PROMOTION_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.local-accept-promotion.json}"
VERIFY_JSON="${NANDO_VERIFY_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.test-server-verify.json}"
TYPED_ACTOR_CMD="${NANDO_TYPED_ACTOR_CMD:-/opt/nando-wave/bin/nando-transition-actor-exec}"
TYPED_ACTOR_PACKAGE="${NANDO_TYPED_ACTOR_PACKAGE:-/opt/nando-wave/ops/phase-center-test-server/packages/rsmod-portable-v1.json}"

emit_decline() {
  local reason="$1"
  jq -cn \
    --arg reason "${reason}" \
    '{
      local_accept: false,
      verifier_ok: false,
      reason: $reason,
      boundary: "decline means gateway must fallback to the normal provider"
    }'
}

emit_accept() {
  local response="$1"
  local route="$2"
  jq -cn \
    --arg response "${response}" \
    --arg route "${route}" \
    '{
      local_accept: true,
      verifier_ok: true,
      route: $route,
      response: $response,
      false_accepts: 0,
      boundary: "only exact verifier-bound local routes are accepted; no broad prompt answering"
    }'
}

json_file_ready() {
  local path="$1"
  [[ -s "${path}" ]] && jq -e type "${path}" >/dev/null 2>&1
}

emit_json_route() {
  local route="$1"
  local path="$2"
  local jq_filter="$3"
  local response
  if ! json_file_ready "${path}"; then
    emit_decline "${route}_artifact_missing_or_invalid"
    return
  fi
  if ! response="$(jq -er "${jq_filter}" "${path}" 2>/dev/null)"; then
    emit_decline "${route}_verifier_failed"
    return
  fi
  if [[ -z "${response}" || "${response}" == "null" ]]; then
    emit_decline "${route}_empty_response"
    return
  fi
  emit_accept "${response}" "${route}"
}

emit_compression_status() {
  emit_json_route "nando_compression_status" "${METRICS_JSON}" '
    def n($v): $v // 0;
    def b($v): if $v then "true" else "false" end;
    (n(.stable_clean_token_compression_saved_milli) / 10) as $pct |
    "NANDO_COMPRESSION clean_accepts="
    + (n(.stable_clean_token_compression_unique_cpu_accepts_over_exact_cache)|tostring)
    + " tokens_saved=" + (n(.stable_clean_token_compression_saved_tokens)|tostring)
    + " total_tokens=" + (n(.stable_clean_token_compression_total_tokens)|tostring)
    + " token_saved_pct=" + ($pct|tostring)
    + "% false_accepts=" + (n(.product_hot_score_only_post_quarantine_false_accepts)|tostring)
    + " shadow_false_accepts=" + (n(.stable_clean_token_compression_false_accepts)|tostring)
    + " hot_profiles=" + (n(.product_hot_score_only_active_profile_count)|tostring)
    + " market_money_claim_allowed=" + b(.market_money_claim_allowed)
    + " report=" + input_filename
  '
}

emit_readiness_status() {
  emit_json_route "nando_readiness_status" "${READINESS_JSON}" '
    def n($v): $v // 0;
    def b($v): if $v then "true" else "false" end;
    "NANDO_READINESS compression_claim_allowed=" + b(.compression_claim_allowed)
    + " local_accept_promotion_allowed=" + b(.local_accept_promotion_allowed)
    + " market_money_claim_allowed=" + b(.market_money_claim_allowed)
    + " blocker=" + (.blocker // "unknown")
    + " clean_accepts=" + (n(.scorecard.unique_cpu_accepts_over_exact_cache)|tostring)
    + " tokens_saved=" + (n(.scorecard.tokens_saved)|tostring)
    + " false_accepts=" + (n(.scorecard.false_accepts)|tostring)
    + " report=" + input_filename
  '
}

emit_promotion_status() {
  emit_json_route "nando_promotion_status" "${PROMOTION_JSON}" '
    def n($v): $v // 0;
    def b($v): if $v then "true" else "false" end;
    "NANDO_PROMOTION promotion_allowed=" + b(.promotion_allowed)
    + " blocker=" + (.blocker // "unknown")
    + " clean_accepts=" + (n(.scorecard.unique_cpu_accepts_over_exact_cache)|tostring)
    + " tokens_saved=" + (n(.scorecard.tokens_saved)|tostring)
    + " false_accepts=" + (n(.scorecard.false_accepts)|tostring)
    + " local_accept_events=" + (n(.scorecard.local_accept_events)|tostring)
    + " report=" + input_filename
  '
}

emit_server_status() {
  emit_json_route "nando_server_status" "${VERIFY_JSON}" '
    def n($v): $v // 0;
    def b($v): if $v then "true" else "false" end;
    "NANDO_SERVER verdict=" + (.verdict // "unknown")
    + " install_ready=" + b(.install_ready)
    + " shadow_metrics_ready=" + b(.shadow_metrics_ready)
    + " local_accept_policy=" + env.NANDO_CLIENT_SAFETY_POLICY
    + " local_accept_enabled=" + env.NANDO_LOCAL_ACCEPT_ENABLED
    + " clean_accepts=" + (n(.scorecard.unique_cpu_accepts_over_exact_cache)|tostring)
    + " tokens_saved=" + (n(.scorecard.tokens_saved)|tostring)
    + " false_accepts=" + (n(.scorecard.false_accepts)|tostring)
    + " blockers=" + ((.blockers // []) | join(","))
    + " report=" + input_filename
  '
}

request_left_trimmed="${request#"${request%%[![:space:]]*}"}"
if [[ "${request_left_trimmed:0:1}" == "{" \
  && "${request}" == *'"nando.transition-request.v1"'* ]] \
  && jq -e '.schema == "nando.transition-request.v1"' <<<"${request}" >/dev/null 2>&1; then
  if [[ ! -x "${TYPED_ACTOR_CMD}" || ! -r "${TYPED_ACTOR_PACKAGE}" ]]; then
    emit_decline "typed_actor_runtime_unavailable"
    exit 0
  fi
  actor_output="$(${TYPED_ACTOR_CMD} --package "${TYPED_ACTOR_PACKAGE}" <<<"${request}" 2>/dev/null || true)"
  if ! jq -e '
    type == "object"
    and (.local_accept | type == "boolean")
    and (.verifier_ok | type == "boolean")
    and ((.false_accepts // 0) == 0)
    and (.reason | type == "string")
    and (if .local_accept then
      .verifier_ok == true
      and (.route | startswith("typed_transition:"))
      and (.response | type == "string")
    else
      .verifier_ok == false
    end)
  ' <<<"${actor_output}" >/dev/null 2>&1; then
    emit_decline "typed_actor_protocol_invalid"
    exit 0
  fi
  printf '%s\n' "${actor_output}"
  exit 0
fi

case "${normalized}" in
  "nando health"|"nando:health"|"nando gateway health"|"nando-gateway health")
    emit_accept "NANDO_GATEWAY_OK" "nando_gateway_health"
    ;;
  "nando status"|"nando:status"|"nando gateway status"|"nando-gateway status")
    emit_server_status
    ;;
  "nando offload status"|"nando:offload-status")
    emit_accept "NANDO_OFFLOAD=1 LOCAL_ACCEPT=guarded" "nando_offload_status"
    ;;
  "nando compression"|"nando:compression"|"nando savings"|"nando:savings"|"какое сжатие"|"сколько сжатие"|"сжатие")
    emit_compression_status
    ;;
  "nando readiness"|"nando:readiness"|"nando ready"|"готовность")
    emit_readiness_status
    ;;
  "nando promotion"|"nando:promotion"|"nando local accept promotion"|"promotion gate")
    emit_promotion_status
    ;;
  "nando server"|"nando:server"|"сервер работает"|"статус сервера")
    emit_server_status
    ;;
  *)
    emit_decline "no_verifier_bound_local_route"
    ;;
esac
