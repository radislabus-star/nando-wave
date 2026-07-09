#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
if [[ -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "${ENV_FILE}"
  set +a
fi

REPORT="${NANDO_LIVE_TAIL_REPORT:-/var/lib/nando-wave/streaming/nando-phase-live-miner-tail.report.json}"
OUT_JSON="${NANDO_METRICS_SNAPSHOT_JSON:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.metrics.json}"
OUT_PROM="${NANDO_METRICS_SNAPSHOT_PROM:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.prom}"
GATEWAY_DECISIONS_JSONL="${NANDO_GATEWAY_DECISIONS_JSONL:-/var/lib/nando-wave/streaming/nando-llm-gateway.decisions.jsonl}"
GATEWAY_METRICS_WINDOW="${NANDO_GATEWAY_METRICS_WINDOW:-1000}"
PROVIDER_BRIDGE_DECISIONS_JSONL="${NANDO_PROVIDER_BRIDGE_DECISIONS_JSONL:-/var/lib/nando-wave/streaming/nando-provider-bridge.decisions.jsonl}"
PROVIDER_BRIDGE_METRICS_WINDOW="${NANDO_PROVIDER_BRIDGE_METRICS_WINDOW:-1000}"
PROVIDER_BRIDGE_BOUNDARY_JSONL="${NANDO_PROVIDER_BRIDGE_BOUNDARY_EVENTS_JSONL:-/var/lib/nando-wave/streaming/nando-provider-bridge.provider-boundary-events.jsonl}"
CLEAN_PROMOTION_MANIFEST_JSON="${NANDO_CLEAN_PROMOTION_MANIFEST_JSON:-/var/lib/nando-wave/streaming/nando-phase-live-miner-tail.report-clean-promotion-manifest.json}"
DASHBOARD_HISTORY_JSONL="${NANDO_STATUS_DASHBOARD_HISTORY_JSONL:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.dashboard-history.jsonl}"
DASHBOARD_HISTORY_COMPACT_MAX_BYTES="${NANDO_STATUS_DASHBOARD_COMPACT_MAX_BYTES:-4194304}"
DASHBOARD_HISTORY_RETAIN_POINTS="${NANDO_STATUS_DASHBOARD_RETAIN_POINTS:-1440}"

mkdir -p "$(dirname "${OUT_JSON}")" "$(dirname "${OUT_PROM}")" "$(dirname "${DASHBOARD_HISTORY_JSONL}")"

write_atomic_file() {
  local target="$1"
  local tmp
  tmp="$(mktemp "${target}.tmp.XXXXXX")"
  cat > "${tmp}"
  chmod 0644 "${tmp}" 2>/dev/null || true
  mv "${tmp}" "${target}"
}

append_dashboard_history_snapshot() {
  local metrics_json="$1"
  local row
  row="$(
    jq -c '
      def ct($prefix):
        ([ (.operator_class_token_ranking // [])[] |
          select(((.class // "") | tostring) | startswith($prefix)) |
          (.tokens_saved // 0)
        ] | add) // 0;
      (.stable_serving_cpu_clean_suffix_tokens_saved // .stable_clean_token_compression_saved_tokens // 0) as $clean_saved |
      (.stable_serving_cpu_clean_suffix_total_tokens // .stable_clean_token_compression_total_tokens // 0) as $clean_total |
      if $clean_total <= 0 then empty else {
        schema_version: "nando_status_dashboard_history_v1",
        history_source: "metrics_snapshot_timer",
        timestamp: (now | todateiso8601),
        epoch: (now | floor),
        source_report_present: (.source_report_present // false),
        clean_saved_tokens: $clean_saved,
        clean_total_tokens: $clean_total,
        clean_compression_pct: (($clean_saved * 100) / $clean_total),
        clean_cpu_accepts: (.stable_serving_cpu_clean_suffix_unique_cpu_accepts_over_exact_cache // .stable_clean_token_compression_unique_cpu_accepts_over_exact_cache // 0),
        clean_false_accepts: (.stable_serving_cpu_clean_suffix_false_accepts // .product_hot_score_only_post_quarantine_false_accepts // 0),
        active_false_accepts: (.product_hot_score_only_post_quarantine_false_accepts // 0),
        shadow_false_accepts: (.stable_clean_token_compression_false_accepts // 0),
        gateway_accepts: (.gateway_local_accept_events // 0),
        provider_v2_accepts: (.provider_bridge_v2_local_accept_events // 0),
        edge_serving_cpu_accepts: (.edge_serving_cpu_local_accept_events // 0),
        edge_saved_tokens: (.edge_serving_cpu_tokens_saved_estimated // 0),
        edge_serving_cpu_tokens: (.edge_serving_cpu_tokens_saved_estimated // 0),
        edge_serving_cpu_false_accepts: (.edge_serving_cpu_false_accepts // 0),
        quarantined_tokens: (ct("hidden_state:quarantined") + ct("observable_subcenter:quarantined")),
        exportable_tokens: (ct("hidden_state:exportable") + ct("observable_subcenter:exportable") + ct("observable_primary:exportable")),
        final_hot_tokens: (ct("hidden_state:final_hot") + ct("observable_subcenter:final_hot"))
      } end
    ' "${metrics_json}" 2>/dev/null || true
  )"
  [[ -n "${row}" ]] || return 0
  printf '%s\n' "${row}" >> "${DASHBOARD_HISTORY_JSONL}" || return 0

  local max_bytes current_bytes retain tmp
  max_bytes="${DASHBOARD_HISTORY_COMPACT_MAX_BYTES}"
  [[ "${max_bytes}" =~ ^[0-9]+$ ]] || max_bytes=0
  (( max_bytes > 0 )) || return 0
  current_bytes="$(stat -c%s "${DASHBOARD_HISTORY_JSONL}" 2>/dev/null || echo 0)"
  [[ "${current_bytes}" =~ ^[0-9]+$ ]] || current_bytes=0
  (( current_bytes > max_bytes )) || return 0

  retain="${DASHBOARD_HISTORY_RETAIN_POINTS}"
  [[ "${retain}" =~ ^[0-9]+$ ]] || retain=1440
  (( retain > 0 )) || retain=1440
  tmp="$(mktemp "${DASHBOARD_HISTORY_JSONL}.tmp.XXXXXX")" || return 0
  if tail -n "${retain}" "${DASHBOARD_HISTORY_JSONL}" > "${tmp}"; then
    mv "${tmp}" "${DASHBOARD_HISTORY_JSONL}"
  else
    rm -f "${tmp}"
  fi
}

gateway_window="$(mktemp)"
provider_bridge_window="$(mktemp)"
provider_bridge_boundary_window="$(mktemp)"
clean_promotion_manifest_window="$(mktemp)"
trap 'rm -f "${gateway_window}" "${provider_bridge_window}" "${provider_bridge_boundary_window}" "${clean_promotion_manifest_window}"' EXIT
if [[ -s "${GATEWAY_DECISIONS_JSONL}" ]]; then
  tail -n "${GATEWAY_METRICS_WINDOW}" "${GATEWAY_DECISIONS_JSONL}" > "${gateway_window}"
fi
if [[ -s "${PROVIDER_BRIDGE_DECISIONS_JSONL}" ]]; then
  tail -n "${PROVIDER_BRIDGE_METRICS_WINDOW}" "${PROVIDER_BRIDGE_DECISIONS_JSONL}" > "${provider_bridge_window}"
fi
if [[ -s "${PROVIDER_BRIDGE_BOUNDARY_JSONL}" ]]; then
  tail -n "${PROVIDER_BRIDGE_METRICS_WINDOW}" "${PROVIDER_BRIDGE_BOUNDARY_JSONL}" > "${provider_bridge_boundary_window}"
fi
if [[ -s "${CLEAN_PROMOTION_MANIFEST_JSON}" ]]; then
  cp "${CLEAN_PROMOTION_MANIFEST_JSON}" "${clean_promotion_manifest_window}"
else
  printf '{}\n' > "${clean_promotion_manifest_window}"
fi

if [[ ! -s "${REPORT}" ]]; then
  jq -n --arg source_report "${REPORT}" '{
    report_kind: "nando_phase_center_metrics_snapshot_v1",
    source_report: $source_report,
    source_report_present: false,
    stable_decision_log_rows: 0,
    stable_decision_log_claim_allowed: false,
    market_money_claim_allowed: false,
    gateway_decision_window_rows: 0,
    gateway_local_accept_events: 0,
    gateway_tokens_saved_estimated: 0,
    gateway_false_accepts: 0,
    provider_bridge_decision_window_rows: 0,
    provider_bridge_local_accept_events: 0,
    provider_bridge_tokens_saved_estimated: 0,
    provider_bridge_false_accepts: 0,
    provider_bridge_v1_local_accept_events: 0,
    provider_bridge_v1_tokens_saved_estimated: 0,
    provider_bridge_v1_false_accepts: 0,
    provider_bridge_v2_local_accept_events: 0,
    provider_bridge_v2_tokens_saved_estimated: 0,
    provider_bridge_v2_false_accepts: 0,
    provider_bridge_v2_dogfood_local_accept_events: 0,
    provider_bridge_v2_dogfood_tokens_saved_estimated: 0,
    provider_bridge_v2_dogfood_false_accepts: 0,
    provider_bridge_v2_non_dogfood_local_accept_events: 0,
    provider_bridge_v2_non_dogfood_tokens_saved_estimated: 0,
    provider_bridge_v2_non_dogfood_false_accepts: 0,
    edge_serving_cpu_local_accept_events: 0,
    edge_serving_cpu_tokens_saved_estimated: 0,
    edge_serving_cpu_false_accepts: 0,
    provider_bridge_boundary_window_rows: 0,
  provider_bridge_boundary_total_tokens: 0,
  provider_bridge_boundary_total_cost_microusd: 0,
  operator_class_token_ranking: [],
  operator_profile_token_ranking: [],
  quarantined_profile_token_ranking: [],
  miner_saturation_control_enabled: false,
    miner_saturation_active: false,
    miner_saturation_sleep_events: 0,
    miner_saturation_last_sleep_ms: 0,
    miner_saturation_idle_heartbeats: 0,
    miner_active_batch_rows: 0,
    miner_active_batch_sleep_ms: 0,
    miner_active_batch_sleep_events: 0,
    blocker: "source_report_missing"
  }' | write_atomic_file "${OUT_JSON}"
  append_dashboard_history_snapshot "${OUT_JSON}"
  {
    echo "nando_phase_source_report_present 0"
    echo "nando_phase_stable_rows 0"
    echo "nando_phase_claim_allowed 0"
    echo "nando_phase_market_money_claim_allowed 0"
    echo "nando_phase_gateway_local_accept_events 0"
    echo "nando_phase_gateway_tokens_saved_estimated 0"
    echo "nando_phase_gateway_false_accepts 0"
    echo "nando_phase_provider_bridge_local_accept_events 0"
    echo "nando_phase_provider_bridge_tokens_saved_estimated 0"
    echo "nando_phase_provider_bridge_false_accepts 0"
    echo "nando_phase_provider_bridge_v1_local_accept_events 0"
    echo "nando_phase_provider_bridge_v1_tokens_saved_estimated 0"
    echo "nando_phase_provider_bridge_v1_false_accepts 0"
    echo "nando_phase_provider_bridge_v2_local_accept_events 0"
    echo "nando_phase_provider_bridge_v2_tokens_saved_estimated 0"
    echo "nando_phase_provider_bridge_v2_false_accepts 0"
    echo "nando_phase_provider_bridge_v2_dogfood_local_accept_events 0"
    echo "nando_phase_provider_bridge_v2_dogfood_tokens_saved_estimated 0"
    echo "nando_phase_provider_bridge_v2_dogfood_false_accepts 0"
    echo "nando_phase_provider_bridge_v2_non_dogfood_local_accept_events 0"
    echo "nando_phase_provider_bridge_v2_non_dogfood_tokens_saved_estimated 0"
    echo "nando_phase_provider_bridge_v2_non_dogfood_false_accepts 0"
    echo "nando_phase_edge_serving_cpu_local_accept_events 0"
    echo "nando_phase_edge_serving_cpu_tokens_saved_estimated 0"
    echo "nando_phase_edge_serving_cpu_false_accepts 0"
    echo "nando_phase_provider_bridge_boundary_window_rows 0"
    echo "nando_phase_provider_bridge_boundary_total_tokens 0"
    echo "nando_phase_provider_bridge_boundary_total_cost_microusd 0"
    echo "nando_phase_miner_saturation_control_enabled 0"
    echo "nando_phase_miner_saturation_active 0"
    echo "nando_phase_miner_saturation_sleep_events 0"
    echo "nando_phase_miner_saturation_last_sleep_ms 0"
    echo "nando_phase_miner_saturation_idle_heartbeats 0"
    echo "nando_phase_miner_active_batch_rows 0"
    echo "nando_phase_miner_active_batch_sleep_ms 0"
    echo "nando_phase_miner_active_batch_sleep_events 0"
  } | write_atomic_file "${OUT_PROM}"
  echo "${OUT_JSON}"
  exit 0
fi

jq --arg source_report "${REPORT}" \
  --arg gateway_decisions_jsonl "${GATEWAY_DECISIONS_JSONL}" \
  --arg provider_bridge_decisions_jsonl "${PROVIDER_BRIDGE_DECISIONS_JSONL}" \
  --arg provider_bridge_boundary_jsonl "${PROVIDER_BRIDGE_BOUNDARY_JSONL}" \
  --arg clean_promotion_manifest_json "${CLEAN_PROMOTION_MANIFEST_JSON}" \
  --argjson gateway_metrics_window "${GATEWAY_METRICS_WINDOW}" \
  --argjson provider_bridge_metrics_window "${PROVIDER_BRIDGE_METRICS_WINDOW}" \
  --slurpfile gateway "${gateway_window}" \
  --slurpfile provider_bridge "${provider_bridge_window}" \
  --slurpfile provider_bridge_boundary "${provider_bridge_boundary_window}" \
  --slurpfile clean_promotion_manifest "${clean_promotion_manifest_window}" '
  def n($v): $v // 0;
  def b($v): if $v then true else false end;
  ($gateway // []) as $g |
  ($provider_bridge // []) as $pb |
  ($provider_bridge_boundary // []) as $pbb |
  ($clean_promotion_manifest[0] // {}) as $clean_manifest |
  (($clean_manifest.allowed == true)
    and (($clean_manifest.false_accepts // 0) == 0)
    and (($clean_manifest.runtime_parity_mismatches // 0) == 0)) as $clean_manifest_safe |
  ([ if $clean_manifest_safe then
       $clean_manifest.routes[]? | .profile_id
     else empty end
   ] | unique) as $clean_promoted_profile_ids |
  ([ $g[] | select((.decision // "") == "local_accept") ]) as $local |
  ([ $pb[] | select((.decision // "") == "local_accept") ]) as $provider_local |
  ([ $provider_local[] | select((.api_version // "v1") == "v1") ]) as $provider_local_v1 |
  ([ $provider_local[] | select((.api_version // "v1") == "v2") ]) as $provider_local_v2 |
  ([ $provider_local_v2[] | select((.traffic_source // "") | startswith("dogfood")) ]) as $provider_local_v2_dogfood |
  ([ $provider_local_v2[] | select(((.traffic_source // "") | startswith("dogfood")) | not) ]) as $provider_local_v2_non_dogfood |
  ([ .clean_candidate_reports[]? |
      .profile_id as $candidate_profile_id |
      (($clean_promoted_profile_ids | index($candidate_profile_id)) != null) as $clean_manifest_promoted |
      . + {
        clean_manifest_promoted: $clean_manifest_promoted,
        _status:
          (if (.final_hot // false) then "final_hot"
           elif $clean_manifest_promoted then "exportable"
           elif (.quarantined // false) then "quarantined"
           elif (.exportable // false) then "exportable"
           elif (.candidate // false) then "candidate"
           else "watch" end)
      }
    ]) as $candidate_reports |
  ([ $candidate_reports[] |
      {
        class: (((.kind // "unknown") | tostring) + ":" + (._status | tostring)),
        profiles: 1,
        unique_cpu_accepts_over_exact_cache: (.unique_cpu_accepts_over_exact_cache // 0),
        tokens_saved: (.tokens_saved // 0),
        false_accepts: (.false_accepts // 0),
        events_seen: (.events_seen // 0)
      }
    ] | group_by(.class) |
    map({
      class: .[0].class,
      profiles: ([.[].profiles] | add // 0),
      unique_cpu_accepts_over_exact_cache: ([.[].unique_cpu_accepts_over_exact_cache] | add // 0),
      tokens_saved: ([.[].tokens_saved] | add // 0),
      false_accepts: ([.[].false_accepts] | add // 0),
      events_seen: ([.[].events_seen] | add // 0)
    }) | sort_by(-.tokens_saved)) as $operator_class_token_ranking |
  ([ $candidate_reports[] |
      select((._status // "") != "quarantined") |
      select((.false_accepts // 0) == 0) |
      {
        profile_id,
        kind,
        status: ._status,
        clean_manifest_promoted: (.clean_manifest_promoted // false),
        final_hot: (.final_hot // false),
        exportable: (.exportable // false),
        unique_cpu_accepts_over_exact_cache: (.unique_cpu_accepts_over_exact_cache // 0),
        tokens_saved: (.tokens_saved // 0),
        events_seen: (.events_seen // 0),
        negative_events: (.negative_events // 0),
        learned_threshold_micro: (.learned_threshold_micro // 0)
      }
    ] | sort_by(-.tokens_saved) | .[:20]) as $operator_profile_token_ranking |
  ([ $candidate_reports[] |
      select((._status // "") == "quarantined") |
      {
        profile_id,
        kind,
        route_id: (.route_id // null),
        active: (.active // false),
        candidate: (.candidate // false),
        shadow_ready: (.shadow_ready // false),
        rejected: (.rejected // false),
        exportable: (.exportable // false),
        final_hot: (.final_hot // false),
        unique_cpu_accepts_over_exact_cache: (.unique_cpu_accepts_over_exact_cache // 0),
        tokens_saved: (.tokens_saved // 0),
        events_seen: (.events_seen // 0),
        false_accepts: (.false_accepts // 0),
        negative_events: (.negative_events // 0),
        learned_threshold_micro: (.learned_threshold_micro // 0),
        calibration_events_seen: (.calibration_events_seen // 0),
        max_calibration_false_margin_micro: (.max_calibration_false_margin_micro // 0),
        trust_drift_micro: (.trust_drift_micro // 0),
        trust_false_risk_micro: (.trust_false_risk_micro // 0),
        trust_quality_micro: (.trust_quality_micro // 0),
        trust_token_value_micro: (.trust_token_value_micro // 0),
        auto_recovery_running: (.auto_recovery_running // false),
        promotion_blocker: (.promotion_blocker // "unknown"),
        next_auto_action: (.next_auto_action // "unknown"),
        best_split_candidate: (.best_split_candidate // "unknown"),
        recovery_retry_after_events: (.recovery_retry_after_events // 0)
      }
    ] | sort_by(-.tokens_saved) | .[:20]) as $quarantined_profile_token_ranking |
  ([ $local[] | (.tokens_saved_estimated // 0) ] | add // 0) as $gateway_tokens_saved |
  ([ $provider_local[] | (.tokens_saved_estimated // 0) ] | add // 0) as $provider_bridge_tokens_saved |
  ([ $provider_local_v1[] | (.tokens_saved_estimated // 0) ] | add // 0) as $provider_bridge_v1_tokens_saved |
  ([ $provider_local_v2[] | (.tokens_saved_estimated // 0) ] | add // 0) as $provider_bridge_v2_tokens_saved |
  ([ $provider_local_v2_dogfood[] | (.tokens_saved_estimated // 0) ] | add // 0) as $provider_bridge_v2_dogfood_tokens_saved |
  ([ $provider_local_v2_non_dogfood[] | (.tokens_saved_estimated // 0) ] | add // 0) as $provider_bridge_v2_non_dogfood_tokens_saved |
  ([ $pbb[] | (.provider_total_tokens // .token_cost.total_tokens // 0) ] | add // 0) as $provider_bridge_boundary_tokens |
  ([ $pbb[] | (.provider_cost_microusd // .token_cost.total_cost_microusd // 0) ] | add // 0) as $provider_bridge_boundary_cost |
  ([ $local[] | (.false_accepts // 0) ] | add // 0) as $gateway_false_accepts |
  ([ $provider_local[] | (.false_accepts // 0) ] | add // 0) as $provider_bridge_false_accepts |
  ([ $provider_local_v1[] | (.false_accepts // 0) ] | add // 0) as $provider_bridge_v1_false_accepts |
  ([ $provider_local_v2[] | (.false_accepts // 0) ] | add // 0) as $provider_bridge_v2_false_accepts |
  ([ $provider_local_v2_dogfood[] | (.false_accepts // 0) ] | add // 0) as $provider_bridge_v2_dogfood_false_accepts |
  ([ $provider_local_v2_non_dogfood[] | (.false_accepts // 0) ] | add // 0) as $provider_bridge_v2_non_dogfood_false_accepts |
  ([ $local[] | (.local_route // "unknown") ] | unique) as $gateway_routes |
  ([ $provider_local[] | (.local_route // "unknown") ] | unique) as $provider_bridge_routes |
  ([ $provider_local_v1[] | (.local_route // "unknown") ] | unique) as $provider_bridge_v1_routes |
  ([ $provider_local_v2[] | (.local_route // "unknown") ] | unique) as $provider_bridge_v2_routes |
  {
  report_kind: "nando_phase_center_metrics_snapshot_v1",
  source_report: $source_report,
  source_report_present: true,
  clean_promotion_manifest_json: $clean_promotion_manifest_json,
  clean_promotion_manifest_safe: $clean_manifest_safe,
  clean_promotion_manifest_promoted_profile_ids: $clean_promoted_profile_ids,
  gateway_decisions_jsonl: $gateway_decisions_jsonl,
  provider_bridge_decisions_jsonl: $provider_bridge_decisions_jsonl,
  provider_bridge_boundary_jsonl: $provider_bridge_boundary_jsonl,
  gateway_metrics_window: $gateway_metrics_window,
  provider_bridge_metrics_window: $provider_bridge_metrics_window,
  gateway_decision_window_rows: ($g | length),
  gateway_local_accept_events: ($local | length),
  gateway_tokens_saved_estimated: $gateway_tokens_saved,
  gateway_false_accepts: $gateway_false_accepts,
  gateway_local_route_count: ($gateway_routes | length),
  gateway_local_routes: $gateway_routes,
  provider_bridge_decision_window_rows: ($pb | length),
  provider_bridge_local_accept_events: ($provider_local | length),
  provider_bridge_tokens_saved_estimated: $provider_bridge_tokens_saved,
  provider_bridge_false_accepts: $provider_bridge_false_accepts,
  provider_bridge_local_route_count: ($provider_bridge_routes | length),
  provider_bridge_local_routes: $provider_bridge_routes,
  provider_bridge_v1_local_accept_events: ($provider_local_v1 | length),
  provider_bridge_v1_tokens_saved_estimated: $provider_bridge_v1_tokens_saved,
  provider_bridge_v1_false_accepts: $provider_bridge_v1_false_accepts,
  provider_bridge_v1_local_route_count: ($provider_bridge_v1_routes | length),
  provider_bridge_v1_local_routes: $provider_bridge_v1_routes,
  provider_bridge_v2_local_accept_events: ($provider_local_v2 | length),
  provider_bridge_v2_tokens_saved_estimated: $provider_bridge_v2_tokens_saved,
  provider_bridge_v2_false_accepts: $provider_bridge_v2_false_accepts,
  provider_bridge_v2_dogfood_local_accept_events: ($provider_local_v2_dogfood | length),
  provider_bridge_v2_dogfood_tokens_saved_estimated: $provider_bridge_v2_dogfood_tokens_saved,
  provider_bridge_v2_dogfood_false_accepts: $provider_bridge_v2_dogfood_false_accepts,
  provider_bridge_v2_non_dogfood_local_accept_events: ($provider_local_v2_non_dogfood | length),
  provider_bridge_v2_non_dogfood_tokens_saved_estimated: $provider_bridge_v2_non_dogfood_tokens_saved,
  provider_bridge_v2_non_dogfood_false_accepts: $provider_bridge_v2_non_dogfood_false_accepts,
  provider_bridge_v2_local_route_count: ($provider_bridge_v2_routes | length),
  provider_bridge_v2_local_routes: $provider_bridge_v2_routes,
  provider_bridge_v2_transition_runtime_events:
    ([ $provider_local_v2[] | select((.architecture // "") == "compact_latent_transition_runtime") ] | length),
  edge_serving_cpu_local_accept_events: (($local | length) + ($provider_local_v2 | length)),
  edge_serving_cpu_tokens_saved_estimated: ($gateway_tokens_saved + $provider_bridge_v2_tokens_saved),
  edge_serving_cpu_false_accepts: ($gateway_false_accepts + $provider_bridge_v2_false_accepts),
  provider_bridge_boundary_window_rows: ($pbb | length),
  provider_bridge_boundary_total_tokens: $provider_bridge_boundary_tokens,
  provider_bridge_boundary_total_cost_microusd: $provider_bridge_boundary_cost,
  provider_bridge_boundary_cost_evidence_ready: ($provider_bridge_boundary_cost > 0),
  operator_class_token_ranking: $operator_class_token_ranking,
  operator_profile_token_ranking: $operator_profile_token_ranking,
  quarantined_profile_token_ranking: $quarantined_profile_token_ranking,
  server_policy_local_accept_enabled: (env.NANDO_LOCAL_ACCEPT_ENABLED == "1"),
  server_policy_client_allow_local_accept: (env.NANDO_CLIENT_ALLOW_LOCAL_ACCEPT == "1"),
  server_policy_safety_policy: (env.NANDO_CLIENT_SAFETY_POLICY // ""),
  miner_saturation_control_enabled: (.miner_saturation_control_enabled // false),
  miner_saturation_active: (.miner_saturation_active // false),
  miner_saturation_sleep_events: (.miner_saturation_sleep_events // 0),
  miner_saturation_last_sleep_ms: (.miner_saturation_last_sleep_ms // 0),
  miner_saturation_idle_heartbeats: (.miner_saturation_idle_heartbeats // 0),
  miner_active_batch_rows: (.miner_active_batch_rows // 0),
  miner_active_batch_sleep_ms: (.miner_active_batch_sleep_ms // 0),
  miner_active_batch_sleep_events: (.miner_active_batch_sleep_events // 0),
  architecture_version_key: (.stable_decision_log_architecture_key // .architecture_version_key // ""),
  append_parsed_rows: (.append_parsed_rows // 0),
  append_score_candidate_events: (.append_score_candidate_events // 0),
  append_unique_cpu_accepts_over_exact_cache: (.append_unique_cpu_accepts_over_exact_cache // 0),
  append_tokens_saved: (.append_tokens_saved // 0),
  append_false_accepts: (.append_false_accepts // 0),
  active_clean_calls_saved: (.active_clean_calls_saved // 0),
  active_clean_tokens_saved: (.active_clean_tokens_saved // 0),
  active_clean_cost_saved_microusd: (.append_estimated_cost_saved_microusd // .append_cost_saved_microusd // 0),
  edge_serving_cpu_local_accept_events: (($local | length) + ($provider_local_v2 | length)),
  edge_serving_cpu_tokens_saved_estimated: ($gateway_tokens_saved + $provider_bridge_v2_tokens_saved),
  edge_serving_cpu_false_accepts: ($gateway_false_accepts + $provider_bridge_v2_false_accepts),
  quarantine_recovery_discovery_events: (.quarantine_recovery_discovery_events // 0),
  quarantine_recovery_discovery_tokens: (.quarantine_recovery_discovery_tokens // 0),
  quarantine_recovery_auto_subcenter_observe_events: (.quarantine_recovery_auto_subcenter_observe_events // 0),
  stable_decision_log_rows: (.stable_decision_log_rows // 0),
  stable_decision_log_score_candidate_events: (.stable_decision_log_score_candidate_events // 0),
  stable_decision_log_unique_cpu_accepts_over_exact_cache: (.stable_decision_log_unique_cpu_accepts_over_exact_cache // 0),
  stable_decision_log_tokens_saved: (.stable_decision_log_tokens_saved // 0),
  stable_decision_log_cost_saved_microusd: (.stable_decision_log_cost_saved_microusd // 0),
  stable_decision_log_false_accepts: (.stable_decision_log_false_accepts // 0),
  stable_decision_log_local_accept_events: (.stable_decision_log_local_accept_events // 0),
  stable_decision_log_total_tokens: (.stable_decision_log_total_tokens // 0),
  stable_decision_log_total_cost_microusd: (.stable_decision_log_total_cost_microusd // 0),
  stable_decision_log_claim_allowed: (.stable_decision_log_claim_allowed // false),
  stable_decision_log_claim_blocker: (.stable_decision_log_claim_blocker // ""),
  stable_clean_token_compression_claim_allowed: (.stable_clean_token_compression_claim_allowed // false),
  stable_clean_token_compression_claim_blocker: (.stable_clean_token_compression_claim_blocker // ""),
  stable_clean_token_compression_unique_cpu_accepts_over_exact_cache: (.stable_clean_token_compression_unique_cpu_accepts_over_exact_cache // 0),
  stable_clean_token_compression_saved_tokens: (.stable_clean_token_compression_saved_tokens // 0),
  stable_clean_token_compression_total_tokens: (.stable_clean_token_compression_total_tokens // 0),
  stable_clean_token_compression_saved_milli: (.stable_clean_token_compression_saved_milli // 0),
  stable_clean_token_compression_false_accepts: (.stable_clean_token_compression_false_accepts // 0),
  stable_serving_cpu_rows: (.stable_serving_cpu_rows // 0),
  stable_serving_cpu_score_candidate_events: (.stable_serving_cpu_score_candidate_events // 0),
  stable_serving_cpu_local_accept_events: (.stable_serving_cpu_local_accept_events // 0),
  stable_serving_cpu_unique_cpu_accepts_over_exact_cache: (.stable_serving_cpu_unique_cpu_accepts_over_exact_cache // 0),
  stable_serving_cpu_tokens_saved: (.stable_serving_cpu_tokens_saved // 0),
  stable_serving_cpu_total_tokens: (.stable_serving_cpu_total_tokens // 0),
  stable_serving_cpu_false_accepts: (.stable_serving_cpu_false_accepts // 0),
  stable_serving_cpu_claim_allowed: (.stable_serving_cpu_claim_allowed // false),
  stable_serving_cpu_claim_blocker: (.stable_serving_cpu_claim_blocker // ""),
  stable_serving_cpu_clean_suffix_rows: (.stable_serving_cpu_clean_suffix_rows // 0),
  stable_serving_cpu_clean_suffix_score_candidate_events: (.stable_serving_cpu_clean_suffix_score_candidate_events // 0),
  stable_serving_cpu_clean_suffix_local_accept_events: (.stable_serving_cpu_clean_suffix_local_accept_events // 0),
  stable_serving_cpu_clean_suffix_unique_cpu_accepts_over_exact_cache: (.stable_serving_cpu_clean_suffix_unique_cpu_accepts_over_exact_cache // 0),
  stable_serving_cpu_clean_suffix_tokens_saved: (.stable_serving_cpu_clean_suffix_tokens_saved // 0),
  stable_serving_cpu_clean_suffix_total_tokens: (.stable_serving_cpu_clean_suffix_total_tokens // 0),
  stable_serving_cpu_clean_suffix_false_accepts: (.stable_serving_cpu_clean_suffix_false_accepts // 0),
  stable_serving_cpu_clean_suffix_saved_milli: (.stable_serving_cpu_clean_suffix_saved_milli // 0),
  stable_serving_cpu_clean_suffix_claim_allowed: (.stable_serving_cpu_clean_suffix_claim_allowed // false),
  stable_serving_cpu_clean_suffix_claim_blocker: (.stable_serving_cpu_clean_suffix_claim_blocker // ""),
  stable_decision_log_clean_suffix_rows: (.stable_decision_log_clean_suffix_rows // 0),
  stable_decision_log_clean_suffix_score_candidate_events: (.stable_decision_log_clean_suffix_score_candidate_events // 0),
  stable_decision_log_clean_suffix_local_accept_events: (.stable_decision_log_clean_suffix_local_accept_events // 0),
  final_hot_runtime_available: (.final_hot_runtime_available // false),
  final_hot_profile_count: (.final_hot_profile_count // 0),
  product_hot_score_only_runtime_active: (.product_hot_score_only_runtime_active // false),
  product_hot_score_only_active_profile_count: (.product_hot_score_only_active_profile_count // 0),
  product_hot_score_only_post_quarantine_score_candidate_events: (.product_hot_score_only_post_quarantine_score_candidate_events // 0),
  product_hot_score_only_post_quarantine_false_accepts: (.product_hot_score_only_post_quarantine_false_accepts // 0),
  product_hot_phase_trust_filtered_events: (.product_hot_phase_trust_filtered_events // 0),
  product_hot_score_only_unique_cpu_accepts_over_exact_cache: (.product_hot_score_only_unique_cpu_accepts_over_exact_cache // 0),
  product_hot_score_only_tokens_saved: (.product_hot_score_only_tokens_saved // 0),
  product_hot_score_only_cost_saved_microusd: (.product_hot_score_only_cost_saved_microusd // 0),
  product_hot_compression_claim_blocker:
    (if (.append_parsed_rows // 0) == 0 then "append_no_rows"
     elif (.local_accept_enabled // false) then "local_accept_already_enabled"
     elif ((.final_hot_runtime_available // false) | not) then "final_hot_runtime_missing"
     elif ((.product_hot_score_only_runtime_active // false) | not) then "product_hot_runtime_inactive"
     elif (.product_hot_score_only_active_profile_count // .final_hot_profile_count // 0) == 0 then "product_hot_profile_count_zero"
     elif (.product_hot_score_only_post_quarantine_score_candidate_events // 0) == 0 then "product_hot_post_quarantine_window_missing"
     elif (.product_hot_score_only_post_quarantine_false_accepts // 0) != 0 then "product_hot_post_quarantine_false_accepts_nonzero"
     elif (.product_hot_score_only_unique_cpu_accepts_over_exact_cache // 0) == 0 then "product_hot_unique_accepts_zero"
     elif (.product_hot_score_only_tokens_saved // 0) == 0 then "product_hot_tokens_saved_zero"
     elif ((.append_total_tokens // 0) == 0 or (.append_total_cost_microusd // 0) == 0) then "token_cost_denominator_missing"
     elif (.append_parsed_rows // 0) < (.append_compression_claim_min_rows // 100) then "append_window_below_min_rows"
     else "none" end),
  product_hot_compression_claim_allowed:
    ((if (.append_parsed_rows // 0) == 0 then "append_no_rows"
      elif (.local_accept_enabled // false) then "local_accept_already_enabled"
      elif ((.final_hot_runtime_available // false) | not) then "final_hot_runtime_missing"
      elif ((.product_hot_score_only_runtime_active // false) | not) then "product_hot_runtime_inactive"
      elif (.product_hot_score_only_active_profile_count // .final_hot_profile_count // 0) == 0 then "product_hot_profile_count_zero"
      elif (.product_hot_score_only_post_quarantine_score_candidate_events // 0) == 0 then "product_hot_post_quarantine_window_missing"
      elif (.product_hot_score_only_post_quarantine_false_accepts // 0) != 0 then "product_hot_post_quarantine_false_accepts_nonzero"
      elif (.product_hot_score_only_unique_cpu_accepts_over_exact_cache // 0) == 0 then "product_hot_unique_accepts_zero"
      elif (.product_hot_score_only_tokens_saved // 0) == 0 then "product_hot_tokens_saved_zero"
      elif ((.append_total_tokens // 0) == 0 or (.append_total_cost_microusd // 0) == 0) then "token_cost_denominator_missing"
      elif (.append_parsed_rows // 0) < (.append_compression_claim_min_rows // 100) then "append_window_below_min_rows"
      else "none" end) == "none"),
  future_shadow_billing_request_rows: (.future_shadow_billing_request_rows // 0),
  future_shadow_billing_request_tokens: (.future_shadow_billing_request_tokens // 0),
  future_shadow_billing_request_current_cost_microusd: (.future_shadow_billing_request_current_cost_microusd // 0),
  provider_export_present: (.provider_export_present // false),
  provider_money_claim_blocker: (.provider_money_claim_blocker // ""),
  market_money_claim_allowed: (.market_money_claim_allowed // false),
  local_accept_enabled: (.local_accept_enabled // false),
  product_runtime_changed: (.product_runtime_changed // false),
  serving_runtime_changed: (.serving_runtime_changed // false)
}' "${REPORT}" | write_atomic_file "${OUT_JSON}"
append_dashboard_history_snapshot "${OUT_JSON}"

jq -r '
  def b($v): if $v then 1 else 0 end;
  [
    "nando_phase_source_report_present 1",
    "nando_phase_append_parsed_rows " + ((.append_parsed_rows // 0)|tostring),
    "nando_phase_append_score_candidate_events " + ((.append_score_candidate_events // 0)|tostring),
    "nando_phase_append_unique_cpu_accepts_over_exact_cache " + ((.append_unique_cpu_accepts_over_exact_cache // 0)|tostring),
    "nando_phase_append_tokens_saved " + ((.append_tokens_saved // 0)|tostring),
    "nando_phase_append_false_accepts " + ((.append_false_accepts // 0)|tostring),
    "nando_phase_active_clean_calls_saved " + ((.active_clean_calls_saved // 0)|tostring),
    "nando_phase_active_clean_tokens_saved " + ((.active_clean_tokens_saved // 0)|tostring),
    "nando_phase_active_clean_cost_saved_microusd " + ((.active_clean_cost_saved_microusd // 0)|tostring),
    "nando_phase_quarantine_recovery_discovery_events " + ((.quarantine_recovery_discovery_events // 0)|tostring),
    "nando_phase_quarantine_recovery_discovery_tokens " + ((.quarantine_recovery_discovery_tokens // 0)|tostring),
    "nando_phase_quarantine_recovery_auto_subcenter_observe_events " + ((.quarantine_recovery_auto_subcenter_observe_events // 0)|tostring),
    "nando_phase_stable_rows " + ((.stable_decision_log_rows // 0)|tostring),
    "nando_phase_stable_score_candidate_events " + ((.stable_decision_log_score_candidate_events // 0)|tostring),
    "nando_phase_stable_unique_cpu_accepts_over_exact_cache " + ((.stable_decision_log_unique_cpu_accepts_over_exact_cache // 0)|tostring),
    "nando_phase_stable_tokens_saved " + ((.stable_decision_log_tokens_saved // 0)|tostring),
    "nando_phase_stable_cost_saved_microusd " + ((.stable_decision_log_cost_saved_microusd // 0)|tostring),
    "nando_phase_stable_false_accepts " + ((.stable_decision_log_false_accepts // 0)|tostring),
    "nando_phase_stable_local_accept_events " + ((.stable_decision_log_local_accept_events // 0)|tostring),
    "nando_phase_stable_total_tokens " + ((.stable_decision_log_total_tokens // 0)|tostring),
    "nando_phase_stable_total_cost_microusd " + ((.stable_decision_log_total_cost_microusd // 0)|tostring),
    "nando_phase_stable_claim_allowed " + (b(.stable_decision_log_claim_allowed // false)|tostring),
    "nando_phase_stable_clean_token_compression_claim_allowed " + (b(.stable_clean_token_compression_claim_allowed // false)|tostring),
    "nando_phase_stable_clean_token_compression_unique_cpu_accepts_over_exact_cache " + ((.stable_clean_token_compression_unique_cpu_accepts_over_exact_cache // 0)|tostring),
    "nando_phase_stable_clean_token_compression_saved_tokens " + ((.stable_clean_token_compression_saved_tokens // 0)|tostring),
    "nando_phase_stable_clean_token_compression_false_accepts " + ((.stable_clean_token_compression_false_accepts // 0)|tostring),
    "nando_phase_stable_serving_cpu_rows " + ((.stable_serving_cpu_rows // 0)|tostring),
    "nando_phase_stable_serving_cpu_local_accept_events " + ((.stable_serving_cpu_local_accept_events // 0)|tostring),
    "nando_phase_stable_serving_cpu_unique_cpu_accepts_over_exact_cache " + ((.stable_serving_cpu_unique_cpu_accepts_over_exact_cache // 0)|tostring),
    "nando_phase_stable_serving_cpu_tokens_saved " + ((.stable_serving_cpu_tokens_saved // 0)|tostring),
    "nando_phase_stable_serving_cpu_false_accepts " + ((.stable_serving_cpu_false_accepts // 0)|tostring),
    "nando_phase_stable_serving_cpu_clean_suffix_rows " + ((.stable_serving_cpu_clean_suffix_rows // 0)|tostring),
    "nando_phase_stable_serving_cpu_clean_suffix_local_accept_events " + ((.stable_serving_cpu_clean_suffix_local_accept_events // 0)|tostring),
    "nando_phase_stable_serving_cpu_clean_suffix_unique_cpu_accepts_over_exact_cache " + ((.stable_serving_cpu_clean_suffix_unique_cpu_accepts_over_exact_cache // 0)|tostring),
    "nando_phase_stable_serving_cpu_clean_suffix_tokens_saved " + ((.stable_serving_cpu_clean_suffix_tokens_saved // 0)|tostring),
    "nando_phase_stable_serving_cpu_clean_suffix_false_accepts " + ((.stable_serving_cpu_clean_suffix_false_accepts // 0)|tostring),
    "nando_phase_final_hot_runtime_available " + (b(.final_hot_runtime_available // false)|tostring),
    "nando_phase_final_hot_profile_count " + ((.final_hot_profile_count // 0)|tostring),
    "nando_phase_product_hot_runtime_active " + (b(.product_hot_score_only_runtime_active // false)|tostring),
    "nando_phase_product_hot_active_profile_count " + ((.product_hot_score_only_active_profile_count // 0)|tostring),
    "nando_phase_product_hot_post_quarantine_score_candidate_events " + ((.product_hot_score_only_post_quarantine_score_candidate_events // 0)|tostring),
    "nando_phase_post_quarantine_false_accepts " + ((.product_hot_score_only_post_quarantine_false_accepts // 0)|tostring),
    "nando_phase_product_hot_phase_trust_filtered_events " + ((.product_hot_phase_trust_filtered_events // 0)|tostring),
    "nando_phase_product_hot_unique_cpu_accepts_over_exact_cache " + ((.product_hot_score_only_unique_cpu_accepts_over_exact_cache // 0)|tostring),
    "nando_phase_product_hot_tokens_saved " + ((.product_hot_score_only_tokens_saved // 0)|tostring),
    "nando_phase_product_hot_cost_saved_microusd " + ((.product_hot_score_only_cost_saved_microusd // 0)|tostring),
    "nando_phase_product_hot_compression_claim_allowed " + (b(.product_hot_compression_claim_allowed // false)|tostring),
    "nando_phase_future_shadow_billing_request_rows " + ((.future_shadow_billing_request_rows // 0)|tostring),
    "nando_phase_future_shadow_billing_request_tokens " + ((.future_shadow_billing_request_tokens // 0)|tostring),
    "nando_phase_future_shadow_billing_request_current_cost_microusd " + ((.future_shadow_billing_request_current_cost_microusd // 0)|tostring),
    "nando_phase_provider_export_present " + (b(.provider_export_present // false)|tostring),
    "nando_phase_market_money_claim_allowed " + (b(.market_money_claim_allowed // false)|tostring),
    "nando_phase_local_accept_enabled " + (b(.local_accept_enabled // false)|tostring),
    "nando_phase_server_policy_local_accept_enabled " + (b(.server_policy_local_accept_enabled // false)|tostring),
    "nando_phase_server_policy_client_allow_local_accept " + (b(.server_policy_client_allow_local_accept // false)|tostring),
    "nando_phase_miner_saturation_control_enabled " + (b(.miner_saturation_control_enabled // false)|tostring),
    "nando_phase_miner_saturation_active " + (b(.miner_saturation_active // false)|tostring),
    "nando_phase_miner_saturation_sleep_events " + ((.miner_saturation_sleep_events // 0)|tostring),
    "nando_phase_miner_saturation_last_sleep_ms " + ((.miner_saturation_last_sleep_ms // 0)|tostring),
    "nando_phase_miner_saturation_idle_heartbeats " + ((.miner_saturation_idle_heartbeats // 0)|tostring),
    "nando_phase_miner_active_batch_rows " + ((.miner_active_batch_rows // 0)|tostring),
    "nando_phase_miner_active_batch_sleep_ms " + ((.miner_active_batch_sleep_ms // 0)|tostring),
    "nando_phase_miner_active_batch_sleep_events " + ((.miner_active_batch_sleep_events // 0)|tostring),
    "nando_phase_gateway_decision_window_rows " + ((.gateway_decision_window_rows // 0)|tostring),
    "nando_phase_gateway_local_accept_events " + ((.gateway_local_accept_events // 0)|tostring),
    "nando_phase_gateway_tokens_saved_estimated " + ((.gateway_tokens_saved_estimated // 0)|tostring),
    "nando_phase_gateway_false_accepts " + ((.gateway_false_accepts // 0)|tostring),
    "nando_phase_gateway_local_route_count " + ((.gateway_local_route_count // 0)|tostring),
    "nando_phase_provider_bridge_decision_window_rows " + ((.provider_bridge_decision_window_rows // 0)|tostring),
    "nando_phase_provider_bridge_local_accept_events " + ((.provider_bridge_local_accept_events // 0)|tostring),
    "nando_phase_provider_bridge_tokens_saved_estimated " + ((.provider_bridge_tokens_saved_estimated // 0)|tostring),
    "nando_phase_provider_bridge_false_accepts " + ((.provider_bridge_false_accepts // 0)|tostring),
    "nando_phase_provider_bridge_local_route_count " + ((.provider_bridge_local_route_count // 0)|tostring),
    "nando_phase_provider_bridge_v1_local_accept_events " + ((.provider_bridge_v1_local_accept_events // 0)|tostring),
    "nando_phase_provider_bridge_v1_tokens_saved_estimated " + ((.provider_bridge_v1_tokens_saved_estimated // 0)|tostring),
    "nando_phase_provider_bridge_v1_false_accepts " + ((.provider_bridge_v1_false_accepts // 0)|tostring),
    "nando_phase_provider_bridge_v1_local_route_count " + ((.provider_bridge_v1_local_route_count // 0)|tostring),
    "nando_phase_provider_bridge_v2_local_accept_events " + ((.provider_bridge_v2_local_accept_events // 0)|tostring),
    "nando_phase_provider_bridge_v2_tokens_saved_estimated " + ((.provider_bridge_v2_tokens_saved_estimated // 0)|tostring),
    "nando_phase_provider_bridge_v2_false_accepts " + ((.provider_bridge_v2_false_accepts // 0)|tostring),
    "nando_phase_provider_bridge_v2_local_route_count " + ((.provider_bridge_v2_local_route_count // 0)|tostring),
    "nando_phase_provider_bridge_v2_transition_runtime_events " + ((.provider_bridge_v2_transition_runtime_events // 0)|tostring),
    "nando_phase_provider_bridge_v2_dogfood_local_accept_events " + ((.provider_bridge_v2_dogfood_local_accept_events // 0)|tostring),
    "nando_phase_provider_bridge_v2_dogfood_tokens_saved_estimated " + ((.provider_bridge_v2_dogfood_tokens_saved_estimated // 0)|tostring),
    "nando_phase_provider_bridge_v2_dogfood_false_accepts " + ((.provider_bridge_v2_dogfood_false_accepts // 0)|tostring),
    "nando_phase_provider_bridge_v2_non_dogfood_local_accept_events " + ((.provider_bridge_v2_non_dogfood_local_accept_events // 0)|tostring),
    "nando_phase_provider_bridge_v2_non_dogfood_tokens_saved_estimated " + ((.provider_bridge_v2_non_dogfood_tokens_saved_estimated // 0)|tostring),
    "nando_phase_provider_bridge_v2_non_dogfood_false_accepts " + ((.provider_bridge_v2_non_dogfood_false_accepts // 0)|tostring),
    "nando_phase_edge_serving_cpu_local_accept_events " + ((.edge_serving_cpu_local_accept_events // 0)|tostring),
    "nando_phase_edge_serving_cpu_tokens_saved_estimated " + ((.edge_serving_cpu_tokens_saved_estimated // 0)|tostring),
    "nando_phase_edge_serving_cpu_false_accepts " + ((.edge_serving_cpu_false_accepts // 0)|tostring),
    "nando_phase_provider_bridge_boundary_window_rows " + ((.provider_bridge_boundary_window_rows // 0)|tostring),
    "nando_phase_provider_bridge_boundary_total_tokens " + ((.provider_bridge_boundary_total_tokens // 0)|tostring),
    "nando_phase_provider_bridge_boundary_total_cost_microusd " + ((.provider_bridge_boundary_total_cost_microusd // 0)|tostring),
    "nando_phase_provider_bridge_boundary_cost_evidence_ready " + (b(.provider_bridge_boundary_cost_evidence_ready // false)|tostring)
  ] | .[]
' "${OUT_JSON}" | write_atomic_file "${OUT_PROM}"

echo "${OUT_JSON}"
