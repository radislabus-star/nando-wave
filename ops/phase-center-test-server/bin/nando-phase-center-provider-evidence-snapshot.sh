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
LIVE_REPORT="${NANDO_LIVE_TAIL_REPORT:-/var/lib/nando-wave/streaming/nando-phase-live-miner-tail.report.json}"
EVIDENCE_DIR="${NANDO_PROVIDER_EVIDENCE_DIR:-/var/lib/nando-wave/streaming/provider-evidence}"
SNAPSHOT_REPORT="${NANDO_PROVIDER_EVIDENCE_SNAPSHOT_REPORT:-${EVIDENCE_DIR}/provider-evidence-snapshot.report.json}"
ACQUISITION_REPORT="${NANDO_PROVIDER_ACQUISITION_REPORT:-${EVIDENCE_DIR}/provider-export-acquisition.report.json}"
ACQUISITION_DIR="${NANDO_PROVIDER_ACQUISITION_PACK_DIR:-${EVIDENCE_DIR}/provider-export-acquisition-pack}"
EVIDENCE_CHAIN_REPORT="${NANDO_PROVIDER_EVIDENCE_CHAIN_REPORT:-${EVIDENCE_DIR}/provider-export-evidence-chain.report.json}"
EVIDENCE_CHAIN_DIR="${NANDO_PROVIDER_EVIDENCE_CHAIN_DIR:-${EVIDENCE_DIR}/provider-export-evidence-chain}"
PROVIDER_EXPORT_JSONL="${NANDO_PROVIDER_EXPORT_JSONL:-}"
PROVIDER_BRIDGE_BOUNDARY_JSONL="${NANDO_PROVIDER_BRIDGE_BOUNDARY_EVENTS_JSONL:-}"
PROVIDER_BOUNDARY_COVERAGE_REPORT="${NANDO_PROVIDER_BOUNDARY_CAPTURE_COVERAGE_REPORT:-${EVIDENCE_DIR}/provider-boundary-capture-coverage.report.json}"

mkdir -p "${EVIDENCE_DIR}" "${ACQUISITION_DIR}" "${EVIDENCE_CHAIN_DIR}"

PROVIDER_BOUNDARY_COVERAGE_STATUS="not_run"
PROVIDER_BOUNDARY_COVERAGE_VERDICT=""
PROVIDER_BOUNDARY_COVERAGE_PROVIDER_ROWS=0
PROVIDER_BOUNDARY_COVERAGE_COVERED_REQUESTS=0
PROVIDER_BOUNDARY_COVERAGE_MISSING_REQUESTS=0
PROVIDER_BOUNDARY_COVERAGE_COVERED_TOKENS=0
PROVIDER_BOUNDARY_COVERAGE_MISSING_TOKENS=0

