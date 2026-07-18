#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
shift || true

REFRESH=0
WRITE_ONLY=0
while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --refresh)
      REFRESH=1
      shift
      ;;
    --write-only)
      WRITE_ONLY=1
      shift
      ;;
    --help|-h)
      cat <<'EOF'
nando phase-center status

Usage:
  nando-phase-center-status.sh [/etc/nando-wave/phase-center.env] [--refresh] [--write-only]

Output:
  one JSON status report for server health, upstream readiness, compression,
  local_accept policy, money-claim boundary, and key systemd services.

This command is read-only unless --refresh is passed. It never prints provider
secrets.
EOF
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "${ENV_FILE}"
  set +a
fi

OPS_DIR="${NANDO_PHASE_CENTER_OPS_DIR:-/opt/nando-wave/ops/phase-center-test-server}"
STATUS_JSON="${NANDO_STATUS_REPORT:-${NANDO_METRICS_DIR:-/var/lib/nando-wave/streaming/metrics}/nando-phase-center.status.json}"
VERIFY_JSON="${NANDO_VERIFY_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.test-server-verify.json}"
METRICS_JSON="${NANDO_METRICS_SNAPSHOT_JSON:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.metrics.json}"
READINESS_JSON="${NANDO_READINESS_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.readiness.json}"
UPSTREAM_JSON="${NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-upstream-readiness.json}"
UPSTREAM_SMOKE_JSON="${NANDO_PROVIDER_BRIDGE_UPSTREAM_SMOKE_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-upstream-smoke.json}"
UPSTREAM_ONBOARD_SMOKE_JSON="${NANDO_PROVIDER_BRIDGE_UPSTREAM_ONBOARD_SMOKE_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-upstream-onboard-smoke.json}"
ACTIVATION_GATE_JSON="${NANDO_PROVIDER_BRIDGE_ACTIVATION_GATE_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-activation-gate.json}"
ACTIVATE_SMOKE_JSON="${NANDO_PROVIDER_BRIDGE_ACTIVATE_SMOKE_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-activate-smoke.json}"
EVIDENCE_JSON="${NANDO_PROVIDER_EVIDENCE_SNAPSHOT_REPORT:-/var/lib/nando-wave/streaming/provider-evidence/provider-evidence-snapshot.report.json}"
BRIDGE_EVENTS_JSONL="${NANDO_PROVIDER_BRIDGE_EVENTS_JSONL:-/var/lib/nando-wave/streaming/nando-provider-bridge.events.jsonl}"
BRIDGE_DECISIONS_JSONL="${NANDO_PROVIDER_BRIDGE_DECISIONS_JSONL:-/var/lib/nando-wave/streaming/nando-provider-bridge.decisions.jsonl}"
LATENCY_WINDOW_ROWS="${NANDO_STATUS_LATENCY_WINDOW_ROWS:-1000}"
BIND="${NANDO_PROVIDER_BRIDGE_BIND:-127.0.0.1:8787}"
BASE_URL="http://${BIND}"

mkdir -p "$(dirname "${STATUS_JSON}")"

if [[ "${REFRESH}" == "1" && -x "${OPS_DIR}/bin/nando-phase-center-refresh-snapshots.sh" ]]; then
  "${OPS_DIR}/bin/nando-phase-center-refresh-snapshots.sh" "${ENV_FILE}" >/dev/null || true
fi
if [[ "${REFRESH}" == "1" && -x "${OPS_DIR}/bin/nando-phase-center-test-server-verify.sh" ]]; then
  "${OPS_DIR}/bin/nando-phase-center-test-server-verify.sh" "${ENV_FILE}" >/dev/null || true
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT
health_json="${tmpdir}/health.json"
verify_json="${tmpdir}/verify.json"
metrics_json="${tmpdir}/metrics.json"
readiness_json="${tmpdir}/readiness.json"
upstream_json="${tmpdir}/upstream.json"
upstream_smoke_json="${tmpdir}/upstream-smoke.json"
upstream_onboard_smoke_json="${tmpdir}/upstream-onboard-smoke.json"
activation_gate_json="${tmpdir}/activation-gate.json"
activate_smoke_json="${tmpdir}/activate-smoke.json"
evidence_json="${tmpdir}/evidence.json"
services_json="${tmpdir}/services.json"
bridge_latency_json="${tmpdir}/bridge-latency.json"
cpu_latency_json="${tmpdir}/cpu-latency.json"

