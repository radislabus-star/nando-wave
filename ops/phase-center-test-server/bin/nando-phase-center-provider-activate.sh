#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
shift || true

BASE_URL="${NANDO_ONBOARD_UPSTREAM_BASE_URL:-https://api.openai.com}"
PROVIDER="${NANDO_ONBOARD_PROVIDER:-openai}"
API_KEY_STDIN=0
ALLOW_REAL_PROBE=0
INSTALL_SYSTEM_CLIENT_ENV=0
PRINT_STATUS_ONLY=0
WRITE_ONLY=0

usage() {
  cat <<'EOF'
nando phase-center provider activate

Usage:
  printf '%s\n' "$OPENAI_API_KEY" | sudo nando-phase-center-provider-activate.sh \
    /etc/nando-wave/phase-center.env \
    --base-url https://api.openai.com \
    --provider openai \
    --api-key-stdin \
    --allow-real-probe \
    --install-system-client-env

Options:
  --base-url URL               upstream OpenAI-compatible base URL
  --provider NAME              provider label stored in server policy
  --api-key-stdin              read provider API key from stdin; key is never printed
  --allow-real-probe           run one reviewed broad upstream readiness probe
  --install-system-client-env  install sanitized /etc/profile.d client env only after activation PASS
  --status                     print current activation/status summary only
  --write-only                 write report path only

Boundary:
  This is an operator command for server activation. It does not accept API keys
  as arguments, never prints provider secrets, does not mutate local_accept, and
  does not unlock money claims.
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
    --install-system-client-env)
      INSTALL_SYSTEM_CLIENT_ENV=1
      shift
      ;;
    --status)
      PRINT_STATUS_ONLY=1
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
OUT_JSON="${NANDO_PROVIDER_BRIDGE_ACTIVATE_REPORT:-${NANDO_METRICS_DIR:-/var/lib/nando-wave/streaming/metrics}/nando-phase-center.provider-activate.json}"
ONBOARD_SCRIPT="${OPS_DIR}/bin/nando-phase-center-upstream-onboard.sh"
ACTIVATION_SCRIPT="${OPS_DIR}/bin/nando-phase-center-provider-activation-gate.sh"
STATUS_SCRIPT="${OPS_DIR}/bin/nando-phase-center-status.sh"
CLIENT_ENV_SCRIPT="${OPS_DIR}/bin/nando-phase-center-client-env.sh"

mkdir -p "$(dirname "${OUT_JSON}")"

for script in "${ONBOARD_SCRIPT}" "${ACTIVATION_SCRIPT}" "${STATUS_SCRIPT}" "${CLIENT_ENV_SCRIPT}"; do
  if [[ ! -x "${script}" ]]; then
    jq -n \
      --arg env_file "${ENV_FILE}" \
      --arg missing_script "${script}" \
      '{
        report_kind: "nando_phase_center_provider_activate_v1",
        env_file: $env_file,
        activation_allowed: false,
        system_client_env_installed: false,
        blockers: ["required_script_missing"],
        missing_script: $missing_script,
        api_key_value_printed: false,
        provider_secret_printed: false,
        market_money_claim_allowed: false,
        boundary: "provider activation failed before checks; no provider call, no policy mutation, no secret printing"
      }' > "${OUT_JSON}"
    if [[ "${WRITE_ONLY}" != "1" ]]; then
      cat "${OUT_JSON}"
    else
      echo "${OUT_JSON}"
    fi
    exit 1
  fi
done