write_snapshot() {
  local blocker="$1"
  local billing_request="${2:-}"
  local provider_export="${3:-}"
  jq -n \
    --arg live_report "${LIVE_REPORT}" \
    --arg blocker "${blocker}" \
    --arg billing_request_jsonl_path "${billing_request}" \
    --arg provider_export_jsonl_path "${provider_export}" \
    --arg acquisition_report_path "${ACQUISITION_REPORT}" \
    --arg acquisition_pack_dir "${ACQUISITION_DIR}" \
    --arg evidence_chain_report_path "${EVIDENCE_CHAIN_REPORT}" \
    --arg provider_bridge_boundary_jsonl_path "${PROVIDER_BRIDGE_BOUNDARY_JSONL}" \
    --arg provider_boundary_capture_coverage_report_path "${PROVIDER_BOUNDARY_COVERAGE_REPORT}" \
    --arg provider_boundary_capture_coverage_status "${PROVIDER_BOUNDARY_COVERAGE_STATUS}" \
    --arg provider_boundary_capture_coverage_verdict "${PROVIDER_BOUNDARY_COVERAGE_VERDICT}" \
    --argjson provider_boundary_capture_coverage_provider_rows "${PROVIDER_BOUNDARY_COVERAGE_PROVIDER_ROWS}" \
    --argjson provider_boundary_capture_coverage_covered_requests "${PROVIDER_BOUNDARY_COVERAGE_COVERED_REQUESTS}" \
    --argjson provider_boundary_capture_coverage_missing_requests "${PROVIDER_BOUNDARY_COVERAGE_MISSING_REQUESTS}" \
    --argjson provider_boundary_capture_coverage_covered_tokens "${PROVIDER_BOUNDARY_COVERAGE_COVERED_TOKENS}" \
    --argjson provider_boundary_capture_coverage_missing_tokens "${PROVIDER_BOUNDARY_COVERAGE_MISSING_TOKENS}" \
    '{
      report_kind: "nando_phase_center_provider_evidence_snapshot_v1",
      live_report: $live_report,
      billing_request_jsonl_path: $billing_request_jsonl_path,
      provider_export_jsonl_path: (if $provider_export_jsonl_path == "" then null else $provider_export_jsonl_path end),
      acquisition_report_path: $acquisition_report_path,
      acquisition_pack_dir: $acquisition_pack_dir,
      evidence_chain_report_path: $evidence_chain_report_path,
      provider_boundary_capture: {
        provider_bridge_boundary_jsonl_path: (if $provider_bridge_boundary_jsonl_path == "" then null else $provider_bridge_boundary_jsonl_path end),
        coverage_report_path: $provider_boundary_capture_coverage_report_path,
        status: $provider_boundary_capture_coverage_status,
        verdict: $provider_boundary_capture_coverage_verdict,
        provider_rows: $provider_boundary_capture_coverage_provider_rows,
        covered_capture_requests: $provider_boundary_capture_coverage_covered_requests,
        missing_capture_requests: $provider_boundary_capture_coverage_missing_requests,
        covered_tokens: $provider_boundary_capture_coverage_covered_tokens,
        missing_tokens: $provider_boundary_capture_coverage_missing_tokens,
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        boundary: "provider-boundary metadata coverage only; does not create billing evidence or money claims"
      },
      blocker: $blocker,
      local_accept_enabled: false,
      market_money_claim_allowed: false,
      boundary: "cold provider-evidence snapshot only: prepares acquisition/evidence reports; no scoring, serving, promotion, local_accept, or fabricated money evidence"
    }' > "${SNAPSHOT_REPORT}"
}

if [[ ! -s "${LIVE_REPORT}" ]]; then
  write_snapshot "live_report_missing"
  echo "${SNAPSHOT_REPORT}"
  exit 0
fi

BILLING_REQUEST_JSONL="$(jq -r '.future_shadow_billing_request_path // empty' "${LIVE_REPORT}")"
if [[ -z "${BILLING_REQUEST_JSONL}" || ! -s "${BILLING_REQUEST_JSONL}" ]]; then
  write_snapshot "future_shadow_billing_request_missing" "${BILLING_REQUEST_JSONL}"
  echo "${SNAPSHOT_REPORT}"
  exit 0
fi

"${NANDO_BIN}" phase-stream-provider-export-acquisition-pack-v1 \
  "${ACQUISITION_REPORT}" \
  "${ACQUISITION_DIR}" \
  "${BILLING_REQUEST_JSONL}" >/dev/null

CAPTURE_REQUEST_JSONL="$(jq -r '.provider_boundary_capture_request_jsonl_path // empty' "${ACQUISITION_REPORT}")"
if [[ -z "${CAPTURE_REQUEST_JSONL}" || ! -s "${CAPTURE_REQUEST_JSONL}" ]]; then
  write_snapshot "provider_boundary_capture_request_missing" "${BILLING_REQUEST_JSONL}" "${PROVIDER_EXPORT_JSONL}"
  echo "${SNAPSHOT_REPORT}"
  exit 0
fi