write_json_or_empty() {
  local src="$1"
  local dst="$2"
  if [[ -s "${src}" ]] && jq -e . "${src}" >/dev/null 2>&1; then
    cp "${src}" "${dst}"
  else
    printf '{}\n' > "${dst}"
  fi
}

health_ok=false
health_status=0
if response="$(curl -fsS --max-time 1 "${BASE_URL}/health" 2>/dev/null)"; then
  health_status=200
  if printf '%s' "${response}" | jq -e . >/dev/null 2>&1; then
    printf '%s\n' "${response}" > "${health_json}"
    if jq -e '.ok == true' "${health_json}" >/dev/null 2>&1; then
      health_ok=true
    fi
  else
    printf '{}\n' > "${health_json}"
  fi
else
  printf '{}\n' > "${health_json}"
fi

write_json_or_empty "${VERIFY_JSON}" "${verify_json}"
write_json_or_empty "${METRICS_JSON}" "${metrics_json}"
write_json_or_empty "${READINESS_JSON}" "${readiness_json}"
write_json_or_empty "${UPSTREAM_JSON}" "${upstream_json}"
write_json_or_empty "${UPSTREAM_SMOKE_JSON}" "${upstream_smoke_json}"
write_json_or_empty "${UPSTREAM_ONBOARD_SMOKE_JSON}" "${upstream_onboard_smoke_json}"
write_json_or_empty "${ACTIVATION_GATE_JSON}" "${activation_gate_json}"
write_json_or_empty "${ACTIVATE_SMOKE_JSON}" "${activate_smoke_json}"
write_json_or_empty "${EVIDENCE_JSON}" "${evidence_json}"

units=(
  nando-phase-center-appender.service
  nando-phase-center-live-tail.service
  nando-provider-bridge.service
  nando-phase-center-metrics-snapshot.timer
  nando-phase-center-provider-evidence-snapshot.timer
  nando-phase-center-readiness-snapshot.timer
  nando-phase-center-test-server-verify.timer
  nando-phase-center-local-accept-promotion-gate.timer
  nando-phase-center-provider-activation-gate.timer
)