write_status_report() {
  local status_json activation_json client_json
  status_json="$("${STATUS_SCRIPT}" "${ENV_FILE}" --refresh)"
  activation_json="$("${ACTIVATION_SCRIPT}" "${ENV_FILE}")"
  client_json="$("${CLIENT_ENV_SCRIPT}" "${ENV_FILE}" status)"
  jq -n \
    --arg env_file "${ENV_FILE}" \
    --arg report_path "${OUT_JSON}" \
    --argjson status "${status_json}" \
    --argjson activation "${activation_json}" \
    --argjson client "${client_json}" \
    '{
      report_kind: "nando_phase_center_provider_activate_v1",
      env_file: $env_file,
      report_path: $report_path,
      status_only: true,
      activation_allowed: ($activation.activation_allowed // false),
      system_client_env_install_allowed: ($activation.system_client_env_install_allowed // false),
      system_client_env_installed: ($client.system_env_installed // false),
      blockers: ($activation.blockers // []),
      next_action: ($activation.next_action // "unknown"),
      upstream_configured: ($activation.upstream_configured // false),
      upstream_ready_for_broad_provider_traffic: ($activation.upstream_ready_for_broad_provider_traffic // false),
      false_accepts: ($activation.false_accepts // 0),
      api_key_value_printed: false,
      provider_secret_printed: false,
      market_money_claim_allowed: false,
      status_summary: ($status.summary // {}),
      boundary: "status-only provider activation report: no provider secret, no provider config write, no local_accept mutation, no money claim unlock"
    }' > "${OUT_JSON}"
}

if [[ "${PRINT_STATUS_ONLY}" == "1" ]]; then
  write_status_report
  if [[ "${WRITE_ONLY}" != "1" ]]; then
    cat "${OUT_JSON}"
  else
    echo "${OUT_JSON}"
  fi
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
if [[ "${ALLOW_REAL_PROBE}" != "1" ]]; then
  echo "--allow-real-probe is required for broad provider activation" >&2
  exit 2
fi

IFS= read -r api_key
if [[ -z "${api_key}" ]]; then
  echo "empty API key on stdin" >&2
  exit 2
fi

onboard_args=(
  "${ENV_FILE}"
  --base-url "${BASE_URL}"
  --provider "${PROVIDER}"
  --api-key-stdin
  --allow-real-probe
)

onboard_json="$(printf '%s\n' "${api_key}" | "${ONBOARD_SCRIPT}" "${onboard_args[@]}")"
unset api_key
activation_json="$("${ACTIVATION_SCRIPT}" "${ENV_FILE}" --allow-real-probe)"

system_client_env_installed=false
client_install_exit_code=0
client_json="$("${CLIENT_ENV_SCRIPT}" "${ENV_FILE}" status)"
if jq -e '.activation_allowed == true' <<<"${activation_json}" >/dev/null 2>&1 && [[ "${INSTALL_SYSTEM_CLIENT_ENV}" == "1" ]]; then
  client_install_err="$(mktemp)"
  set +e
  client_json="$("${CLIENT_ENV_SCRIPT}" "${ENV_FILE}" install-system 2>"${client_install_err}")"
  client_install_exit_code=$?
  set -e
  if [[ "${client_install_exit_code}" -eq 0 ]] && jq -e '.system_env_installed == true' <<<"${client_json}" >/dev/null 2>&1; then
    system_client_env_installed=true
  else
    client_json="$("${CLIENT_ENV_SCRIPT}" "${ENV_FILE}" status)"
  fi
  rm -f "${client_install_err}"
fi

status_json="$("${STATUS_SCRIPT}" "${ENV_FILE}" --refresh)"

jq -n \
  --arg env_file "${ENV_FILE}" \
  --arg report_path "${OUT_JSON}" \
  --arg provider "${PROVIDER}" \
  --arg base_url "${BASE_URL%/}" \
  --argjson onboard "${onboard_json}" \
  --argjson activation "${activation_json}" \
  --argjson status "${status_json}" \
  --argjson client "${client_json}" \
  --argjson install_requested "$(if [[ "${INSTALL_SYSTEM_CLIENT_ENV}" == "1" ]]; then echo true; else echo false; fi)" \
  --argjson client_install_exit_code "${client_install_exit_code}" \
  --argjson system_client_env_installed "${system_client_env_installed}" \
  '{
    report_kind: "nando_phase_center_provider_activate_v1",
    env_file: $env_file,
    report_path: $report_path,
    provider: $provider,
    upstream_base_url: $base_url,
    api_key_value_printed: false,
    provider_secret_printed: false,
    provider_secret_stored_in_client_env: ($client.provider_secret_stored // false),
    upstream_configured: ($activation.upstream_configured // false),
    upstream_ready_for_broad_provider_traffic: ($activation.upstream_ready_for_broad_provider_traffic // false),
    activation_allowed: ($activation.activation_allowed // false),
    system_client_env_install_requested: $install_requested,
    system_client_env_install_allowed: ($activation.system_client_env_install_allowed // false),
    system_client_env_installed: $system_client_env_installed,
    client_install_exit_code: $client_install_exit_code,
    blockers: ($activation.blockers // []),
    next_action: (if ($activation.activation_allowed // false) and ($install_requested | not) then "install_system_client_env_after_review"
      elif ($activation.activation_allowed // false) and $system_client_env_installed then "start_broad_shadow"
      else ($activation.next_action // "fix_blockers") end),
    false_accepts: ($activation.false_accepts // 0),
    unique_cpu_accepts_over_exact_cache: ($activation.unique_cpu_accepts_over_exact_cache // 0),
    tokens_saved: ($activation.tokens_saved // 0),
    readiness_verdict: ($onboard.readiness_verdict // ""),
    status_summary: ($status.summary // {}),
    market_money_claim_allowed: false,
    boundary: "server activation command: configures upstream from stdin, runs one reviewed real readiness probe, may install sanitized client env only after activation PASS; no secret printing, no local_accept mutation, no money claim unlock"
  }' > "${OUT_JSON}"

if [[ "${WRITE_ONLY}" != "1" ]]; then
  cat "${OUT_JSON}"
else
  echo "${OUT_JSON}"
fi
