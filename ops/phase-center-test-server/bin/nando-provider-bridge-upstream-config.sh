#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
ACTION="${2:-status}"
shift 2 || true

usage() {
  cat <<'EOF'
nando provider bridge upstream config

Usage:
  nando-provider-bridge-upstream-config.sh /etc/nando-wave/phase-center.env status
  nando-provider-bridge-upstream-config.sh /etc/nando-wave/phase-center.env set --base-url URL --api-key-stdin [--api-prefix PATH] [--provider NAME] [--allow-real-probe 0|1]
  nando-provider-bridge-upstream-config.sh /etc/nando-wave/phase-center.env set-base --base-url URL [--api-prefix PATH] [--provider NAME] [--allow-real-probe 0|1]
  nando-provider-bridge-upstream-config.sh /etc/nando-wave/phase-center.env unset
  nando-provider-bridge-upstream-config.sh /etc/nando-wave/phase-center.env probe-on
  nando-provider-bridge-upstream-config.sh /etc/nando-wave/phase-center.env probe-off

Examples:
  printf '%s\n' "$OPENAI_API_KEY" | sudo nando-provider-bridge-upstream-config.sh /etc/nando-wave/phase-center.env set --base-url https://api.openai.com --api-key-stdin --provider openai
  sudo nando-provider-bridge-upstream-config.sh /etc/nando-wave/phase-center.env set-base --base-url https://api.openai.com --provider openai
  sudo nando-provider-bridge-upstream-config.sh /etc/nando-wave/phase-center.env probe-on

The API key is read from stdin and is never printed.
`set-base` stores only the upstream base URL. It is for client-authorization
forwarding: Codex supplies Authorization per request, while the server keeps no
provider API secret.
EOF
}

if [[ "${ENV_FILE}" == "--help" || "${ENV_FILE}" == "-h" || "${ACTION}" == "--help" || "${ACTION}" == "-h" ]]; then
  usage
  exit 0
fi

if [[ ! -f "${ENV_FILE}" ]]; then
  echo "env file not found: ${ENV_FILE}" >&2
  exit 1
fi

quote_env_value() {
  printf '%q' "$1"
}

set_kv_in_file() {
  local file="$1"
  local key="$2"
  local value="$3"
  local quoted
  if [[ -z "${value}" ]]; then
    quoted=""
  else
    quoted="$(quote_env_value "${value}")"
  fi
  if grep -qE "^${key}=" "${file}"; then
    sed -i -E "s#^${key}=.*#${key}=${quoted}#" "${file}"
  else
    printf '%s=%s\n' "${key}" "${quoted}" >> "${file}"
  fi
}

restart_bridge_if_available() {
  if [[ "${NANDO_PROVIDER_BRIDGE_RESTART_ON_CONFIG:-1}" != "1" ]]; then
    return 0
  fi
  if command -v systemctl >/dev/null 2>&1; then
    systemctl restart nando-provider-bridge.service >/dev/null 2>&1 || true
  fi
}

run_readiness_if_available() {
  local ops_dir
  ops_dir="$(grep -E '^NANDO_PHASE_CENTER_OPS_DIR=' "${ENV_FILE}" | tail -n 1 | cut -d= -f2- | sed "s#^'##;s#'\$##")"
  ops_dir="${ops_dir:-/opt/nando-wave/ops/phase-center-test-server}"
  if [[ -x "${ops_dir}/bin/nando-provider-bridge-upstream-readiness.sh" ]]; then
    "${ops_dir}/bin/nando-provider-bridge-upstream-readiness.sh" "${ENV_FILE}" >/dev/null || true
  fi
}