printf '{"count":0,"p50_ms":0,"p99_ms":0,"max_ms":0}\n' > "${bridge_latency_json}"
if [[ -s "${BRIDGE_EVENTS_JSONL}" ]]; then
  tail -n "${LATENCY_WINDOW_ROWS}" "${BRIDGE_EVENTS_JSONL}" | jq -Rsc '
    [split("\n")[] | fromjson? | select(.stage == "egress" and ((.elapsed_ns // 0) > 0)) | .elapsed_ns]
    | sort
    | . as $values
    | {
        count: ($values | length),
        p50_ms: (($values | if length == 0 then 0 else .[((length * 0.50 | ceil) - 1)] end) / 1000000),
        p99_ms: (($values | if length == 0 then 0 else .[((length * 0.99 | ceil) - 1)] end) / 1000000),
        max_ms: (($values | last // 0) / 1000000)
      }
  ' > "${bridge_latency_json}"
fi

printf '{"count":0,"p50_ms":0,"p99_ms":0,"max_ms":0}\n' > "${cpu_latency_json}"
if [[ -s "${BRIDGE_DECISIONS_JSONL}" ]]; then
  tail -n "${LATENCY_WINDOW_ROWS}" "${BRIDGE_DECISIONS_JSONL}" | jq -Rsc '
    [split("\n")[] | fromjson? | select(.decision == "local_accept" and ((.elapsed_ns // 0) > 0)) | .elapsed_ns]
    | sort
    | . as $values
    | {
        count: ($values | length),
        p50_ms: (($values | if length == 0 then 0 else .[((length * 0.50 | ceil) - 1)] end) / 1000000),
        p99_ms: (($values | if length == 0 then 0 else .[((length * 0.99 | ceil) - 1)] end) / 1000000),
        max_ms: (($values | last // 0) / 1000000)
      }
  ' > "${cpu_latency_json}"
fi

{
  for unit in "${units[@]}"; do
    active=false
    state="unknown"
    main_pid=0
    memory_current=0
    if command -v systemctl >/dev/null 2>&1; then
      if systemctl is-active --quiet "${unit}" 2>/dev/null; then
        active=true
      fi
      state="$(systemctl is-active "${unit}" 2>/dev/null || true)"
      state="${state:-unknown}"
      raw_main_pid="$(systemctl show "${unit}" -p MainPID --value 2>/dev/null || true)"
      raw_memory_current="$(systemctl show "${unit}" -p MemoryCurrent --value 2>/dev/null || true)"
      [[ "${raw_main_pid}" =~ ^[0-9]+$ ]] && main_pid="${raw_main_pid}"
      [[ "${raw_memory_current}" =~ ^[0-9]+$ ]] && memory_current="${raw_memory_current}"
    fi
    jq -n --arg unit "${unit}" --arg state "${state}" --argjson active "${active}" \
      --argjson main_pid "${main_pid}" --argjson memory_current "${memory_current}" \
      '{unit: $unit, active: $active, state: $state, main_pid: $main_pid, memory_current_bytes: $memory_current}'
  done
} | jq -s . > "${services_json}"

jq -n \
  --arg env_file "${ENV_FILE}" \
  --arg base_url "${BASE_URL}" \
  --arg status_json "${STATUS_JSON}" \
  --arg verify_json_path "${VERIFY_JSON}" \
  --arg metrics_json_path "${METRICS_JSON}" \
  --arg readiness_json_path "${READINESS_JSON}" \
  --arg upstream_json_path "${UPSTREAM_JSON}" \
  --arg upstream_smoke_json_path "${UPSTREAM_SMOKE_JSON}" \
  --arg upstream_onboard_smoke_json_path "${UPSTREAM_ONBOARD_SMOKE_JSON}" \
  --arg activation_gate_json_path "${ACTIVATION_GATE_JSON}" \
  --arg activate_smoke_json_path "${ACTIVATE_SMOKE_JSON}" \
  --arg evidence_json_path "${EVIDENCE_JSON}" \
  --argjson health_ok "${health_ok}" \
  --argjson health_status "${health_status}" \
  --argjson latency_window_rows "${LATENCY_WINDOW_ROWS}" \
  --slurpfile health "${health_json}" \
  --slurpfile verify "${verify_json}" \
  --slurpfile metrics "${metrics_json}" \
  --slurpfile readiness "${readiness_json}" \
  --slurpfile upstream "${upstream_json}" \
  --slurpfile upstream_smoke "${upstream_smoke_json}" \
  --slurpfile upstream_onboard_smoke "${upstream_onboard_smoke_json}" \
  --slurpfile activation_gate "${activation_gate_json}" \
  --slurpfile activate_smoke "${activate_smoke_json}" \
  --slurpfile evidence "${evidence_json}" \
  --slurpfile services "${services_json}" \
  --slurpfile bridge_latency "${bridge_latency_json}" \
  --slurpfile cpu_latency "${cpu_latency_json}" '
  def n($v): $v // 0;
  def b($v): if $v then true else false end;
  ($health[0] // {}) as $h |
  ($verify[0] // {}) as $v |
  ($metrics[0] // {}) as $m |
  ($readiness[0] // {}) as $r |
  ($upstream[0] // {}) as $u |
  ($upstream_smoke[0] // {}) as $us |
  ($upstream_onboard_smoke[0] // {}) as $uos |
  ($activation_gate[0] // {}) as $ag |
  ($activate_smoke[0] // {}) as $as |
  ($evidence[0] // {}) as $e |
  ($services[0] // []) as $svc |
  ($bridge_latency[0] // {}) as $bl |
  ($cpu_latency[0] // {}) as $cl |
  (b($health_ok)
    and b($v.install_ready)
    and b($v.shadow_metrics_ready)
    and n($v.scorecard.false_accepts) == 0) as $canary_ready |
  (b($u.ready_for_broad_provider_traffic)) as $broad_ready |
  (if (b($health_ok) | not) then "bridge_health_down"
   elif (b($v.install_ready) | not) then "install_not_ready"
   elif (b($v.shadow_metrics_ready) | not) then "shadow_metrics_not_ready"
   elif n($v.scorecard.false_accepts) != 0 then "false_accepts_nonzero"
   elif (b($u.upstream_configured) | not) then "configure_provider_upstream"
   elif (b($u.ready_for_broad_provider_traffic) | not) then "prove_upstream_readiness"
   elif (b($v.market_money_claim_allowed) | not) then "join_provider_billing_evidence"
   else "none" end) as $next_action |
  {
    report_kind: "nando_phase_center_status_v1",
    env_file: $env_file,
    status_json: $status_json,
    generated_utc: (now | todateiso8601),
    bridge: {
      base_url: $base_url,
      health_ok: $health_ok,
      health_status: $health_status,
      service: ($h.service // ""),
      local_accept_enabled: b($h.local_accept_enabled),
      client_allow_local_accept: b($h.client_allow_local_accept),
      safety_policy: ($h.safety_policy // ""),
      upstream_configured: b($h.upstream_configured),
      upstream_base_url_configured: b($h.upstream_base_url_configured),
      upstream_server_api_key_configured: b($h.upstream_server_api_key_configured),
      client_auth_forwarding_supported: b($h.client_auth_forwarding_supported)
    },
    verify: {
      path: $verify_json_path,
      verdict: ($v.verdict // "missing"),
      install_ready: b($v.install_ready),
      shadow_metrics_ready: b($v.shadow_metrics_ready),
      compression_claim_allowed: b($v.compression_claim_allowed),
      local_accept_enabled: b($v.local_accept_enabled),
      market_money_claim_allowed: b($v.market_money_claim_allowed),
      blockers: ($v.blockers // []),
      forbidden_flags: ($v.forbidden_flags // {})
    },
    scorecard: {
      stable_rows: n($v.scorecard.stable_rows),
      unique_cpu_accepts_over_exact_cache: n($v.scorecard.unique_cpu_accepts_over_exact_cache),
      tokens_saved: n($v.scorecard.tokens_saved),
      false_accepts: n($v.scorecard.false_accepts)
    },
    readiness: {
      path: $readiness_json_path,
      blocker: ($r.blocker // "missing"),
      local_accept_promotion_allowed: b($r.local_accept_promotion_allowed),
      money_evidence_ready: b($r.money_evidence_ready),
      runtime_canary_active: b($r.runtime_canary_active)
    },
    upstream: {
      path: $upstream_json_path,
      verdict: ($u.verdict // "missing"),
      upstream_configured: b($u.upstream_configured),
      ready_for_broad_provider_traffic: $broad_ready,
      real_probe_allowed: b($u.real_probe_allowed),
      real_probe_attempted: b($u.real_probe_attempted),
      boundary_rows_added: n($u.boundary_rows_added),
      observed_live_upstream_success: b($u.observed_live_upstream_success),
      observed_live_success_count: n($u.observed_live_success_count),
      observed_live_latest_timestamp: ($u.observed_live_latest_timestamp // ""),
      observed_live_latest_path: ($u.observed_live_latest_path // ""),
      observed_live_latest_status: n($u.observed_live_latest_status),
      observed_live_latest_provider: ($u.observed_live_latest_provider // "")
    },
    upstream_lab_smoke: {
      path: $upstream_smoke_json_path,
      verdict: ($us.verdict // "missing"),
      pass: (($us.verdict // "") == "NANDO_PROVIDER_BRIDGE_UPSTREAM_SMOKE_PASS"
        and n($us.failed_count) == 0
        and n($us.upstream_hit_count) >= 1
        and n($us.provider_boundary_event_count) >= 1),
      failed_count: n($us.failed_count),
      upstream_hit_count: n($us.upstream_hit_count),
      provider_boundary_event_count: n($us.provider_boundary_event_count),
      boundary: "lab proof only: fake upstream transport and provider-boundary capture; does not configure real upstream and does not unlock money claims"
    },
    upstream_onboard_smoke: {
      path: $upstream_onboard_smoke_json_path,
      verdict: ($uos.verdict // "missing"),
      pass: b($uos.pass),
      real_env_unchanged: b($uos.real_env_unchanged),
      upstream_hit_count: n($uos.upstream_hit_count),
      provider_boundary_event_count: n($uos.provider_boundary_event_count),
      provider_boundary_total_tokens: n($uos.provider_boundary_total_tokens),
      boundary: "lab proof only: configure-only upstream onboarding plus temporary bridge/readiness; does not mutate real server policy and does not unlock money claims"
    },
    activation_gate: {
      path: $activation_gate_json_path,
      activation_allowed: b($ag.activation_allowed),
      system_client_env_install_allowed: b($ag.system_client_env_install_allowed),
      blockers: ($ag.blockers // []),
      next_action: ($ag.next_action // "run_activation_gate"),
      boundary: "provider activation gate only: no provider secret printing, no local_accept mutation, no money claim unlock"
    },
    provider_activate_smoke: {
      path: $activate_smoke_json_path,
      verdict: ($as.verdict // "missing"),
      pass: b($as.pass),
      real_env_unchanged: b($as.real_env_unchanged),
      activation_allowed: b($as.activate.activation_allowed),
      upstream_hit_count: n($as.upstream_hit_count),
      provider_boundary_event_count: n($as.provider_boundary_event_count),
      provider_boundary_total_tokens: n($as.provider_boundary_total_tokens),
      boundary: "lab proof only: one-command provider activation wrapper against fake upstream; does not mutate real server policy and does not unlock money claims"
    },
    provider_evidence: {
      path: $evidence_json_path,
      market_money_claim_allowed: b($e.evidence_chain.market_money_claim_allowed),
      provider_billing_evidence_present: b($e.evidence_chain.provider_billing_evidence_present),
      blocker: ($e.blocker // "")
    },
    latency: {
      window_rows: $latency_window_rows,
      bridge_egress_count: n($bl.count),
      bridge_egress_p50_ms: n($bl.p50_ms),
      bridge_egress_p99_ms: n($bl.p99_ms),
      bridge_egress_max_ms: n($bl.max_ms),
      cpu_local_accept_count: n($cl.count),
      cpu_local_accept_p50_ms: n($cl.p50_ms),
      cpu_local_accept_p99_ms: n($cl.p99_ms),
      cpu_local_accept_max_ms: n($cl.max_ms),
      boundary: "observed elapsed time from recent bridge event rows; bridge egress includes upstream provider time, CPU local accept does not"
    },
    resources: {
      provider_bridge_rss_bytes: ([ $svc[] | select(.unit == "nando-provider-bridge.service") | n(.memory_current_bytes) ] | first // 0),
      live_tail_rss_bytes: ([ $svc[] | select(.unit == "nando-phase-center-live-tail.service") | n(.memory_current_bytes) ] | first // 0),
      appender_rss_bytes: ([ $svc[] | select(.unit == "nando-phase-center-appender.service") | n(.memory_current_bytes) ] | first // 0),
      serving_rss_bytes: ([ $svc[] | select(.unit == "nando-provider-bridge.service" or .unit == "nando-phase-center-live-tail.service" or .unit == "nando-phase-center-appender.service") | n(.memory_current_bytes) ] | add // 0),
      boundary: "live systemd MemoryCurrent snapshot; not a configured limit"
    },
    metrics: {
      path: $metrics_json_path,
      gateway_local_accept_events: n($m.gateway_local_accept_events),
      gateway_false_accepts: n($m.gateway_false_accepts),
      provider_bridge_local_accept_events: n($m.provider_bridge_local_accept_events),
      provider_bridge_tokens_saved_estimated: n($m.provider_bridge_tokens_saved_estimated),
      provider_bridge_false_accepts: n($m.provider_bridge_false_accepts),
      provider_bridge_v1_local_accept_events: n($m.provider_bridge_v1_local_accept_events),
      provider_bridge_v1_tokens_saved_estimated: n($m.provider_bridge_v1_tokens_saved_estimated),
      provider_bridge_v1_false_accepts: n($m.provider_bridge_v1_false_accepts),
      provider_bridge_v2_local_accept_events: n($m.provider_bridge_v2_local_accept_events),
      provider_bridge_v2_tokens_saved_estimated: n($m.provider_bridge_v2_tokens_saved_estimated),
      provider_bridge_v2_false_accepts: n($m.provider_bridge_v2_false_accepts),
      provider_bridge_v2_transition_runtime_events: n($m.provider_bridge_v2_transition_runtime_events),
      provider_bridge_v2_dogfood_local_accept_events: n($m.provider_bridge_v2_dogfood_local_accept_events),
      provider_bridge_v2_dogfood_tokens_saved_estimated: n($m.provider_bridge_v2_dogfood_tokens_saved_estimated),
      provider_bridge_v2_dogfood_false_accepts: n($m.provider_bridge_v2_dogfood_false_accepts),
      provider_bridge_v2_non_dogfood_local_accept_events: n($m.provider_bridge_v2_non_dogfood_local_accept_events),
      provider_bridge_v2_non_dogfood_tokens_saved_estimated: n($m.provider_bridge_v2_non_dogfood_tokens_saved_estimated),
      provider_bridge_v2_non_dogfood_false_accepts: n($m.provider_bridge_v2_non_dogfood_false_accepts),
      provider_bridge_boundary_window_rows: n($m.provider_bridge_boundary_window_rows)
    },
    services: $svc,
    summary: {
      canary_local_accept_ready: $canary_ready,
      broad_provider_traffic_ready: $broad_ready,
      money_claim_ready: b($v.market_money_claim_allowed),
      next_action: $next_action
    },
    boundary: "status report only: reads health and existing reports; no mining, scoring, provider calls, policy mutation, or secret printing"
  }' > "${STATUS_JSON}"

if [[ "${WRITE_ONLY}" != "1" ]]; then
  cat "${STATUS_JSON}"
else
  echo "${STATUS_JSON}"
fi
