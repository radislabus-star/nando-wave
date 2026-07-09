#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
if [[ -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "${ENV_FILE}"
  set +a
fi

NANDO_BIN="${NANDO_BIN:-/opt/nando-wave/bin/nando-cli}"
OPS_DIR="${NANDO_PHASE_CENTER_OPS_DIR:-/opt/nando-wave/ops/phase-center-test-server}"
SYSTEMD_DIR="${NANDO_SYSTEMD_DIR:-/etc/systemd/system}"
VERIFY_REPORT="${NANDO_VERIFY_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.test-server-verify.json}"
LIVE_REPORT="${NANDO_LIVE_TAIL_REPORT:-/var/lib/nando-wave/streaming/nando-phase-live-miner-tail.report.json}"
METRICS_JSON="${NANDO_METRICS_SNAPSHOT_JSON:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.metrics.json}"
EVIDENCE_JSON="${NANDO_PROVIDER_EVIDENCE_SNAPSHOT_REPORT:-/var/lib/nando-wave/streaming/provider-evidence/provider-evidence-snapshot.report.json}"
READINESS_JSON="${NANDO_READINESS_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.readiness.json}"
UPSTREAM_READINESS_JSON="${NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-upstream-readiness.json}"
UPSTREAM_SMOKE_JSON="${NANDO_PROVIDER_BRIDGE_UPSTREAM_SMOKE_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-upstream-smoke.json}"
UPSTREAM_ONBOARD_SMOKE_JSON="${NANDO_PROVIDER_BRIDGE_UPSTREAM_ONBOARD_SMOKE_REPORT:-/var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-upstream-onboard-smoke.json}"

mkdir -p "$(dirname "${VERIFY_REPORT}")"

required_scripts=(
  "${OPS_DIR}/bin/nando-phase-center-metrics-snapshot.sh"
  "${OPS_DIR}/bin/nando-phase-center-provider-evidence-snapshot.sh"
  "${OPS_DIR}/bin/nando-phase-center-provider-export-contract-pack.sh"
  "${OPS_DIR}/bin/nando-phase-center-readiness-snapshot.sh"
  "${OPS_DIR}/bin/nando-phase-center-test-server-verify.sh"
  "${OPS_DIR}/bin/nando-phase-center-local-accept-promotion-gate.sh"
  "${OPS_DIR}/bin/nando-phase-center-refresh-snapshots.sh"
  "${OPS_DIR}/bin/nando-phase-center-status.sh"
  "${OPS_DIR}/bin/nando-phase-center-client-env.sh"
  "${OPS_DIR}/bin/nando-llm-gateway.sh"
  "${OPS_DIR}/bin/nando-llm-local-executor.sh"
  "${OPS_DIR}/bin/nando-provider-bridge.py"
  "${OPS_DIR}/bin/nando-provider-bridge-smoke.sh"
  "${OPS_DIR}/bin/nando-provider-bridge-v2-dogfood.sh"
  "${OPS_DIR}/bin/nando-provider-bridge-upstream-smoke.sh"
  "${OPS_DIR}/bin/nando-phase-center-upstream-onboard-smoke.sh"
  "${OPS_DIR}/bin/nando-provider-bridge-upstream-readiness.sh"
  "${OPS_DIR}/bin/nando-provider-bridge-upstream-config.sh"
  "${OPS_DIR}/bin/nando-phase-center-upstream-onboard.sh"
  "${OPS_DIR}/bin/nando-phase-center-gateway-canary-smoke.sh"
  "${OPS_DIR}/bin/nando-phase-center-policy-set.sh"
  "${OPS_DIR}/bin/nando-phase-center-provider-activation-gate.sh"
  "${OPS_DIR}/bin/nando-phase-center-provider-activate.sh"
  "${OPS_DIR}/bin/nando-phase-center-provider-activate-smoke.sh"
  "${OPS_DIR}/bin/nando-phase-center-provider-deactivate.sh"
)

required_units=(
  "nando-phase-center-appender.service"
  "nando-phase-center-live-tail.service"
  "nando-provider-bridge.service"
  "nando-phase-center-metrics-snapshot.service"
  "nando-phase-center-metrics-snapshot.timer"
  "nando-phase-center-provider-evidence-snapshot.service"
  "nando-phase-center-provider-evidence-snapshot.timer"
  "nando-phase-center-provider-export-contract-pack.service"
  "nando-phase-center-provider-export-contract-pack.timer"
  "nando-phase-center-readiness-snapshot.service"
  "nando-phase-center-readiness-snapshot.timer"
  "nando-phase-center-test-server-verify.service"
  "nando-phase-center-test-server-verify.timer"
  "nando-phase-center-status.service"
  "nando-phase-center-status.timer"
  "nando-phase-center-local-accept-promotion-gate.service"
  "nando-phase-center-local-accept-promotion-gate.timer"
  "nando-phase-center-provider-activation-gate.service"
  "nando-phase-center-provider-activation-gate.timer"
  "nando-phase-center-provider-export-watch.service"
  "nando-phase-center-provider-export-watch.timer"
)

missing_scripts=()
for script in "${required_scripts[@]}"; do
  if [[ ! -x "${script}" ]]; then
    missing_scripts+=("${script}")
  fi
done

missing_units=()
unit_paths=()
for unit in "${required_units[@]}"; do
  unit_path="${SYSTEMD_DIR}/${unit}"
  unit_paths+=("${unit_path}")
  if [[ ! -f "${unit_path}" ]]; then
    missing_units+=("${unit_path}")
  fi
done

systemd_verify_available=false
systemd_verify_pass=false
systemd_verify_checked=false
systemd_verify_log=""
if command -v systemd-analyze >/dev/null 2>&1; then
  systemd_verify_available=true
  if [[ "${#missing_units[@]}" -eq 0 ]]; then
    systemd_verify_checked=true
    set +e
    systemd_verify_log="$(systemd-analyze verify "${unit_paths[@]}" 2>&1)"
    status=$?
    set -e
    if [[ "${status}" -eq 0 ]]; then
      systemd_verify_pass=true
    fi
  fi
fi

binary_present=false
binary_executable=false
if [[ -f "${NANDO_BIN}" ]]; then
  binary_present=true
fi
if [[ -x "${NANDO_BIN}" ]]; then
  binary_executable=true
fi

env_file_present=false
env_file_mode=""
env_file_private=false
if [[ -f "${ENV_FILE}" ]]; then
  env_file_present=true
  env_file_mode="$(stat -c '%a' "${ENV_FILE}" 2>/dev/null || true)"
  if [[ "${env_file_mode}" == "600" ]]; then
    env_file_private=true
  fi
fi

live_report_present=false
metrics_present=false
evidence_present=false
readiness_present=false
upstream_readiness_present=false
upstream_lab_smoke_present=false
upstream_lab_smoke_pass=false
upstream_lab_smoke_verdict="upstream_lab_smoke_missing"
upstream_lab_smoke_failed_count=0
upstream_lab_smoke_hit_count=0
upstream_lab_smoke_boundary_count=0
upstream_onboard_smoke_present=false
upstream_onboard_smoke_pass=false
upstream_onboard_smoke_verdict="upstream_onboard_smoke_missing"
upstream_onboard_smoke_env_unchanged=false
[[ -s "${LIVE_REPORT}" ]] && live_report_present=true
[[ -s "${METRICS_JSON}" ]] && metrics_present=true
[[ -s "${EVIDENCE_JSON}" ]] && evidence_present=true
[[ -s "${READINESS_JSON}" ]] && readiness_present=true
[[ -s "${UPSTREAM_READINESS_JSON}" ]] && upstream_readiness_present=true
if [[ -s "${UPSTREAM_SMOKE_JSON}" ]] && jq -e . "${UPSTREAM_SMOKE_JSON}" >/dev/null 2>&1; then
  upstream_lab_smoke_present=true
  upstream_lab_smoke_verdict="$(jq -r '.verdict // "unknown"' "${UPSTREAM_SMOKE_JSON}")"
  upstream_lab_smoke_failed_count="$(jq -r '.failed_count // 0' "${UPSTREAM_SMOKE_JSON}")"
  upstream_lab_smoke_hit_count="$(jq -r '.upstream_hit_count // 0' "${UPSTREAM_SMOKE_JSON}")"
  upstream_lab_smoke_boundary_count="$(jq -r '.provider_boundary_event_count // 0' "${UPSTREAM_SMOKE_JSON}")"
  if [[ "${upstream_lab_smoke_verdict}" == "NANDO_PROVIDER_BRIDGE_UPSTREAM_SMOKE_PASS" && "${upstream_lab_smoke_failed_count}" == "0" && "${upstream_lab_smoke_hit_count}" -ge 1 && "${upstream_lab_smoke_boundary_count}" -ge 1 ]]; then
    upstream_lab_smoke_pass=true
  fi
fi
if [[ -s "${UPSTREAM_ONBOARD_SMOKE_JSON}" ]] && jq -e . "${UPSTREAM_ONBOARD_SMOKE_JSON}" >/dev/null 2>&1; then
  upstream_onboard_smoke_present=true
  upstream_onboard_smoke_verdict="$(jq -r '.verdict // "unknown"' "${UPSTREAM_ONBOARD_SMOKE_JSON}")"
  upstream_onboard_smoke_pass="$(jq -r '.pass // false' "${UPSTREAM_ONBOARD_SMOKE_JSON}")"
  upstream_onboard_smoke_env_unchanged="$(jq -r '.real_env_unchanged // false' "${UPSTREAM_ONBOARD_SMOKE_JSON}")"
fi

readiness_blocker="readiness_missing"
upstream_readiness_verdict="upstream_readiness_missing"
upstream_configured=false
upstream_ready_for_broad_provider_traffic=false
compression_claim_allowed=false
money_evidence_ready=false
market_money_claim_allowed=false
local_accept_promotion_allowed=false
local_accept_enabled=false
stable_false_accepts=0
stable_rows=0
unique_cpu_accepts=0
tokens_saved=0

if [[ "${readiness_present}" == "true" ]]; then
  readiness_blocker="$(jq -r '.blocker // "unknown"' "${READINESS_JSON}")"
  compression_claim_allowed="$(jq -r '.compression_claim_allowed // false' "${READINESS_JSON}")"
  money_evidence_ready="$(jq -r '.money_evidence_ready // false' "${READINESS_JSON}")"
  market_money_claim_allowed="$(jq -r '.market_money_claim_allowed // false' "${READINESS_JSON}")"
  local_accept_promotion_allowed="$(jq -r '.local_accept_promotion_allowed // false' "${READINESS_JSON}")"
  local_accept_enabled="$(jq -r '.local_accept_enabled // false' "${READINESS_JSON}")"
  stable_false_accepts="$(jq -r '.scorecard.false_accepts // 0' "${READINESS_JSON}")"
  stable_rows="$(jq -r '.scorecard.stable_rows // 0' "${READINESS_JSON}")"
  unique_cpu_accepts="$(jq -r '.scorecard.unique_cpu_accepts_over_exact_cache // 0' "${READINESS_JSON}")"
  tokens_saved="$(jq -r '.scorecard.tokens_saved // 0' "${READINESS_JSON}")"
fi
if [[ "${upstream_readiness_present}" == "true" ]]; then
  upstream_readiness_verdict="$(jq -r '.verdict // "unknown"' "${UPSTREAM_READINESS_JSON}")"
  upstream_configured="$(jq -r '.upstream_configured // false' "${UPSTREAM_READINESS_JSON}")"
  upstream_ready_for_broad_provider_traffic="$(jq -r '.ready_for_broad_provider_traffic // false' "${UPSTREAM_READINESS_JSON}")"
fi

install_ready=false
if [[ "${binary_executable}" == "true" && "${env_file_private}" == "true" && "${#missing_scripts[@]}" -eq 0 && "${#missing_units[@]}" -eq 0 ]]; then
  if [[ "${systemd_verify_checked}" == "false" || "${systemd_verify_pass}" == "true" ]]; then
    install_ready=true
  fi
fi

shadow_metrics_ready=false
if [[ "${readiness_present}" == "true" && "${compression_claim_allowed}" == "true" && "${stable_false_accepts}" == "0" ]]; then
  shadow_metrics_ready=true
fi

blockers=()
[[ "${binary_executable}" != "true" ]] && blockers+=("nando_cli_missing_or_not_executable")
[[ "${env_file_present}" != "true" ]] && blockers+=("env_file_missing")
[[ "${env_file_present}" == "true" && "${env_file_private}" != "true" ]] && blockers+=("env_file_not_private_0600")
[[ "${#missing_scripts[@]}" -gt 0 ]] && blockers+=("snapshot_scripts_missing")
[[ "${#missing_units[@]}" -gt 0 ]] && blockers+=("systemd_units_missing")
[[ "${systemd_verify_checked}" == "true" && "${systemd_verify_pass}" != "true" ]] && blockers+=("systemd_verify_failed")
[[ "${readiness_present}" != "true" ]] && blockers+=("readiness_snapshot_missing")
[[ "${upstream_readiness_present}" != "true" ]] && blockers+=("provider_bridge_upstream_readiness_missing")
[[ "${readiness_present}" == "true" && "${compression_claim_allowed}" != "true" ]] && blockers+=("${readiness_blocker}")
[[ "${market_money_claim_allowed}" != "true" ]] && blockers+=("market_money_claim_blocked")
[[ "${local_accept_promotion_allowed}" != "true" ]] && blockers+=("local_accept_promotion_blocked")

missing_scripts_json="[]"
if [[ "${#missing_scripts[@]}" -gt 0 ]]; then
  missing_scripts_json="$(printf '%s\n' "${missing_scripts[@]}" | jq -R . | jq -s .)"
fi

missing_units_json="[]"
if [[ "${#missing_units[@]}" -gt 0 ]]; then
  missing_units_json="$(printf '%s\n' "${missing_units[@]}" | jq -R . | jq -s .)"
fi

verdict="NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_WATCH"
if [[ "${install_ready}" == "true" && "${shadow_metrics_ready}" == "true" && "${market_money_claim_allowed}" == "true" ]]; then
  verdict="NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_MARKET_MONEY_READY"
elif [[ "${install_ready}" == "true" && "${shadow_metrics_ready}" == "true" ]]; then
  verdict="NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_COMPRESSION_WATCH_MONEY"
elif [[ "${install_ready}" == "true" ]]; then
  verdict="NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_INSTALL_WATCH_METRICS"
fi

jq -n \
  --arg env_file "${ENV_FILE}" \
  --arg nando_bin "${NANDO_BIN}" \
  --arg ops_dir "${OPS_DIR}" \
  --arg systemd_dir "${SYSTEMD_DIR}" \
  --arg live_report "${LIVE_REPORT}" \
  --arg metrics_json "${METRICS_JSON}" \
  --arg evidence_json "${EVIDENCE_JSON}" \
  --arg readiness_json "${READINESS_JSON}" \
  --arg upstream_readiness_json "${UPSTREAM_READINESS_JSON}" \
  --arg upstream_smoke_json "${UPSTREAM_SMOKE_JSON}" \
  --arg upstream_onboard_smoke_json "${UPSTREAM_ONBOARD_SMOKE_JSON}" \
  --arg readiness_blocker "${readiness_blocker}" \
  --arg upstream_readiness_verdict "${upstream_readiness_verdict}" \
  --arg upstream_lab_smoke_verdict "${upstream_lab_smoke_verdict}" \
  --arg upstream_onboard_smoke_verdict "${upstream_onboard_smoke_verdict}" \
  --arg verdict "${verdict}" \
  --arg systemd_verify_log "${systemd_verify_log}" \
  --arg env_file_mode "${env_file_mode}" \
  --argjson missing_scripts "${missing_scripts_json}" \
  --argjson missing_units "${missing_units_json}" \
  --argjson blockers "$(printf '%s\n' "${blockers[@]}" | jq -R . | jq -s 'map(select(length > 0))')" \
  --argjson binary_present "${binary_present}" \
  --argjson binary_executable "${binary_executable}" \
  --argjson env_file_present "${env_file_present}" \
  --argjson env_file_private "${env_file_private}" \
  --argjson systemd_verify_available "${systemd_verify_available}" \
  --argjson systemd_verify_checked "${systemd_verify_checked}" \
  --argjson systemd_verify_pass "${systemd_verify_pass}" \
  --argjson live_report_present "${live_report_present}" \
  --argjson metrics_present "${metrics_present}" \
  --argjson evidence_present "${evidence_present}" \
  --argjson readiness_present "${readiness_present}" \
  --argjson upstream_readiness_present "${upstream_readiness_present}" \
  --argjson upstream_lab_smoke_present "${upstream_lab_smoke_present}" \
  --argjson upstream_lab_smoke_pass "${upstream_lab_smoke_pass}" \
  --argjson upstream_lab_smoke_failed_count "${upstream_lab_smoke_failed_count}" \
  --argjson upstream_lab_smoke_hit_count "${upstream_lab_smoke_hit_count}" \
  --argjson upstream_lab_smoke_boundary_count "${upstream_lab_smoke_boundary_count}" \
  --argjson upstream_onboard_smoke_present "${upstream_onboard_smoke_present}" \
  --argjson upstream_onboard_smoke_pass "${upstream_onboard_smoke_pass}" \
  --argjson upstream_onboard_smoke_env_unchanged "${upstream_onboard_smoke_env_unchanged}" \
  --argjson upstream_configured "${upstream_configured}" \
  --argjson upstream_ready_for_broad_provider_traffic "${upstream_ready_for_broad_provider_traffic}" \
  --argjson install_ready "${install_ready}" \
  --argjson shadow_metrics_ready "${shadow_metrics_ready}" \
  --argjson compression_claim_allowed "${compression_claim_allowed}" \
  --argjson money_evidence_ready "${money_evidence_ready}" \
  --argjson market_money_claim_allowed "${market_money_claim_allowed}" \
  --argjson local_accept_promotion_allowed "${local_accept_promotion_allowed}" \
  --argjson local_accept_enabled "${local_accept_enabled}" \
  --argjson stable_false_accepts "${stable_false_accepts}" \
  --argjson stable_rows "${stable_rows}" \
  --argjson unique_cpu_accepts "${unique_cpu_accepts}" \
  --argjson tokens_saved "${tokens_saved}" \
  '{
    report_kind: "nando_phase_center_test_server_verify_v1",
    env_file: $env_file,
    nando_bin: $nando_bin,
    ops_dir: $ops_dir,
    systemd_dir: $systemd_dir,
    live_report: $live_report,
    metrics_json: $metrics_json,
    evidence_json: $evidence_json,
    readiness_json: $readiness_json,
    upstream_readiness_json: $upstream_readiness_json,
    upstream_smoke_json: $upstream_smoke_json,
    upstream_onboard_smoke_json: $upstream_onboard_smoke_json,
    binary_present: $binary_present,
    binary_executable: $binary_executable,
    env_file_present: $env_file_present,
    env_file_mode: $env_file_mode,
    env_file_private: $env_file_private,
    missing_scripts: $missing_scripts,
    missing_units: $missing_units,
    systemd_verify_available: $systemd_verify_available,
    systemd_verify_checked: $systemd_verify_checked,
    systemd_verify_pass: $systemd_verify_pass,
    systemd_verify_log: $systemd_verify_log,
    live_report_present: $live_report_present,
    metrics_present: $metrics_present,
    evidence_present: $evidence_present,
    readiness_present: $readiness_present,
    upstream_readiness_present: $upstream_readiness_present,
    upstream_lab_smoke: {
      present: $upstream_lab_smoke_present,
      pass: $upstream_lab_smoke_pass,
      verdict: $upstream_lab_smoke_verdict,
      failed_count: $upstream_lab_smoke_failed_count,
      upstream_hit_count: $upstream_lab_smoke_hit_count,
      provider_boundary_event_count: $upstream_lab_smoke_boundary_count,
      boundary: "lab proof only: fake upstream transport and provider-boundary capture; does not configure real upstream and does not unlock money claims"
    },
    upstream_onboard_smoke: {
      present: $upstream_onboard_smoke_present,
      pass: $upstream_onboard_smoke_pass,
      verdict: $upstream_onboard_smoke_verdict,
      real_env_unchanged: $upstream_onboard_smoke_env_unchanged,
      boundary: "lab proof only: configure-only upstream onboarding plus temporary bridge/readiness; does not mutate real server policy and does not unlock money claims"
    },
    upstream_readiness: {
      upstream_configured: $upstream_configured,
      ready_for_broad_provider_traffic: $upstream_ready_for_broad_provider_traffic,
      verdict: $upstream_readiness_verdict
    },
    install_ready: $install_ready,
    shadow_metrics_ready: $shadow_metrics_ready,
    compression_claim_allowed: $compression_claim_allowed,
    money_evidence_ready: $money_evidence_ready,
    market_money_claim_allowed: $market_money_claim_allowed,
    local_accept_promotion_allowed: $local_accept_promotion_allowed,
    readiness_blocker: $readiness_blocker,
    scorecard: {
      stable_rows: $stable_rows,
      unique_cpu_accepts_over_exact_cache: $unique_cpu_accepts,
      tokens_saved: $tokens_saved,
      false_accepts: $stable_false_accepts
    },
    blockers: $blockers,
    forbidden_flags: {
      nwrb_used: false,
      role_binding_backend_used: false,
      lookup_used: false,
      target_id_or_proof_rule_id_authority_used: false,
      concrete_x_lookup_used: false,
      manual_local_out_t_used: false,
      local_accept_without_verifier_used: false
    },
    local_accept_enabled: $local_accept_enabled,
    verdict: $verdict,
    boundary: "test-server verify only: checks installed files, unit syntax, and latest readiness reports; no mining, scoring, serving, provider estimation, promotion, or local_accept"
  }' > "${VERIFY_REPORT}"

echo "${VERIFY_REPORT}"