print_status() {
  local base_url api_prefix provider allow_probe has_key verdict report raw_key
  base_url="$(grep -E '^NANDO_PROVIDER_UPSTREAM_BASE_URL=' "${ENV_FILE}" | tail -n 1 | cut -d= -f2- | sed "s#^'##;s#'\$##")"
  api_prefix="$(grep -E '^NANDO_PROVIDER_UPSTREAM_API_PREFIX=' "${ENV_FILE}" | tail -n 1 | cut -d= -f2- | sed "s#^'##;s#'\$##")"
  provider="$(grep -E '^NANDO_PROVIDER_NAME=' "${ENV_FILE}" | tail -n 1 | cut -d= -f2- | sed "s#^'##;s#'\$##")"
  allow_probe="$(grep -E '^NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_ALLOW_REAL_CALL=' "${ENV_FILE}" | tail -n 1 | cut -d= -f2- | sed "s#^'##;s#'\$##")"
  has_key=false
  raw_key="$(grep -E '^NANDO_PROVIDER_UPSTREAM_API_KEY=' "${ENV_FILE}" | tail -n 1 | cut -d= -f2- | sed "s#^'##;s#'\$##")"
  if [[ -n "${raw_key}" ]]; then
    has_key=true
  fi
  report="$(grep -E '^NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_REPORT=' "${ENV_FILE}" | tail -n 1 | cut -d= -f2- | sed "s#^'##;s#'\$##")"
  verdict=""
  if [[ -n "${report}" && -s "${report}" ]]; then
    verdict="$(jq -r '.verdict // ""' "${report}" 2>/dev/null || true)"
  fi
  jq -n \
    --arg env_file "${ENV_FILE}" \
    --arg provider "${provider}" \
    --arg base_url "${base_url}" \
    --arg api_prefix "${api_prefix}" \
    --arg allow_probe "${allow_probe}" \
    --arg readiness_report "${report}" \
    --arg readiness_verdict "${verdict}" \
    --argjson upstream_configured "$(if [[ -n "${base_url}" && "${has_key}" == "true" ]]; then echo true; else echo false; fi)" \
    --argjson transport_configured "$(if [[ -n "${base_url}" ]]; then echo true; else echo false; fi)" \
    --argjson upstream_base_url_configured "$(if [[ -n "${base_url}" ]]; then echo true; else echo false; fi)" \
    --argjson api_key_present "${has_key}" \
    '{
      env_file: $env_file,
      provider: $provider,
      upstream_base_url: $base_url,
      upstream_api_prefix: $api_prefix,
      upstream_base_url_configured: $upstream_base_url_configured,
      api_key_present: $api_key_present,
      api_key_value_printed: false,
      upstream_configured: $transport_configured,
      upstream_server_api_key_configured: $upstream_configured,
      client_auth_forwarding_supported: true,
      real_probe_allowed: ($allow_probe == "1"),
      readiness_report: $readiness_report,
      readiness_verdict: $readiness_verdict
    }'
}

