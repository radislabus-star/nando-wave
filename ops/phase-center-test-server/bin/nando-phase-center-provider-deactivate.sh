#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
shift || true

REMOVE_SYSTEM_CLIENT_ENV=0
STATUS_ONLY=0
WRITE_ONLY=0

usage() {
  cat <<'EOF'
nando phase-center provider deactivate

Usage:
  sudo nando-phase-center-provider-deactivate.sh /etc/nando-wave/phase-center.env
  sudo nando-phase-center-provider-deactivate.sh /etc/nando-wave/phase-center.env --remove-system-client-env

Purpose:
  Roll broad provider traffic back to canary-only mode. This unsets upstream
  provider transport and refreshes activation/status reports. It does not
  disable verifier-bound local canary routes.

Options:
  --remove-system-client-env  remove /etc/profile.d/nando-wave-client.sh
  --status                    print current rollback/status summary only
  --write-only                write report path only

Boundary:
  This is a rollback command. It does not print provider secrets, does not
  mutate local_accept, and does not unlock money claims.
EOF
}

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --remove-system-client-env)
      REMOVE_SYSTEM_CLIENT_ENV=1
      shift
      ;;
    --status)
      STATUS_ONLY=1
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
OUT_JSON="${NANDO_PROVIDER_BRIDGE_DEACTIVATE_REPORT:-${NANDO_METRICS_DIR:-/var/lib/nando-wave/streaming/metrics}/nando-phase-center.provider-deactivate.json}"
CONFIG_SCRIPT="${OPS_DIR}/bin/nando-provider-bridge-upstream-config.sh"
ACTIVATION_SCRIPT="${OPS_DIR}/bin/nando-phase-center-provider-activation-gate.sh"
STATUS_SCRIPT="${OPS_DIR}/bin/nando-phase-center-status.sh"
CLIENT_ENV_SCRIPT="${OPS_DIR}/bin/nando-phase-center-client-env.sh"
SYSTEM_CLIENT_ENV_PATH="${NANDO_SYSTEM_CLIENT_ENV_PATH:-/etc/profile.d/nando-wave-client.sh}"

mkdir -p "$(dirname "${OUT_JSON}")"

for script in "${CONFIG_SCRIPT}" "${ACTIVATION_SCRIPT}" "${STATUS_SCRIPT}" "${CLIENT_ENV_SCRIPT}"; do
  if [[ ! -x "${script}" ]]; then
    jq -n \
      --arg env_file "${ENV_FILE}" \
      --arg missing_script "${script}" \
      '{
        report_kind: "nando_phase_center_provider_deactivate_v1",
        env_file: $env_file,
        rollback_applied: false,
        broad_provider_traffic_ready: false,
        blockers: ["required_script_missing"],
        missing_script: $missing_script,
        provider_secret_printed: false,
        market_money_claim_allowed: false,
        boundary: "provider deactivate failed before checks; no provider call, no secret printing"
      }' > "${OUT_JSON}"
    if [[ "${WRITE_ONLY}" != "1" ]]; then
      cat "${OUT_JSON}"
    else
      echo "${OUT_JSON}"
    fi
    exit 1
  fi
done

write_report() {
  local rollback_applied="$1"
  local system_client_env_removed="$2"
  local config_json activation_json status_json client_json
  config_json="$("${CONFIG_SCRIPT}" "${ENV_FILE}" status)"
  activation_json="$("${ACTIVATION_SCRIPT}" "${ENV_FILE}")"
  status_json="$("${STATUS_SCRIPT}" "${ENV_FILE}" --refresh)"
  client_json="$("${CLIENT_ENV_SCRIPT}" "${ENV_FILE}" status)"
  jq -n \
    --arg env_file "${ENV_FILE}" \
    --arg report_path "${OUT_JSON}" \
    --arg system_client_env_path "${SYSTEM_CLIENT_ENV_PATH}" \
    --argjson rollback_applied "${rollback_applied}" \
    --argjson system_client_env_removed "${system_client_env_removed}" \
    --argjson config "${config_json}" \
    --argjson activation "${activation_json}" \
    --argjson status "${status_json}" \
    --argjson client "${client_json}" \
    '{
      report_kind: "nando_phase_center_provider_deactivate_v1",
      env_file: $env_file,
      report_path: $report_path,
      rollback_applied: $rollback_applied,
      upstream_configured: ($config.upstream_configured // false),
      api_key_present: ($config.api_key_present // false),
      api_key_value_printed: false,
      provider_secret_printed: false,
      activation_allowed: ($activation.activation_allowed // false),
      broad_provider_traffic_ready: ($status.summary.broad_provider_traffic_ready // false),
      canary_local_accept_ready: ($status.summary.canary_local_accept_ready // false),
      false_accepts: ($status.scorecard.false_accepts // 0),
      system_client_env_path: $system_client_env_path,
      system_client_env_removed: $system_client_env_removed,
      system_client_env_installed: ($client.system_env_installed // false),
      blockers: ($activation.blockers // []),
      next_action: (if (($config.upstream_configured // false) == false)
        and (($status.summary.broad_provider_traffic_ready // false) == false)
        then "canary_only_mode"
        else "check_provider_deactivate"
      end),
      market_money_claim_allowed: false,
      boundary: "provider deactivate report: canary-only rollback; no provider secret printing, no local_accept mutation, no money claim unlock"
    }' > "${OUT_JSON}"
}

if [[ "${STATUS_ONLY}" == "1" ]]; then
  write_report false false
  if [[ "${WRITE_ONLY}" != "1" ]]; then
    cat "${OUT_JSON}"
  else
    echo "${OUT_JSON}"
  fi
  exit 0
fi

"${CONFIG_SCRIPT}" "${ENV_FILE}" unset >/dev/null

system_client_env_removed=false
if [[ "${REMOVE_SYSTEM_CLIENT_ENV}" == "1" && -e "${SYSTEM_CLIENT_ENV_PATH}" ]]; then
  rm -f "${SYSTEM_CLIENT_ENV_PATH}"
  system_client_env_removed=true
fi

write_report true "${system_client_env_removed}"

if [[ "${WRITE_ONLY}" != "1" ]]; then
  cat "${OUT_JSON}"
else
  echo "${OUT_JSON}"
fi