if [[ -n "${PROVIDER_BRIDGE_BOUNDARY_JSONL}" && -s "${PROVIDER_BRIDGE_BOUNDARY_JSONL}" ]]; then
  if "${NANDO_BIN}" phase-stream-provider-boundary-capture-coverage-gate-v1 \
    "${PROVIDER_BOUNDARY_COVERAGE_REPORT}" \
    "${CAPTURE_REQUEST_JSONL}" \
    --provider "${PROVIDER_BRIDGE_BOUNDARY_JSONL}" >/dev/null; then
    PROVIDER_BOUNDARY_COVERAGE_STATUS="ready"
    PROVIDER_BOUNDARY_COVERAGE_VERDICT="$(jq -r '.verdict // ""' "${PROVIDER_BOUNDARY_COVERAGE_REPORT}")"
    PROVIDER_BOUNDARY_COVERAGE_PROVIDER_ROWS="$(jq -r '.provider.rows // 0' "${PROVIDER_BOUNDARY_COVERAGE_REPORT}")"
    PROVIDER_BOUNDARY_COVERAGE_COVERED_REQUESTS="$(jq -r '.capture_requests.covered_capture_requests // 0' "${PROVIDER_BOUNDARY_COVERAGE_REPORT}")"
    PROVIDER_BOUNDARY_COVERAGE_MISSING_REQUESTS="$(jq -r '.capture_requests.missing_capture_requests // 0' "${PROVIDER_BOUNDARY_COVERAGE_REPORT}")"
    PROVIDER_BOUNDARY_COVERAGE_COVERED_TOKENS="$(jq -r '.capture_requests.covered_tokens // 0' "${PROVIDER_BOUNDARY_COVERAGE_REPORT}")"
    PROVIDER_BOUNDARY_COVERAGE_MISSING_TOKENS="$(jq -r '.capture_requests.missing_tokens // 0' "${PROVIDER_BOUNDARY_COVERAGE_REPORT}")"
  else
    PROVIDER_BOUNDARY_COVERAGE_STATUS="coverage_gate_failed"
  fi
else
  PROVIDER_BOUNDARY_COVERAGE_STATUS="no_provider_bridge_boundary_rows"
fi

LIVE_REPORT_EXPORT="$(jq -r '.provider_export_drop_path // empty' "${LIVE_REPORT}")"
if [[ -z "${PROVIDER_EXPORT_JSONL}" || ! -s "${PROVIDER_EXPORT_JSONL}" ]]; then
  if [[ -n "${LIVE_REPORT_EXPORT}" && -s "${LIVE_REPORT_EXPORT}" ]]; then
    PROVIDER_EXPORT_JSONL="${LIVE_REPORT_EXPORT}"
  fi
fi

if [[ -n "${PROVIDER_EXPORT_JSONL}" && -s "${PROVIDER_EXPORT_JSONL}" ]]; then
  "${NANDO_BIN}" phase-stream-provider-export-evidence-chain-v1 \
    "${EVIDENCE_CHAIN_REPORT}" \
    "${EVIDENCE_CHAIN_DIR}" \
    "${BILLING_REQUEST_JSONL}" \
    "${CAPTURE_REQUEST_JSONL}" \
    "${PROVIDER_EXPORT_JSONL}" >/dev/null
else
  "${NANDO_BIN}" phase-stream-provider-export-evidence-chain-v1 \
    "${EVIDENCE_CHAIN_REPORT}" \
    "${EVIDENCE_CHAIN_DIR}" \
    "${BILLING_REQUEST_JSONL}" \
    "${CAPTURE_REQUEST_JSONL}" >/dev/null
fi

MARKET_ALLOWED="$(jq -r '.market_money_claim_allowed // false' "${EVIDENCE_CHAIN_REPORT}")"
CHAIN_VERDICT="$(jq -r '.verdict // ""' "${EVIDENCE_CHAIN_REPORT}")"
BLOCKER="external_provider_export_missing"
if [[ "${MARKET_ALLOWED}" == "true" ]]; then
  BLOCKER="none"
elif [[ -n "${PROVIDER_EXPORT_JSONL}" && -s "${PROVIDER_EXPORT_JSONL}" ]]; then
  BLOCKER="provider_evidence_chain_blocked"
fi