case "${ACTION}" in
  status)
    run_readiness_if_available
    print_status
    ;;
  set)
    base_url=""
    provider=""
    api_prefix=""
    api_prefix_set=false
    allow_probe=""
    api_key_from_stdin=false
    while [[ "$#" -gt 0 ]]; do
      case "$1" in
        --base-url)
          base_url="${2:-}"
          shift 2
          ;;
        --provider)
          provider="${2:-}"
          shift 2
          ;;
        --api-prefix)
          api_prefix="${2:-}"
          api_prefix_set=true
          shift 2
          ;;
        --allow-real-probe)
          allow_probe="${2:-}"
          shift 2
          ;;
        --api-key-stdin)
          api_key_from_stdin=true
          shift
          ;;
        *)
          echo "unknown set argument: $1" >&2
          usage >&2
          exit 2
          ;;
      esac
    done
    if [[ -z "${base_url}" ]]; then
      echo "--base-url is required" >&2
      exit 2
    fi
    if [[ "${api_key_from_stdin}" != "true" ]]; then
      echo "--api-key-stdin is required; do not pass provider secrets as command arguments" >&2
      exit 2
    fi
    IFS= read -r api_key
    if [[ -z "${api_key}" ]]; then
      echo "empty API key on stdin" >&2
      exit 2
    fi
    tmp="$(mktemp)"
    cp "${ENV_FILE}" "${tmp}"
    set_kv_in_file "${tmp}" "NANDO_PROVIDER_UPSTREAM_BASE_URL" "${base_url%/}"
    if [[ "${api_prefix_set}" == "true" ]]; then
      set_kv_in_file "${tmp}" "NANDO_PROVIDER_UPSTREAM_API_PREFIX" "${api_prefix%/}"
    fi
    set_kv_in_file "${tmp}" "NANDO_PROVIDER_UPSTREAM_API_KEY" "${api_key}"
    if [[ -n "${provider}" ]]; then
      set_kv_in_file "${tmp}" "NANDO_PROVIDER_NAME" "${provider}"
    fi
    if [[ -n "${allow_probe}" ]]; then
      case "${allow_probe}" in
        0|1) set_kv_in_file "${tmp}" "NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_ALLOW_REAL_CALL" "${allow_probe}" ;;
        *) echo "--allow-real-probe must be 0 or 1" >&2; rm -f "${tmp}"; exit 2 ;;
      esac
    fi
    install -m 0600 "${tmp}" "${ENV_FILE}"
    rm -f "${tmp}"
    restart_bridge_if_available
    run_readiness_if_available
    print_status
    ;;
  set-base)
    base_url=""
    provider=""
    api_prefix=""
    api_prefix_set=false
    allow_probe=""
    while [[ "$#" -gt 0 ]]; do
      case "$1" in
        --base-url)
          base_url="${2:-}"
          shift 2
          ;;
        --provider)
          provider="${2:-}"
          shift 2
          ;;
        --api-prefix)
          api_prefix="${2:-}"
          api_prefix_set=true
          shift 2
          ;;
        --allow-real-probe)
          allow_probe="${2:-}"
          shift 2
          ;;
        *)
          echo "unknown set-base argument: $1" >&2
          usage >&2
          exit 2
          ;;
      esac
    done
    if [[ -z "${base_url}" ]]; then
      echo "--base-url is required" >&2
      exit 2
    fi
    tmp="$(mktemp)"
    cp "${ENV_FILE}" "${tmp}"
    set_kv_in_file "${tmp}" "NANDO_PROVIDER_UPSTREAM_BASE_URL" "${base_url%/}"
    if [[ "${api_prefix_set}" == "true" ]]; then
      set_kv_in_file "${tmp}" "NANDO_PROVIDER_UPSTREAM_API_PREFIX" "${api_prefix%/}"
    fi
    set_kv_in_file "${tmp}" "NANDO_PROVIDER_UPSTREAM_API_KEY" ""
    if [[ -n "${provider}" ]]; then
      set_kv_in_file "${tmp}" "NANDO_PROVIDER_NAME" "${provider}"
    fi
    if [[ -n "${allow_probe}" ]]; then
      case "${allow_probe}" in
        0|1) set_kv_in_file "${tmp}" "NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_ALLOW_REAL_CALL" "${allow_probe}" ;;
        *) echo "--allow-real-probe must be 0 or 1" >&2; rm -f "${tmp}"; exit 2 ;;
      esac
    fi
    install -m 0600 "${tmp}" "${ENV_FILE}"
    rm -f "${tmp}"
    restart_bridge_if_available
    run_readiness_if_available
    print_status
    ;;
  unset)
    tmp="$(mktemp)"
    cp "${ENV_FILE}" "${tmp}"
    set_kv_in_file "${tmp}" "NANDO_PROVIDER_UPSTREAM_BASE_URL" ""
    set_kv_in_file "${tmp}" "NANDO_PROVIDER_UPSTREAM_API_KEY" ""
    set_kv_in_file "${tmp}" "NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_ALLOW_REAL_CALL" "0"
    install -m 0600 "${tmp}" "${ENV_FILE}"
    rm -f "${tmp}"
    restart_bridge_if_available
    run_readiness_if_available
    print_status
    ;;
  probe-on)
    tmp="$(mktemp)"
    cp "${ENV_FILE}" "${tmp}"
    set_kv_in_file "${tmp}" "NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_ALLOW_REAL_CALL" "1"
    install -m 0600 "${tmp}" "${ENV_FILE}"
    rm -f "${tmp}"
    run_readiness_if_available
    print_status
    ;;
  probe-off)
    tmp="$(mktemp)"
    cp "${ENV_FILE}" "${tmp}"
    set_kv_in_file "${tmp}" "NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_ALLOW_REAL_CALL" "0"
    install -m 0600 "${tmp}" "${ENV_FILE}"
    rm -f "${tmp}"
    run_readiness_if_available
    print_status
    ;;
  *)
    echo "unknown action: ${ACTION}" >&2
    usage >&2
    exit 2
    ;;
esac
