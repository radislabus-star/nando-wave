#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
shift || true

ALLOW_REAL_PROBE=0
WRITE_ONLY=0

usage() {
  cat <<'EOF'
nando phase-center provider activation gate

Usage:
  nando-phase-center-provider-activation-gate.sh /etc/nando-wave/phase-center.env
  nando-phase-center-provider-activation-gate.sh /etc/nando-wave/phase-center.env --allow-real-probe

Purpose:
  Produce one server-side activation verdict for broad provider traffic after
  upstream onboarding. It does not accept provider secrets, does not print
  provider secrets, and does not mutate local_accept or client policy.

Boundary:
  --allow-real-probe permits the existing upstream readiness script to make one
  broad upstream probe. Without that flag this command is report-only.
EOF
}

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --allow-real-probe)
      ALLOW_REAL_PROBE=1
      shift
      ;;
    --write-only)
      WRITE_ONLY=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
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
OUT_JSON="${NANDO_PROVIDER_BRIDGE_ACTIVATION_GATE_REPORT:-${NANDO_METRICS_DIR:-/var/lib/nando-wave/streaming/metrics}/nando-phase-center.provider-activation-gate.json}"
READINESS_SCRIPT="${OPS_DIR}/bin/nando-provider-bridge-upstream-readiness.sh"
VERIFY_SCRIPT="${OPS_DIR}/bin/nando-phase-center-test-server-verify.sh"
STATUS_SCRIPT="${OPS_DIR}/bin/nando-phase-center-status.sh"
CLIENT_ENV_SCRIPT="${OPS_DIR}/bin/nando-phase-center-client-env.sh"

mkdir -p "$(dirname "${OUT_JSON}")"

for script in "${READINESS_SCRIPT}" "${VERIFY_SCRIPT}" "${STATUS_SCRIPT}" "${CLIENT_ENV_SCRIPT}"; do
  if [[ ! -x "${script}" ]]; then
    jq -n \
      --arg env_file "${ENV_FILE}" \
      --arg missing_script "${script}" \
      '{
        report_kind: "nando_phase_center_provider_activation_gate_v1",
        env_file: $env_file,
        activation_allowed: false,
        system_client_env_install_allowed: false,
        blockers: ["required_script_missing"],
        missing_script: $missing_script,
        provider_secret_printed: false,
        market_money_claim_allowed: false,
        boundary: "activation gate failed before checks; no provider call, no policy mutation, no secret printing"
      }' > "${OUT_JSON}"
    if [[ "${WRITE_ONLY}" != "1" ]]; then
      cat "${OUT_JSON}"
    else
      echo "${OUT_JSON}"
    fi
    exit 1
  fi
done

if [[ "${ALLOW_REAL_PROBE}" == "1" ]]; then
  NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_ALLOW_REAL_CALL=1 "${READINESS_SCRIPT}" "${ENV_FILE}" >/dev/null || true
else
  "${READINESS_SCRIPT}" "${ENV_FILE}" >/dev/null || true
fi

"${VERIFY_SCRIPT}" "${ENV_FILE}" >/dev/null || true
status_json="$("${STATUS_SCRIPT}" "${ENV_FILE}" --refresh)"
client_json="$("${CLIENT_ENV_SCRIPT}" "${ENV_FILE}" status)"

jq -n \
  --arg env_file "${ENV_FILE}" \
  --arg report_path "${OUT_JSON}" \
  --argjson allow_real_probe "$(if [[ "${ALLOW_REAL_PROBE}" == "1" ]]; then echo true; else echo false; fi)" \
  --argjson status "${status_json}" \
  --argjson client "${client_json}" '
  def b($v): if $v then true else false end;
  def n($v): $v // 0;
  ($status.summary // {}) as $summary |
  ($status.verify // {}) as $verify |
  ($status.upstream // {}) as $upstream |
  ($status.scorecard // {}) as $score |
  ($status.bridge // {}) as $bridge |
  ($client // {}) as $client_status |
  [
    (if (b($bridge.health_ok) | not) then "bridge_health_down" else empty end),
    (if (b($verify.install_ready) | not) then "install_not_ready" else empty end),
    (if (b($summary.canary_local_accept_ready) | not) then "canary_local_accept_not_ready" else empty end),
    (if n($score.false_accepts) != 0 then "false_accepts_nonzero" else empty end),
    (if (b($upstream.upstream_configured) | not) then "upstream_not_configured" else empty end),
    (if (b($upstream.ready_for_broad_provider_traffic) | not) then "broad_provider_traffic_not_ready" else empty end),
    (if (b($client_status.default_bridge_allowed) | not) then "client_default_bridge_blocked" else empty end),
    (if b($client_status.provider_secret_printed) then "provider_secret_printed" else empty end)
  ] as $blockers |
  ($blockers | length == 0) as $activation_allowed |
  {
    report_kind: "nando_phase_center_provider_activation_gate_v1",
    env_file: $env_file,
    report_path: $report_path,
    generated_utc: (now | todateiso8601),
    allow_real_probe: $allow_real_probe,
    activation_allowed: $activation_allowed,
    system_client_env_install_allowed: $activation_allowed,
    blockers: $blockers,
    canary_local_accept_ready: b($summary.canary_local_accept_ready),
    broad_provider_traffic_ready: b($summary.broad_provider_traffic_ready),
    money_claim_ready: b($summary.money_claim_ready),
    bridge_health_ok: b($bridge.health_ok),
    install_ready: b($verify.install_ready),
    upstream_configured: b($upstream.upstream_configured),
    upstream_verdict: ($upstream.verdict // "missing"),
    upstream_ready_for_broad_provider_traffic: b($upstream.ready_for_broad_provider_traffic),
    upstream_real_probe_attempted: b($upstream.real_probe_attempted),
    upstream_boundary_rows_added: n($upstream.boundary_rows_added),
    false_accepts: n($score.false_accepts),
    unique_cpu_accepts_over_exact_cache: n($score.unique_cpu_accepts_over_exact_cache),
    tokens_saved: n($score.tokens_saved),
    client_default_bridge_allowed: b($client_status.default_bridge_allowed),
    client_default_bridge_blocker: ($client_status.default_bridge_blocker // "unknown"),
    provider_secret_printed: b($client_status.provider_secret_printed),
    provider_secret_stored_in_client_env: b($client_status.provider_secret_stored),
    market_money_claim_allowed: false,
    next_action: (if $activation_allowed then "install_system_client_env_or_start_broad_shadow"
      elif (($blockers | index("upstream_not_configured")) != null) then "configure_provider_upstream"
      elif (($blockers | index("broad_provider_traffic_not_ready")) != null) then "run_activation_gate_with_real_probe_after_review"
      elif (($blockers | index("false_accepts_nonzero")) != null) then "quarantine_bad_profile"
      else "fix_blockers" end),
    boundary: "activation gate only: proves broad provider traffic readiness for default client env; no provider secret printing, no local_accept mutation, no money claim unlock"
  }' > "${OUT_JSON}"

if [[ "${WRITE_ONLY}" != "1" ]]; then
  cat "${OUT_JSON}"
else
  echo "${OUT_JSON}"
fi
