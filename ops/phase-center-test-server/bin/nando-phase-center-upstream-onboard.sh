#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
shift || true

BASE_URL="${NANDO_ONBOARD_UPSTREAM_BASE_URL:-https://api.openai.com}"
PROVIDER="${NANDO_ONBOARD_PROVIDER:-openai}"
API_KEY_STDIN=0
ALLOW_REAL_PROBE=0
DRY_RUN=0
CONFIGURE_ONLY=0
PRINT_STATUS_ONLY=0

usage() {
  cat <<'EOF'
nando phase-center upstream onboarding

Usage:
  nando-phase-center-upstream-onboard.sh /etc/nando-wave/phase-center.env \
    --base-url https://api.openai.com --provider openai --api-key-stdin

Options:
  --base-url URL        upstream OpenAI-compatible base URL
  --provider NAME      provider label stored in server policy
  --api-key-stdin      read provider API key from stdin; key is never printed
  --allow-real-probe   allow one broad upstream readiness probe after setting
  --configure-only     set upstream policy and print config summary only
  --dry-run            use a temporary env copy and do not mutate server policy
  --status             print current upstream/status summary only

Examples:
  printf '%s\n' "$OPENAI_API_KEY" | sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-upstream-onboard.sh \
    /etc/nando-wave/phase-center.env --base-url https://api.openai.com --provider openai --api-key-stdin

  sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-upstream-onboard.sh \
    /etc/nando-wave/phase-center.env --status

Boundary:
  This configures upstream transport only. It does not enable market money
  claims, does not print provider secrets, and does not change phase-center
  scoring or local_accept policy.
EOF
}

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --base-url)
      BASE_URL="${2:-}"
      shift 2
      ;;
    --provider)
      PROVIDER="${2:-}"
      shift 2
      ;;
    --api-key-stdin)
      API_KEY_STDIN=1
      shift
      ;;
    --allow-real-probe)
      ALLOW_REAL_PROBE=1
      shift
      ;;
    --configure-only)
      CONFIGURE_ONLY=1
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --status)
      PRINT_STATUS_ONLY=1
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

if [[ ! -f "${ENV_FILE}" ]]; then
  echo "env file not found: ${ENV_FILE}" >&2
  exit 1
fi

