#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
if [[ -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "${ENV_FILE}"
  set +a
fi

METRICS_JSON="${NANDO_METRICS_SNAPSHOT_JSON:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.metrics.json}"
EVIDENCE_JSON="${NANDO_PROVIDER_EVIDENCE_SNAPSHOT_REPORT:-/var/lib/nando-wave/streaming/provider-evidence/provider-evidence-snapshot.report.json}"
OUT_JSON="${NANDO_READINESS_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.readiness.json}"
OUT_PROM="${NANDO_READINESS_PROM:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.readiness.prom}"

mkdir -p "$(dirname "${OUT_JSON}")" "$(dirname "${OUT_PROM}")"

if [[ ! -s "${METRICS_JSON}" ]]; then
  jq -n --arg metrics_json "${METRICS_JSON}" '{
    report_kind: "nando_phase_center_readiness_snapshot_v1",
    metrics_json: $metrics_json,
    metrics_present: false,
    evidence_present: false,
    compression_claim_allowed: false,
    money_evidence_ready: false,
    market_money_claim_allowed: false,
    local_accept_promotion_allowed: false,
    blocker: "metrics_snapshot_missing",
    local_accept_enabled: false,
    boundary: "readiness gate only: no mining, scoring, serving, provider estimation, promotion, or local_accept"
  }' > "${OUT_JSON}"
else
  if [[ -s "${EVIDENCE_JSON}" ]]; then
    jq -n \
      --arg metrics_json "${METRICS_JSON}" \
      --arg evidence_json "${EVIDENCE_JSON}" \
      --slurpfile metrics "${METRICS_JSON}" \
      --slurpfile evidence "${EVIDENCE_JSON}" '
      def b($v): if $v then true else false end;
      def n($v): $v // 0;

      ($metrics[0] // {}) as $m |
      ($evidence[0] // {}) as $e |
      (b($m.stable_decision_log_claim_allowed)
        and n($m.stable_decision_log_unique_cpu_accepts_over_exact_cache) > 0
        and n($m.stable_decision_log_tokens_saved) > 0
        and n($m.stable_decision_log_false_accepts) == 0
        and b($m.final_hot_runtime_available)
        and (b($m.local_accept_enabled) | not)) as $compression_ready |
      (b($m.product_hot_compression_claim_allowed)
        and n($m.product_hot_score_only_unique_cpu_accepts_over_exact_cache) > 0
        and n($m.product_hot_score_only_tokens_saved) > 0
        and n($m.product_hot_score_only_post_quarantine_score_candidate_events) > 0
        and n($m.product_hot_score_only_post_quarantine_false_accepts) == 0
        and b($m.final_hot_runtime_available)
        and b($m.product_hot_score_only_runtime_active)
        and (b($m.local_accept_enabled) | not)) as $product_hot_compression_ready |
      (b($m.stable_clean_token_compression_claim_allowed)
        and n($m.stable_clean_token_compression_unique_cpu_accepts_over_exact_cache) > 0
        and n($m.stable_clean_token_compression_saved_tokens) > 0
        and n($m.stable_clean_token_compression_false_accepts) == 0
        and n($m.stable_decision_log_clean_suffix_local_accept_events) == 0
        and b($m.final_hot_runtime_available)
        and (b($m.local_accept_enabled) | not)) as $clean_token_compression_ready |
      (b($e.evidence_chain.market_money_claim_allowed)
        and b($e.evidence_chain.provider_billing_evidence_present)
        and b($e.evidence_chain.external_evidence_chain_ready)) as $money_evidence_ready |
      ($clean_token_compression_ready) as $token_local_accept_ready |
      ($clean_token_compression_ready and $money_evidence_ready and b($m.market_money_claim_allowed)) as $market_money_ready |
      (if n($m.stable_clean_token_compression_false_accepts) > 0 then "stable_clean_token_false_accepts_nonzero"
       elif b($m.local_accept_enabled) then "local_accept_already_enabled"
       elif (b($m.final_hot_runtime_available) | not) then "final_hot_runtime_missing"
       elif (b($m.stable_clean_token_compression_claim_allowed) | not) then ($m.stable_clean_token_compression_claim_blocker // "stable_clean_token_compression_claim_blocked")
       elif n($m.stable_clean_token_compression_unique_cpu_accepts_over_exact_cache) == 0 then "stable_clean_token_unique_accepts_zero"
       elif n($m.stable_clean_token_compression_saved_tokens) == 0 then "stable_clean_token_tokens_saved_zero"
       elif ($token_local_accept_ready and ($money_evidence_ready | not)) then ($e.blocker // $m.provider_money_claim_blocker // "provider_evidence_missing")
       elif (b($m.market_money_claim_allowed) | not) then ($m.provider_money_claim_blocker // "market_money_claim_blocked")
       else "none"
       end) as $blocker |
      {
        report_kind: "nando_phase_center_readiness_snapshot_v1",
        metrics_json: $metrics_json,
        evidence_json: $evidence_json,
        metrics_present: true,
        evidence_present: true,
        compression_claim_allowed: $clean_token_compression_ready,
        raw_stable_compression_claim_allowed: $compression_ready,
        raw_stable_compression_claim_blocker: ($m.stable_decision_log_claim_blocker // ""),
        product_hot_compression_claim_allowed: $product_hot_compression_ready,
        product_hot_compression_claim_blocker: ($m.product_hot_compression_claim_blocker // ""),
        stable_clean_token_compression_claim_allowed: $clean_token_compression_ready,
        stable_clean_token_compression_claim_blocker: ($m.stable_clean_token_compression_claim_blocker // ""),
        money_evidence_ready: $money_evidence_ready,
        market_money_claim_allowed: $market_money_ready,
        local_accept_promotion_allowed: $token_local_accept_ready,
        blocker: $blocker,
        scorecard: {
          stable_rows: n($m.stable_decision_log_rows),
          raw_stable_unique_cpu_accepts_over_exact_cache: n($m.stable_decision_log_unique_cpu_accepts_over_exact_cache),
          raw_stable_tokens_saved: n($m.stable_decision_log_tokens_saved),
          raw_stable_cost_saved_microusd: n($m.stable_decision_log_cost_saved_microusd),
          raw_stable_false_accepts: n($m.stable_decision_log_false_accepts),
          unique_cpu_accepts_over_exact_cache: n($m.stable_clean_token_compression_unique_cpu_accepts_over_exact_cache),
          tokens_saved: n($m.stable_clean_token_compression_saved_tokens),
          cost_saved_microusd: n($m.stable_clean_token_compression_saved_tokens),
          false_accepts: n($m.stable_clean_token_compression_false_accepts),
          product_hot_post_quarantine_score_candidate_events: n($m.product_hot_score_only_post_quarantine_score_candidate_events),
          local_accept_events: n($m.stable_decision_log_clean_suffix_local_accept_events),
          gateway_local_accept_events: n($m.gateway_local_accept_events),
          gateway_tokens_saved_estimated: n($m.gateway_tokens_saved_estimated),
          gateway_false_accepts: n($m.gateway_false_accepts),
          gateway_local_route_count: n($m.gateway_local_route_count),
          provider_bridge_local_accept_events: n($m.provider_bridge_local_accept_events),
          provider_bridge_tokens_saved_estimated: n($m.provider_bridge_tokens_saved_estimated),
          provider_bridge_false_accepts: n($m.provider_bridge_false_accepts),
          provider_bridge_local_route_count: n($m.provider_bridge_local_route_count),
          final_hot_profile_count: n($m.final_hot_profile_count),
          product_hot_active_profile_count: n($m.product_hot_score_only_active_profile_count),
          future_shadow_billing_request_rows: n($m.future_shadow_billing_request_rows),
          future_shadow_billing_request_tokens: n($m.future_shadow_billing_request_tokens),
          provider_export_present: b($m.provider_export_present)
        },
        evidence: {
          billing_request_rows: n($e.acquisition.billing_request_rows),
          provider_boundary_capture_request_rows: n($e.acquisition.provider_boundary_capture_request_rows),
          total_tokens_requiring_billing: n($e.acquisition.total_tokens_requiring_billing),
          provider_export_required: b($e.evidence_chain.provider_export_required),
          provider_billing_evidence_present: b($e.evidence_chain.provider_billing_evidence_present),
          external_evidence_chain_ready: b($e.evidence_chain.external_evidence_chain_ready),
          evidence_chain_verdict: ($e.evidence_chain.verdict // "")
        },
        forbidden_flags: {
          nwrb_used: false,
          role_binding_backend_used: false,
          lookup_used: false,
          target_id_or_proof_rule_id_authority_used: false,
          concrete_x_lookup_used: false,
          manual_local_out_t_used: false,
          local_accept_without_verifier_used: false
        },
        server_policy: {
          local_accept_enabled: b($m.server_policy_local_accept_enabled),
          client_allow_local_accept: b($m.server_policy_client_allow_local_accept),
          safety_policy: ($m.server_policy_safety_policy // "")
        },
        runtime_canary_active:
          (b($m.server_policy_local_accept_enabled)
            and b($m.server_policy_client_allow_local_accept)
            and (n($m.gateway_local_accept_events) > 0 or n($m.provider_bridge_local_accept_events) > 0)
            and n($m.gateway_false_accepts) == 0
            and n($m.provider_bridge_false_accepts) == 0),
        local_accept_enabled: b($m.server_policy_local_accept_enabled),
        boundary: "readiness gate only: joins metrics and provider-evidence reports; no mining, scoring, serving, provider estimation, promotion, or local_accept"
      }' > "${OUT_JSON}"
  else
    jq -n \
      --arg metrics_json "${METRICS_JSON}" \
      --arg evidence_json "${EVIDENCE_JSON}" \
      --slurpfile metrics "${METRICS_JSON}" '
      ($metrics[0] // {}) as $m |
      {
        report_kind: "nando_phase_center_readiness_snapshot_v1",
        metrics_json: $metrics_json,
        evidence_json: $evidence_json,
        metrics_present: true,
        evidence_present: false,
        compression_claim_allowed: ($m.product_hot_compression_claim_allowed // false),
        raw_stable_compression_claim_allowed: ($m.stable_decision_log_claim_allowed // false),
        raw_stable_compression_claim_blocker: ($m.stable_decision_log_claim_blocker // ""),
        product_hot_compression_claim_allowed: ($m.product_hot_compression_claim_allowed // false),
        product_hot_compression_claim_blocker: ($m.product_hot_compression_claim_blocker // ""),
        money_evidence_ready: false,
        market_money_claim_allowed: false,
        local_accept_promotion_allowed: false,
        blocker: "provider_evidence_snapshot_missing",
        scorecard: {
          stable_rows: ($m.stable_decision_log_rows // 0),
          raw_stable_unique_cpu_accepts_over_exact_cache: ($m.stable_decision_log_unique_cpu_accepts_over_exact_cache // 0),
          raw_stable_tokens_saved: ($m.stable_decision_log_tokens_saved // 0),
          raw_stable_false_accepts: ($m.stable_decision_log_false_accepts // 0),
          unique_cpu_accepts_over_exact_cache: ($m.product_hot_score_only_unique_cpu_accepts_over_exact_cache // 0),
          tokens_saved: ($m.product_hot_score_only_tokens_saved // 0),
          false_accepts: ($m.product_hot_score_only_post_quarantine_false_accepts // 0),
          product_hot_post_quarantine_score_candidate_events: ($m.product_hot_score_only_post_quarantine_score_candidate_events // 0),
          gateway_local_accept_events: ($m.gateway_local_accept_events // 0),
          gateway_tokens_saved_estimated: ($m.gateway_tokens_saved_estimated // 0),
          gateway_false_accepts: ($m.gateway_false_accepts // 0),
          gateway_local_route_count: ($m.gateway_local_route_count // 0),
          provider_bridge_local_accept_events: ($m.provider_bridge_local_accept_events // 0),
          provider_bridge_tokens_saved_estimated: ($m.provider_bridge_tokens_saved_estimated // 0),
          provider_bridge_false_accepts: ($m.provider_bridge_false_accepts // 0),
          provider_bridge_local_route_count: ($m.provider_bridge_local_route_count // 0)
        },
        server_policy: {
          local_accept_enabled: ($m.server_policy_local_accept_enabled // false),
          client_allow_local_accept: ($m.server_policy_client_allow_local_accept // false),
          safety_policy: ($m.server_policy_safety_policy // "")
        },
        runtime_canary_active:
          (($m.server_policy_local_accept_enabled // false)
            and ($m.server_policy_client_allow_local_accept // false)
            and ((($m.gateway_local_accept_events // 0) > 0) or (($m.provider_bridge_local_accept_events // 0) > 0))
            and (($m.gateway_false_accepts // 0) == 0)
            and (($m.provider_bridge_false_accepts // 0) == 0)),
        local_accept_enabled: ($m.server_policy_local_accept_enabled // false),
        boundary: "readiness gate only: no mining, scoring, serving, provider estimation, promotion, or local_accept"
      }' > "${OUT_JSON}"
  fi
fi

jq -r '
  def b($v): if $v then 1 else 0 end;
  [
    "nando_phase_readiness_compression_claim_allowed " + (b(.compression_claim_allowed // false)|tostring),
    "nando_phase_readiness_raw_stable_compression_claim_allowed " + (b(.raw_stable_compression_claim_allowed // false)|tostring),
    "nando_phase_readiness_product_hot_compression_claim_allowed " + (b(.product_hot_compression_claim_allowed // false)|tostring),
    "nando_phase_readiness_money_evidence_ready " + (b(.money_evidence_ready // false)|tostring),
    "nando_phase_readiness_market_money_claim_allowed " + (b(.market_money_claim_allowed // false)|tostring),
    "nando_phase_readiness_local_accept_promotion_allowed " + (b(.local_accept_promotion_allowed // false)|tostring),
    "nando_phase_readiness_stable_rows " + ((.scorecard.stable_rows // 0)|tostring),
    "nando_phase_readiness_unique_cpu_accepts_over_exact_cache " + ((.scorecard.unique_cpu_accepts_over_exact_cache // 0)|tostring),
    "nando_phase_readiness_tokens_saved " + ((.scorecard.tokens_saved // 0)|tostring),
    "nando_phase_readiness_false_accepts " + ((.scorecard.false_accepts // 0)|tostring),
    "nando_phase_readiness_raw_stable_false_accepts " + ((.scorecard.raw_stable_false_accepts // 0)|tostring),
    "nando_phase_readiness_product_hot_post_quarantine_score_candidate_events " + ((.scorecard.product_hot_post_quarantine_score_candidate_events // 0)|tostring),
    "nando_phase_readiness_gateway_local_accept_events " + ((.scorecard.gateway_local_accept_events // 0)|tostring),
    "nando_phase_readiness_gateway_tokens_saved_estimated " + ((.scorecard.gateway_tokens_saved_estimated // 0)|tostring),
    "nando_phase_readiness_gateway_false_accepts " + ((.scorecard.gateway_false_accepts // 0)|tostring),
    "nando_phase_readiness_gateway_local_route_count " + ((.scorecard.gateway_local_route_count // 0)|tostring),
    "nando_phase_readiness_provider_bridge_local_accept_events " + ((.scorecard.provider_bridge_local_accept_events // 0)|tostring),
    "nando_phase_readiness_provider_bridge_tokens_saved_estimated " + ((.scorecard.provider_bridge_tokens_saved_estimated // 0)|tostring),
    "nando_phase_readiness_provider_bridge_false_accepts " + ((.scorecard.provider_bridge_false_accepts // 0)|tostring),
    "nando_phase_readiness_provider_bridge_local_route_count " + ((.scorecard.provider_bridge_local_route_count // 0)|tostring),
    "nando_phase_readiness_server_policy_local_accept_enabled " + (b(.server_policy.local_accept_enabled // false)|tostring),
    "nando_phase_readiness_runtime_canary_active " + (b(.runtime_canary_active // false)|tostring),
    "nando_phase_readiness_billing_request_rows " + ((.evidence.billing_request_rows // 0)|tostring),
    "nando_phase_readiness_provider_billing_evidence_present " + (b(.evidence.provider_billing_evidence_present // false)|tostring)
  ] | .[]
' "${OUT_JSON}" > "${OUT_PROM}"

echo "${OUT_JSON}"
