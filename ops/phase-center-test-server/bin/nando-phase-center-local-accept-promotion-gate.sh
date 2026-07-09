#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
if [[ -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "${ENV_FILE}"
  set +a
fi

READINESS_JSON="${NANDO_READINESS_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.readiness.json}"
VERIFY_JSON="${NANDO_VERIFY_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.test-server-verify.json}"
PROMOTION_REPORT="${NANDO_LOCAL_ACCEPT_PROMOTION_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.local-accept-promotion.json}"
POLICY_CANDIDATE="${NANDO_LOCAL_ACCEPT_POLICY_CANDIDATE:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.local-accept-policy-candidate.json}"

mkdir -p "$(dirname "${PROMOTION_REPORT}")" "$(dirname "${POLICY_CANDIDATE}")"

if [[ ! -s "${READINESS_JSON}" || ! -s "${VERIFY_JSON}" ]]; then
  jq -n \
    --arg readiness_json "${READINESS_JSON}" \
    --arg verify_json "${VERIFY_JSON}" \
    --arg policy_candidate "${POLICY_CANDIDATE}" \
    '{
      report_kind: "nando_phase_center_local_accept_promotion_gate_v1",
      readiness_json: $readiness_json,
      verify_json: $verify_json,
      policy_candidate_json: $policy_candidate,
      promotion_allowed: false,
      local_accept_policy_candidate_written: false,
      local_accept_enabled: false,
      blocker: "readiness_or_verify_missing",
      boundary: "local-accept promotion gate only: no serving mutation, no systemd change, no local_accept enable"
    }' > "${PROMOTION_REPORT}"
  echo "${PROMOTION_REPORT}"
  exit 0
fi

jq -n \
  --arg readiness_json "${READINESS_JSON}" \
  --arg verify_json "${VERIFY_JSON}" \
  --arg policy_candidate_json "${POLICY_CANDIDATE}" \
  --slurpfile readiness "${READINESS_JSON}" \
  --slurpfile verify "${VERIFY_JSON}" '
  def b($v): if $v then true else false end;
  def n($v): $v // 0;

  ($readiness[0] // {}) as $r |
  ($verify[0] // {}) as $v |
  (b($v.install_ready)
    and b($v.shadow_metrics_ready)
    and b($r.compression_claim_allowed)
    and b($r.local_accept_promotion_allowed)
    and n($r.scorecard.false_accepts) == 0
    and n($r.scorecard.local_accept_events) == 0
    and n($r.scorecard.unique_cpu_accepts_over_exact_cache) > 0
    and n($r.scorecard.tokens_saved) > 0) as $promotion_allowed |
  (if (b($v.install_ready) | not) then "install_not_ready"
   elif (b($v.shadow_metrics_ready) | not) then "shadow_metrics_not_ready"
   elif (b($r.compression_claim_allowed) | not) then ($r.blocker // "compression_not_ready")
   elif (b($r.local_accept_promotion_allowed) | not) then ($r.blocker // "token_local_accept_promotion_not_ready")
   elif n($r.scorecard.false_accepts) != 0 then "false_accepts_nonzero"
   elif n($r.scorecard.local_accept_events) != 0 then "local_accept_already_enabled"
   elif n($r.scorecard.unique_cpu_accepts_over_exact_cache) == 0 then "no_unique_cpu_accepts"
   elif n($r.scorecard.tokens_saved) == 0 then "no_tokens_saved"
   else "none"
   end) as $blocker |
  {
    report_kind: "nando_phase_center_local_accept_promotion_gate_v1",
    readiness_json: $readiness_json,
    verify_json: $verify_json,
    policy_candidate_json: $policy_candidate_json,
    promotion_allowed: $promotion_allowed,
    local_accept_policy_candidate_written: true,
    local_accept_enabled: b($r.server_policy.local_accept_enabled),
    requires_manual_activation_after_review: true,
    blocker: $blocker,
    required_conditions: {
      install_ready: b($v.install_ready),
      shadow_metrics_ready: b($v.shadow_metrics_ready),
      compression_claim_allowed: b($r.compression_claim_allowed),
      local_accept_promotion_allowed: b($r.local_accept_promotion_allowed),
      money_evidence_ready: b($r.money_evidence_ready),
      market_money_claim_allowed: b($r.market_money_claim_allowed),
      false_accepts_zero: n($r.scorecard.false_accepts) == 0,
      gateway_false_accepts_zero: n($r.scorecard.gateway_false_accepts) == 0,
      provider_bridge_false_accepts_zero: n($r.scorecard.provider_bridge_false_accepts) == 0,
      local_accept_events_zero: n($r.scorecard.local_accept_events) == 0,
      unique_accepts_positive: n($r.scorecard.unique_cpu_accepts_over_exact_cache) > 0,
      tokens_saved_positive: n($r.scorecard.tokens_saved) > 0,
      provider_capture_rows_positive: n($r.evidence.provider_boundary_capture_request_rows) > 0,
      provider_billing_evidence_present: b($r.evidence.provider_billing_evidence_present),
      external_evidence_chain_ready: b($r.evidence.external_evidence_chain_ready)
    },
    scorecard: {
      stable_rows: n($r.scorecard.stable_rows),
      unique_cpu_accepts_over_exact_cache: n($r.scorecard.unique_cpu_accepts_over_exact_cache),
      tokens_saved: n($r.scorecard.tokens_saved),
      cost_saved_microusd: n($r.scorecard.cost_saved_microusd),
      false_accepts: n($r.scorecard.false_accepts),
      local_accept_events: n($r.scorecard.local_accept_events),
      gateway_local_accept_events: n($r.scorecard.gateway_local_accept_events),
      gateway_tokens_saved_estimated: n($r.scorecard.gateway_tokens_saved_estimated),
      gateway_false_accepts: n($r.scorecard.gateway_false_accepts),
      gateway_local_route_count: n($r.scorecard.gateway_local_route_count),
      provider_bridge_local_accept_events: n($r.scorecard.provider_bridge_local_accept_events),
      provider_bridge_tokens_saved_estimated: n($r.scorecard.provider_bridge_tokens_saved_estimated),
      provider_bridge_false_accepts: n($r.scorecard.provider_bridge_false_accepts),
      provider_bridge_local_route_count: n($r.scorecard.provider_bridge_local_route_count)
    },
    server_policy: ($r.server_policy // {}),
    runtime_canary_active: b($r.runtime_canary_active),
    runtime_canary_safe:
      (b($r.runtime_canary_active)
        and n($r.scorecard.gateway_false_accepts) == 0
        and n($r.scorecard.provider_bridge_false_accepts) == 0
        and b($r.server_policy.local_accept_enabled)
        and b($r.server_policy.client_allow_local_accept))
    ,
    forbidden_flags: {
      nwrb_used: false,
      role_binding_backend_used: false,
      lookup_used: false,
      target_id_or_proof_rule_id_authority_used: false,
      concrete_x_lookup_used: false,
      manual_local_out_t_used: false,
      local_accept_without_verifier_used: false
    },
    verdict: (if $promotion_allowed then
      "NANDO_PHASE_CENTER_LOCAL_ACCEPT_PROMOTION_GATE_READY_FOR_MANUAL_REVIEW"
    else
      "NANDO_PHASE_CENTER_LOCAL_ACCEPT_PROMOTION_GATE_BLOCKED"
    end),
    boundary: "token-first local-accept promotion gate only: writes a disabled policy candidate and report; no serving mutation, no systemd change, no automatic provider call skipping; market money claim remains separate"
  }' > "${PROMOTION_REPORT}"

jq \
  --arg promotion_report "${PROMOTION_REPORT}" \
  --slurpfile promotion "${PROMOTION_REPORT}" '
  ($promotion[0] // {}) as $p |
  {
    schema_version: "nando_phase_center_local_accept_policy_candidate_v1",
    source_promotion_report: $promotion_report,
    policy_candidate_allowed: ($p.promotion_allowed // false),
    local_accept_enabled: false,
    requires_manual_activation_after_review: true,
    admission_conditions: $p.required_conditions,
    scorecard: $p.scorecard,
    server_policy: ($p.server_policy // {}),
    runtime_canary_active: ($p.runtime_canary_active // false),
    runtime_canary_safe: ($p.runtime_canary_safe // false),
    blocker: ($p.blocker // "unknown"),
    forbidden_flags: $p.forbidden_flags,
    boundary: "disabled local-accept policy candidate only; it is not installed into serving and cannot skip provider calls by itself"
  }' "${PROMOTION_REPORT}" > "${POLICY_CANDIDATE}"

echo "${PROMOTION_REPORT}"
