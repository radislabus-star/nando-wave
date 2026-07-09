#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
MODE="${2:-}"

usage() {
  cat <<'EOF'
nando phase-center server policy

Usage:
  nando-phase-center-policy-set.sh /etc/nando-wave/phase-center.env shadow
  nando-phase-center-policy-set.sh /etc/nando-wave/phase-center.env canary-health
  nando-phase-center-policy-set.sh /etc/nando-wave/phase-center.env canary-verified
  nando-phase-center-policy-set.sh /etc/nando-wave/phase-center.env kill

Modes:
  shadow         Record telemetry, never emit local accepts.
  canary-health Enable only verifier-bound built-in health/status local routes.
  canary-verified Enable exact verifier-bound boxed routes only.
  kill           Keep provider fallback alive, disable all Nando offload attempts.
EOF
}

if [[ -z "${MODE}" || "${MODE}" == "--help" || "${MODE}" == "-h" ]]; then
  usage
  exit 0
fi

case "${MODE}" in
  shadow|canary-health|canary-verified|kill) ;;
  *)
    echo "unknown policy mode: ${MODE}" >&2
    usage >&2
    exit 2
    ;;
esac

if [[ ! -f "${ENV_FILE}" ]]; then
  echo "env file not found: ${ENV_FILE}" >&2
  exit 1
fi

tmp="$(mktemp)"
cp "${ENV_FILE}" "${tmp}"

set_kv() {
  local key="$1"
  local value="$2"
  if grep -qE "^${key}=" "${tmp}"; then
    sed -i -E "s#^${key}=.*#${key}=${value}#" "${tmp}"
  else
    printf '%s=%s\n' "${key}" "${value}" >> "${tmp}"
  fi
}

set_kv "NANDO_CLIENT_REQUIRE_VERIFIER" "1"
set_kv "NANDO_CLIENT_REQUIRE_FALSE_ACCEPTS_ZERO" "1"
set_kv "NANDO_GATEWAY_CAPTURE_RAW" "0"

case "${MODE}" in
  shadow)
    set_kv "NANDO_OFFLOAD" "1"
    set_kv "NANDO_LOCAL_ACCEPT_ENABLED" "0"
    set_kv "NANDO_CLIENT_ALLOW_LOCAL_ACCEPT" "0"
    set_kv "NANDO_CLIENT_SAFETY_POLICY" "shadow_only"
    set_kv "NANDO_CLIENT_TIER" "shadow"
    set_kv "NANDO_CLIENT_KILL_SWITCH" "0"
    ;;
  canary-health)
    set_kv "NANDO_OFFLOAD" "1"
    set_kv "NANDO_LOCAL_ACCEPT_ENABLED" "1"
    set_kv "NANDO_CLIENT_ALLOW_LOCAL_ACCEPT" "1"
    set_kv "NANDO_CLIENT_SAFETY_POLICY" "guarded_exact_health_only"
    set_kv "NANDO_CLIENT_TIER" "canary"
    set_kv "NANDO_CLIENT_KILL_SWITCH" "0"
    ;;
  canary-verified)
    set_kv "NANDO_OFFLOAD" "1"
    set_kv "NANDO_LOCAL_ACCEPT_ENABLED" "1"
    set_kv "NANDO_CLIENT_ALLOW_LOCAL_ACCEPT" "1"
    set_kv "NANDO_CLIENT_SAFETY_POLICY" "guarded_verified_routes"
    set_kv "NANDO_CLIENT_TIER" "canary_verified"
    set_kv "NANDO_CLIENT_KILL_SWITCH" "0"
    ;;
  kill)
    set_kv "NANDO_OFFLOAD" "0"
    set_kv "NANDO_LOCAL_ACCEPT_ENABLED" "0"
    set_kv "NANDO_CLIENT_ALLOW_LOCAL_ACCEPT" "0"
    set_kv "NANDO_CLIENT_SAFETY_POLICY" "kill_switch"
    set_kv "NANDO_CLIENT_TIER" "disabled"
    set_kv "NANDO_CLIENT_KILL_SWITCH" "1"
    ;;
esac

install -m 0600 "${tmp}" "${ENV_FILE}"
rm -f "${tmp}"

echo "policy=${MODE}"
grep -E '^(NANDO_OFFLOAD|NANDO_LOCAL_ACCEPT_ENABLED|NANDO_CLIENT_ALLOW_LOCAL_ACCEPT|NANDO_CLIENT_SAFETY_POLICY|NANDO_CLIENT_TIER|NANDO_CLIENT_KILL_SWITCH|NANDO_CLIENT_REQUIRE_VERIFIER|NANDO_CLIENT_REQUIRE_FALSE_ACCEPTS_ZERO)=' "${ENV_FILE}"