if [[ -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "${ENV_FILE}"
  set +a
fi

OPS_DIR="${NANDO_PHASE_CENTER_OPS_DIR:-/opt/nando-wave/ops/phase-center-test-server}"
CONFIG_SCRIPT="${OPS_DIR}/bin/nando-provider-bridge-upstream-config.sh"
STATUS_SCRIPT="${OPS_DIR}/bin/nando-phase-center-status.sh"
READINESS_SCRIPT="${OPS_DIR}/bin/nando-provider-bridge-upstream-readiness.sh"
REFRESH_SCRIPT="${OPS_DIR}/bin/nando-phase-center-refresh-snapshots.sh"

for script in "${CONFIG_SCRIPT}" "${STATUS_SCRIPT}" "${READINESS_SCRIPT}"; do
  if [[ ! -x "${script}" ]]; then
    echo "required script missing or not executable: ${script}" >&2
    exit 1
  fi
done

work_env="${ENV_FILE}"
tmp_env=""
if [[ "${DRY_RUN}" == "1" ]]; then
  tmp_env="$(mktemp)"
  cp "${ENV_FILE}" "${tmp_env}"
  work_env="${tmp_env}"
fi
cleanup() {
  if [[ -n "${tmp_env}" ]]; then
    rm -f "${tmp_env}"
  fi
}
trap cleanup EXIT

status_summary() {
  "${CONFIG_SCRIPT}" "${work_env}" status | jq '{
    upstream_configured,
    api_key_present,
    api_key_value_printed,
    real_probe_allowed,
    readiness_verdict
  }'
  if [[ -x "${STATUS_SCRIPT}" ]]; then
    "${STATUS_SCRIPT}" "${work_env}" --refresh | jq '{summary, upstream, bridge}'
  fi
}

if [[ "${PRINT_STATUS_ONLY}" == "1" ]]; then
  status_summary
  exit 0
fi

if [[ -z "${BASE_URL}" ]]; then
  echo "--base-url is required" >&2
  exit 2
fi
if [[ "${API_KEY_STDIN}" != "1" ]]; then
  echo "--api-key-stdin is required; do not pass provider secrets as command arguments" >&2
  exit 2
fi

IFS= read -r api_key
if [[ -z "${api_key}" ]]; then
  echo "empty API key on stdin" >&2
  exit 2
fi

printf '%s\n' "${api_key}" | "${CONFIG_SCRIPT}" "${work_env}" set \
  --base-url "${BASE_URL}" \
  --provider "${PROVIDER}" \
  --api-key-stdin \
  --allow-real-probe "${ALLOW_REAL_PROBE}" >/dev/null
unset api_key

if [[ "${CONFIGURE_ONLY}" == "1" ]]; then
  config_json="$("${CONFIG_SCRIPT}" "${work_env}" status)"
  jq -n \
    --arg env_file "${ENV_FILE}" \
    --arg base_url "${BASE_URL%/}" \
    --arg provider "${PROVIDER}" \
    --argjson dry_run "$(if [[ "${DRY_RUN}" == "1" ]]; then echo true; else echo false; fi)" \
    --argjson configure_only true \
    --argjson allow_real_probe "$(if [[ "${ALLOW_REAL_PROBE}" == "1" ]]; then echo true; else echo false; fi)" \
    --argjson config "${config_json}" \
    '{
      report_kind: "nando_phase_center_upstream_onboard_v1",
      env_file: $env_file,
      dry_run: $dry_run,
      configure_only: $configure_only,
      provider: $provider,
      upstream_base_url: $base_url,
      api_key_value_printed: false,
      real_probe_allowed: $allow_real_probe,
      upstream_configured: ($config.upstream_configured // false),
      api_key_present: ($config.api_key_present // false),
      readiness_verdict: ($config.readiness_verdict // ""),
      broad_provider_traffic_ready: false,
      money_claim_ready: false,
      boundary: "upstream onboarding configure-only: no secret printing, no provider probe, no money claim unlock, no scoring/local_accept policy mutation"
    }'
  exit 0
fi

if [[ -x "${REFRESH_SCRIPT}" && "${DRY_RUN}" != "1" ]]; then
  "${REFRESH_SCRIPT}" "${work_env}" >/dev/null || true
fi
if [[ -x "${READINESS_SCRIPT}" ]]; then
  "${READINESS_SCRIPT}" "${work_env}" >/dev/null || true
fi

config_json="$("${CONFIG_SCRIPT}" "${work_env}" status)"
status_json="$("${STATUS_SCRIPT}" "${work_env}" --refresh)"

jq -n \
  --arg env_file "${ENV_FILE}" \
  --arg base_url "${BASE_URL%/}" \
  --arg provider "${PROVIDER}" \
  --argjson dry_run "$(if [[ "${DRY_RUN}" == "1" ]]; then echo true; else echo false; fi)" \
  --argjson allow_real_probe "$(if [[ "${ALLOW_REAL_PROBE}" == "1" ]]; then echo true; else echo false; fi)" \
  --argjson config "${config_json}" \
  --argjson status "${status_json}" \
  '{
    report_kind: "nando_phase_center_upstream_onboard_v1",
    env_file: $env_file,
    dry_run: $dry_run,
    provider: $provider,
    upstream_base_url: $base_url,
    api_key_value_printed: false,
    real_probe_allowed: $allow_real_probe,
    upstream_configured: ($config.upstream_configured // false),
    api_key_present: ($config.api_key_present // false),
    readiness_verdict: ($config.readiness_verdict // ""),
    status_summary: ($status.summary // {}),
    broad_provider_traffic_ready: ($status.summary.broad_provider_traffic_ready // false),
    money_claim_ready: ($status.summary.money_claim_ready // false),
    boundary: "upstream onboarding only: configures fail-open provider transport; no secret printing, no money claim unlock, no scoring/local_accept policy mutation"
  }'
