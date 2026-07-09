#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
if [[ -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "${ENV_FILE}"
  set +a
fi

OPS_DIR="${NANDO_PHASE_CENTER_OPS_DIR:-/opt/nando-wave/ops/phase-center-test-server}"
OUT_JSON="${NANDO_REFRESH_SNAPSHOTS_REPORT:-${NANDO_METRICS_DIR:-/var/lib/nando-wave/streaming/metrics}/nando-phase-center.refresh-snapshots.json}"

mkdir -p "$(dirname "${OUT_JSON}")"

run_step() {
  local name="$1"
  local script="$2"
  local started_ns ended_ns status output
  started_ns="$(date +%s%N)"
  set +e
  output="$("${script}" "${ENV_FILE}" 2>&1)"
  status=$?
  set -e
  ended_ns="$(date +%s%N)"
  jq -cn \
    --arg name "${name}" \
    --arg script "${script}" \
    --arg output "${output}" \
    --argjson status "${status}" \
    --argjson elapsed_ns "$((ended_ns - started_ns))" \
    '{
      name: $name,
      script: $script,
      status: $status,
      elapsed_ns: $elapsed_ns,
      output: $output,
      passed: ($status == 0)
    }'
  return "${status}"
}

rows="$(mktemp)"
trap 'rm -f "${rows}"' EXIT

overall_status=0
for spec in \
  "metrics:${OPS_DIR}/bin/nando-phase-center-metrics-snapshot.sh" \
  "provider_evidence:${OPS_DIR}/bin/nando-phase-center-provider-evidence-snapshot.sh" \
  "provider_bridge_upstream_readiness:${OPS_DIR}/bin/nando-provider-bridge-upstream-readiness.sh" \
  "readiness:${OPS_DIR}/bin/nando-phase-center-readiness-snapshot.sh" \
  "verify:${OPS_DIR}/bin/nando-phase-center-test-server-verify.sh" \
  "promotion:${OPS_DIR}/bin/nando-phase-center-local-accept-promotion-gate.sh"
do
  name="${spec%%:*}"
  script="${spec#*:}"
  if [[ ! -x "${script}" ]]; then
    jq -cn \
      --arg name "${name}" \
      --arg script "${script}" \
      '{
        name: $name,
        script: $script,
        status: 127,
        elapsed_ns: 0,
        output: "script_missing_or_not_executable",
        passed: false
      }' >> "${rows}"
    overall_status=1
    continue
  fi
  if ! run_step "${name}" "${script}" >> "${rows}"; then
    overall_status=1
  fi
done

jq -s \
  --arg env_file "${ENV_FILE}" \
  '{
    report_kind: "nando_phase_center_refresh_snapshots_v1",
    env_file: $env_file,
    steps: .,
    step_count: length,
    passed_count: ([.[] | select(.passed)] | length),
    failed_count: ([.[] | select(.passed | not)] | length),
    verdict: (if all(.[]; .passed) then
      "NANDO_PHASE_CENTER_REFRESH_SNAPSHOTS_PASS"
    else
      "NANDO_PHASE_CENTER_REFRESH_SNAPSHOTS_FAIL"
    end),
    boundary: "cold refresh only: updates reports used by status routes; not used in hot request scoring"
  }' "${rows}" > "${OUT_JSON}"

echo "${OUT_JSON}"
exit "${overall_status}"