jq -n \
  --arg live_report "${LIVE_REPORT}" \
  --arg billing_request_jsonl_path "${BILLING_REQUEST_JSONL}" \
  --arg provider_export_jsonl_path "${PROVIDER_EXPORT_JSONL}" \
  --arg acquisition_report_path "${ACQUISITION_REPORT}" \
  --arg acquisition_pack_dir "${ACQUISITION_DIR}" \
  --arg evidence_chain_report_path "${EVIDENCE_CHAIN_REPORT}" \
  --arg chain_verdict "${CHAIN_VERDICT}" \
  --arg blocker "${BLOCKER}" \
  --arg provider_bridge_boundary_jsonl_path "${PROVIDER_BRIDGE_BOUNDARY_JSONL}" \
  --arg provider_boundary_capture_coverage_report_path "${PROVIDER_BOUNDARY_COVERAGE_REPORT}" \
  --arg provider_boundary_capture_coverage_status "${PROVIDER_BOUNDARY_COVERAGE_STATUS}" \
  --arg provider_boundary_capture_coverage_verdict "${PROVIDER_BOUNDARY_COVERAGE_VERDICT}" \
  --argjson provider_boundary_capture_coverage_provider_rows "${PROVIDER_BOUNDARY_COVERAGE_PROVIDER_ROWS}" \
  --argjson provider_boundary_capture_coverage_covered_requests "${PROVIDER_BOUNDARY_COVERAGE_COVERED_REQUESTS}" \
  --argjson provider_boundary_capture_coverage_missing_requests "${PROVIDER_BOUNDARY_COVERAGE_MISSING_REQUESTS}" \
  --argjson provider_boundary_capture_coverage_covered_tokens "${PROVIDER_BOUNDARY_COVERAGE_COVERED_TOKENS}" \
  --argjson provider_boundary_capture_coverage_missing_tokens "${PROVIDER_BOUNDARY_COVERAGE_MISSING_TOKENS}" \
  --slurpfile acquisition "${ACQUISITION_REPORT}" \
  --slurpfile evidence "${EVIDENCE_CHAIN_REPORT}" \
  '{
    report_kind: "nando_phase_center_provider_evidence_snapshot_v1",
    live_report: $live_report,
    billing_request_jsonl_path: $billing_request_jsonl_path,
    provider_export_jsonl_path: (if $provider_export_jsonl_path == "" then null else $provider_export_jsonl_path end),
    acquisition_report_path: $acquisition_report_path,
    acquisition_pack_dir: $acquisition_pack_dir,
    evidence_chain_report_path: $evidence_chain_report_path,
    acquisition: {
      billing_request_rows: ($acquisition[0].billing_request_rows // 0),
      provider_boundary_capture_request_rows: ($acquisition[0].provider_boundary_capture_request_rows // 0),
      total_tokens_requiring_billing: ($acquisition[0].total_tokens_requiring_billing // 0),
      external_provider_collection_worklist_ready: ($acquisition[0].external_provider_collection_worklist_ready // false),
      provider_boundary_correlation_ready: ($acquisition[0].provider_boundary_correlation_ready // false),
      ready_for_external_provider_export: ($acquisition[0].ready_for_external_provider_export // false),
      verdict: ($acquisition[0].verdict // "")
    },
    evidence_chain: {
      provider_export_required: ($evidence[0].provider_export_required // false),
      provider_billing_evidence_present: ($evidence[0].provider_billing_evidence_present // false),
      external_evidence_chain_ready: ($evidence[0].external_evidence_chain_ready // false),
      market_money_claim_allowed: ($evidence[0].market_money_claim_allowed // false),
      verdict: $chain_verdict
    },
    provider_boundary_capture: {
      provider_bridge_boundary_jsonl_path: (if $provider_bridge_boundary_jsonl_path == "" then null else $provider_bridge_boundary_jsonl_path end),
      coverage_report_path: $provider_boundary_capture_coverage_report_path,
      status: $provider_boundary_capture_coverage_status,
      verdict: $provider_boundary_capture_coverage_verdict,
      provider_rows: $provider_boundary_capture_coverage_provider_rows,
      covered_capture_requests: $provider_boundary_capture_coverage_covered_requests,
      missing_capture_requests: $provider_boundary_capture_coverage_missing_requests,
      covered_tokens: $provider_boundary_capture_coverage_covered_tokens,
      missing_tokens: $provider_boundary_capture_coverage_missing_tokens,
      provider_capture_complete: ($provider_boundary_capture_coverage_missing_requests == 0 and $provider_boundary_capture_coverage_covered_requests > 0),
      local_accept_enabled: false,
      market_money_claim_allowed: false,
      boundary: "provider-boundary metadata coverage only; does not create billing evidence or money claims"
    },
    blocker: $blocker,
    local_accept_enabled: false,
    market_money_claim_allowed: ($evidence[0].market_money_claim_allowed // false),
    forbidden_flags: {
      nwrb_used: false,
      role_binding_backend_used: false,
      lookup_used: false,
      target_id_or_proof_rule_id_authority_used: false,
      concrete_x_lookup_used: false,
      manual_local_out_t_used: false,
      local_accept_without_verifier_used: false
    },
    boundary: "cold provider-evidence snapshot only: prepares acquisition/evidence reports; no scoring, serving, promotion, local_accept, or fabricated money evidence"
  }' > "${SNAPSHOT_REPORT}"

echo "${SNAPSHOT_REPORT}"
