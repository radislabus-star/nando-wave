#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
if [[ -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "${ENV_FILE}"
  set +a
fi

ACQUISITION_REPORT="${NANDO_PROVIDER_ACQUISITION_REPORT:-/var/lib/nando-wave/streaming/provider-evidence/provider-export-acquisition.report.json}"
SNAPSHOT_REPORT="${NANDO_PROVIDER_EVIDENCE_SNAPSHOT_REPORT:-/var/lib/nando-wave/streaming/provider-evidence/provider-evidence-snapshot.report.json}"
CONTRACT_DIR="${NANDO_PROVIDER_EXPORT_CONTRACT_DIR:-/var/lib/nando-wave/streaming/provider-evidence/provider-export-contract}"
CONTRACT_REPORT="${NANDO_PROVIDER_EXPORT_CONTRACT_REPORT:-/var/lib/nando-wave/streaming/provider-evidence/provider-export-contract.report.json}"
PROVIDER_EXPORT_DROP_DIR="${NANDO_PROVIDER_EXPORT_DROP_DIR:-/var/lib/nando-wave/provider-export-drop}"

mkdir -p "${CONTRACT_DIR}" "$(dirname "${CONTRACT_REPORT}")"

write_blocked_report() {
  local blocker="$1"
  jq -n \
    --arg acquisition_report "${ACQUISITION_REPORT}" \
    --arg snapshot_report "${SNAPSHOT_REPORT}" \
    --arg contract_dir "${CONTRACT_DIR}" \
    --arg blocker "${blocker}" \
    '{
      report_kind: "nando_phase_center_provider_export_contract_pack_v1",
      acquisition_report: $acquisition_report,
      provider_evidence_snapshot_report: $snapshot_report,
      contract_dir: $contract_dir,
      contract_ready: false,
      blocker: $blocker,
      local_accept_enabled: false,
      market_money_claim_allowed: false,
      boundary: "provider export contract pack only: no billing evidence, no money claim, no serving mutation, no local_accept"
    }' > "${CONTRACT_REPORT}"
}

if [[ ! -s "${ACQUISITION_REPORT}" ]]; then
  write_blocked_report "acquisition_report_missing"
  echo "${CONTRACT_REPORT}"
  exit 0
fi

if [[ ! -s "${SNAPSHOT_REPORT}" ]]; then
  write_blocked_report "provider_evidence_snapshot_missing"
  echo "${CONTRACT_REPORT}"
  exit 0
fi

SNAPSHOT_BLOCKER="$(jq -r '.blocker // "unknown"' "${SNAPSHOT_REPORT}")"
SNAPSHOT_ACQUISITION_REPORT="$(jq -r '.acquisition_report_path // empty' "${SNAPSHOT_REPORT}")"
SNAPSHOT_BILLING_ROWS="$(jq -r '.acquisition.billing_request_rows // 0' "${SNAPSHOT_REPORT}")"
if [[ "${SNAPSHOT_BLOCKER}" != "external_provider_export_missing" ]]; then
  write_blocked_report "${SNAPSHOT_BLOCKER}"
  echo "${CONTRACT_REPORT}"
  exit 0
fi
if [[ -n "${SNAPSHOT_ACQUISITION_REPORT}" && "${SNAPSHOT_ACQUISITION_REPORT}" != "${ACQUISITION_REPORT}" ]]; then
  write_blocked_report "acquisition_snapshot_path_mismatch"
  echo "${CONTRACT_REPORT}"
  exit 0
fi
if [[ "${SNAPSHOT_BILLING_ROWS}" == "0" ]]; then
  write_blocked_report "snapshot_billing_rows_missing"
  echo "${CONTRACT_REPORT}"
  exit 0
fi

MANIFEST_JSONL="$(jq -r '.acquisition_manifest_jsonl_path // empty' "${ACQUISITION_REPORT}")"
CAPTURE_JSONL="$(jq -r '.provider_boundary_capture_request_jsonl_path // empty' "${ACQUISITION_REPORT}")"
REQUIRED_COLUMNS_CSV="$(jq -r '.required_columns_csv_path // empty' "${ACQUISITION_REPORT}")"
REQUIRED_SCHEMA_JSON="$(jq -r '.required_schema_json_path // empty' "${ACQUISITION_REPORT}")"
BILLING_ROWS="$(jq -r '.billing_request_rows // 0' "${ACQUISITION_REPORT}")"
TOKENS_REQUIRING_BILLING="$(jq -r '.total_tokens_requiring_billing // 0' "${ACQUISITION_REPORT}")"
REQUEST_FILE_FINGERPRINT64="$(jq -r '.request_file_fingerprint64 // 0' "${ACQUISITION_REPORT}")"

if [[ -z "${MANIFEST_JSONL}" || ! -s "${MANIFEST_JSONL}" ]]; then
  write_blocked_report "acquisition_manifest_missing"
  echo "${CONTRACT_REPORT}"
  exit 0
fi
if [[ -z "${CAPTURE_JSONL}" || ! -s "${CAPTURE_JSONL}" ]]; then
  write_blocked_report "provider_boundary_capture_request_missing"
  echo "${CONTRACT_REPORT}"
  exit 0
fi
if [[ -z "${REQUIRED_COLUMNS_CSV}" || ! -s "${REQUIRED_COLUMNS_CSV}" ]]; then
  write_blocked_report "required_columns_missing"
  echo "${CONTRACT_REPORT}"
  exit 0
fi
if [[ -z "${REQUIRED_SCHEMA_JSON}" || ! -s "${REQUIRED_SCHEMA_JSON}" ]]; then
  write_blocked_report "required_schema_missing"
  echo "${CONTRACT_REPORT}"
  exit 0
fi

cp "${REQUIRED_COLUMNS_CSV}" "${CONTRACT_DIR}/provider-export-required-columns.csv"
cp "${REQUIRED_SCHEMA_JSON}" "${CONTRACT_DIR}/provider-export-required-schema.json"
head -5 "${MANIFEST_JSONL}" > "${CONTRACT_DIR}/provider-export-acquisition.sample.jsonl"
head -5 "${CAPTURE_JSONL}" > "${CONTRACT_DIR}/provider-boundary-capture-request.sample.jsonl"

jq -c '
  {
    schema_version: "external_provider_export_template_v1",
    template_do_not_submit: true,
    request_file_fingerprint64: (.request_file_fingerprint64 // 0),
    billing_evidence_id: "FILL_REAL_BILLING_EVIDENCE_ID",
    billing_source: "FILL_REAL_PROVIDER_BILLING_EXPORT",
    provider: "FILL_REAL_PROVIDER_NAME",
    provider_cost_microusd: 0,
    provider_total_tokens: 0,
    request_fingerprint: (.request_fingerprint // null),
    exact_cache_key: (.exact_cache_key // null),
    trace_id: (.trace_id // null),
    match_keys: (.join_keys_to_echo_in_provider_export // []),
    provider_request_id: "FILL_REAL_PROVIDER_REQUEST_ID_OR_USE_ANOTHER_ALLOWED_PROVIDER_ID_FIELD",
    provider_response_id: null,
    provider_trace_id: null,
    external_provider_request_id: null,
    openai_request_id: null,
    anthropic_request_id: null,
    custom_id: null,
    local_accept_enabled: false,
    market_money_claim_allowed: false,
    boundary: "template only: replace placeholders with real external provider billing data before placing export in the provider drop dir"
  }
' "${MANIFEST_JSONL}" | sed -n '1,5p' > "${CONTRACT_DIR}/provider-export.template.jsonl"

README_PATH="${CONTRACT_DIR}/README_PROVIDER_EXPORT.md"
cat > "${README_PATH}" <<EOF
# Nando Provider Export Contract

This folder tells an external billing/export process how to produce the real
provider JSONL needed to unblock Nando money evidence.

Current worklist:

\`\`\`text
billing_request_rows: ${BILLING_ROWS}
tokens_requiring_billing: ${TOKENS_REQUIRING_BILLING}
request_file_fingerprint64: ${REQUEST_FILE_FINGERPRINT64}
\`\`\`

Write the real export JSONL into:

\`\`\`text
${PROVIDER_EXPORT_DROP_DIR}
\`\`\`

The export must include, per covered request:

\`\`\`text
billing_evidence_id
billing_source
provider
provider_cost_microusd or provider_cost_usd
provider_total_tokens or input_tokens/output_tokens/cached_input_tokens
one join key: request_fingerprint / exact_cache_key / trace_id / match_keys
one real provider id: provider_request_id / provider_response_id / provider_trace_id / external_provider_request_id / openai_request_id / anthropic_request_id / custom_id
\`\`\`

Use these files:

\`\`\`text
provider-export-required-columns.csv
provider-export-required-schema.json
provider-export-acquisition.sample.jsonl
provider-boundary-capture-request.sample.jsonl
provider-export.template.jsonl
\`\`\`

Do not submit the template as evidence. Rows with placeholders, zero provider
cost/tokens, synthetic/internal/request-only sources, or no real provider id
must remain blocked by the evidence gate.

This contract does not enable local_accept and does not claim money.
EOF

CONTRACT_READY=true
jq -n \
  --arg acquisition_report "${ACQUISITION_REPORT}" \
  --arg snapshot_report "${SNAPSHOT_REPORT}" \
  --arg contract_dir "${CONTRACT_DIR}" \
  --arg provider_export_drop_dir "${PROVIDER_EXPORT_DROP_DIR}" \
  --arg readme_path "${README_PATH}" \
  --arg required_columns_csv "${CONTRACT_DIR}/provider-export-required-columns.csv" \
  --arg required_schema_json "${CONTRACT_DIR}/provider-export-required-schema.json" \
  --arg manifest_sample_jsonl "${CONTRACT_DIR}/provider-export-acquisition.sample.jsonl" \
  --arg capture_sample_jsonl "${CONTRACT_DIR}/provider-boundary-capture-request.sample.jsonl" \
  --arg template_jsonl "${CONTRACT_DIR}/provider-export.template.jsonl" \
  --argjson billing_request_rows "${BILLING_ROWS}" \
  --argjson total_tokens_requiring_billing "${TOKENS_REQUIRING_BILLING}" \
  --argjson request_file_fingerprint64 "${REQUEST_FILE_FINGERPRINT64}" \
  --argjson contract_ready "${CONTRACT_READY}" \
  '{
    report_kind: "nando_phase_center_provider_export_contract_pack_v1",
    acquisition_report: $acquisition_report,
    provider_evidence_snapshot_report: $snapshot_report,
    contract_dir: $contract_dir,
    provider_export_drop_dir: $provider_export_drop_dir,
    readme_path: $readme_path,
    required_columns_csv: $required_columns_csv,
    required_schema_json: $required_schema_json,
    manifest_sample_jsonl: $manifest_sample_jsonl,
    capture_sample_jsonl: $capture_sample_jsonl,
    template_jsonl: $template_jsonl,
    billing_request_rows: $billing_request_rows,
    total_tokens_requiring_billing: $total_tokens_requiring_billing,
    request_file_fingerprint64: $request_file_fingerprint64,
    contract_ready: $contract_ready,
    provider_export_required: true,
    local_accept_enabled: false,
    market_money_claim_allowed: false,
    forbidden_flags: {
      nwrb_used: false,
      role_binding_backend_used: false,
      lookup_used: false,
      target_id_or_proof_rule_id_authority_used: false,
      concrete_x_lookup_used: false,
      manual_local_out_t_used: false,
      local_accept_without_verifier_used: false
    },
    blocker: "external_provider_export_missing",
    boundary: "provider export contract pack only: prepares external billing/export instructions; no billing evidence, no money claim, no serving mutation, no local_accept"
  }' > "${CONTRACT_REPORT}"

echo "${CONTRACT_REPORT}"
