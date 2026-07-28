#!/usr/bin/env bash
set -euo pipefail

ACTION="${1:-}"
PROJECT_ROOT="${NANDO_REMOTE_PROJECT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
GATE_BIN="${NANDO_REMOTE_GATE_BIN:-/usr/local/bin/nando-live-transition-gate}"
AUTHORITY_ENV="${NANDO_REMOTE_AUTHORITY_ENV:-/etc/nando-wave/authority.env}"
RECEIPT="${NANDO_REMOTE_AUTHORITY_RECEIPT:-/var/lib/nando-wave/transition/remote-backend-authority-receipt.json}"
HOT_HEALTH="${NANDO_REMOTE_HOT_HEALTH:-http://127.0.0.1:18789/health}"
HOT_REFRESH="${NANDO_REMOTE_HOT_REFRESH:-http://127.0.0.1:18789/v2/runtime/refresh}"
CONTROL_HEALTH="${NANDO_REMOTE_CONTROL_HEALTH:-http://127.0.0.1:18788/health}"

usage() {
  cat <<'EOF'
Reconcile authority for the private-LAN Nando backend.

Usage:
  ops/remote-backend/reconcile-authority.sh enable
  ops/remote-backend/reconcile-authority.sh disable

Enable is fail-closed. It requires all non-deployment composite sections to
pass before writing authority.env, then requires a full PASS and effective
runtime authority. Any failure removes authority.env and restarts in shadow.
EOF
}

if [[ "${ACTION}" != "enable" && "${ACTION}" != "disable" ]]; then
  usage >&2
  exit 2
fi
if [[ ! -x "${GATE_BIN}" ]]; then
  echo "gate is not executable: ${GATE_BIN}" >&2
  exit 2
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT
expected_env="${tmp_dir}/authority.env"
preflight="${tmp_dir}/preflight.json"
final_gate="${tmp_dir}/final-gate.json"
health="${tmp_dir}/health.json"

printf '%s\n' \
  'NANDO_LOCAL_ACCEPT_ENABLED=1' \
  'NANDO_CLIENT_ALLOW_LOCAL_ACCEPT=1' \
  'NANDO_GATEWAY_CPU_ROUTE_READY=1' \
  > "${expected_env}"
chmod 0600 "${expected_env}"

restart_runtime() {
  sudo -n systemctl daemon-reload
  sudo -n systemctl restart nando-gateway-control.service
  sudo -n systemctl restart nando-transition-serving.service
  for _attempt in $(seq 1 50); do
    if curl -fsS --max-time 1 "${CONTROL_HEALTH}" >/dev/null 2>&1 \
      && curl -fsS --max-time 1 "${HOT_HEALTH}" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.1
  done
  return 1
}

run_gate() {
  "${GATE_BIN}" \
    --project-root "${PROJECT_ROOT}" \
    --status-mode
}

write_receipt() {
  local enabled="$1"
  local verdict="$2"
  python3 - \
    "${ACTION}" \
    "${PROJECT_ROOT}" \
    "${enabled}" \
    "${verdict}" \
    "$(date +%s)" \
    "${final_gate}" \
    "${health}" \
    "${tmp_dir}/receipt.json" <<'PY'
import json
import sys

action, project_root, enabled, verdict, generated_at, gate_path, health_path, output_path = (
    sys.argv[1:]
)
with open(gate_path, encoding="utf-8") as gate_file:
    gate = json.load(gate_file)
with open(health_path, encoding="utf-8") as health_file:
    health = json.load(health_file)

receipt = {
    "schema": "nando.remote-backend-authority-receipt.v1",
    "action": action,
    "project_root": project_root,
    "generated_at_unix": int(generated_at),
    "verdict": verdict,
    "authority_enabled": enabled == "true",
    "gate": gate,
    "health": health,
}
with open(output_path, "w", encoding="utf-8") as output:
    json.dump(receipt, output, sort_keys=True, separators=(",", ":"))
    output.write("\n")
PY
  install -m 0600 "${tmp_dir}/receipt.json" "${RECEIPT}"
}

rollback() {
  sudo -n rm -f "${AUTHORITY_ENV}"
  restart_runtime || true
  run_gate > "${final_gate}" || true
  curl -fsS --max-time 2 "${HOT_HEALTH}" > "${health}" || printf '{}\n' > "${health}"
  write_receipt false ROLLED_BACK
}

if [[ "${ACTION}" == "disable" ]]; then
  sudo -n rm -f "${AUTHORITY_ENV}"
  restart_runtime
  run_gate > "${final_gate}"
  curl -fsS --max-time 2 "${HOT_HEALTH}" > "${health}"
  if ! jq -e '
    .effective_local_accept_enabled == false
    and .response_effective_local_accept_enabled == false
  ' "${health}" >/dev/null; then
    echo "runtime authority remained enabled after disable" >&2
    exit 1
  fi
  write_receipt false PASS
  jq '{verdict,authority_enabled,gate_verdict:.gate.verdict}' "${RECEIPT}"
  exit 0
fi

if sudo -n test -e "${AUTHORITY_ENV}"; then
  if ! sudo -n cmp -s "${AUTHORITY_ENV}" "${expected_env}"; then
    echo "unexpected authority file already exists: ${AUTHORITY_ENV}" >&2
    exit 1
  fi
else
  run_gate > "${preflight}"
  if ! jq -e '
    .m3_complete == true
    and .sections.structural.verdict == "PASS"
    and .sections.wave_causal.verdict == "PASS"
    and .sections.runtime_admission.verdict == "PASS"
    and .sections.response_runtime.verdict == "PASS"
    and .sections.response_runtime.safety_pass == true
    and .sections.deployment.service_failures == 0
    and .sections.deployment.verdict == "VETO"
    and .sections.deployment.health_pass == false
    and (.required_actions | length) == 1
    and .required_actions[0].section == "deployment"
  ' "${preflight}" >/dev/null; then
    echo "authority preflight did not reach the expected shadow-only boundary" >&2
    jq '{verdict,m3_complete,required_actions,sections}' "${preflight}" >&2
    exit 1
  fi
  sudo -n install -o root -g root -m 0600 "${expected_env}" "${AUTHORITY_ENV}"
fi

if ! restart_runtime; then
  rollback
  echo "runtime failed to restart with authority candidate" >&2
  exit 1
fi
if ! run_gate > "${final_gate}"; then
  rollback
  echo "composite gate execution failed" >&2
  exit 1
fi
if ! jq -e '
  .verdict == "PASS"
  and .eligible_for_local_accept == true
  and .m3_complete == true
  and (.response_authority.packages | length) > 0
' "${final_gate}" >/dev/null; then
  rollback
  echo "composite gate did not grant authority" >&2
  exit 1
fi
if ! curl -fsS --max-time 3 -X POST "${HOT_REFRESH}" \
  | jq -e '.response_executor_ready == true' >/dev/null; then
  rollback
  echo "runtime rejected the freshly committed authority" >&2
  exit 1
fi

for _attempt in $(seq 1 50); do
  curl -fsS --max-time 2 "${HOT_HEALTH}" > "${health}"
  if jq -e '
    .effective_local_accept_enabled == true
    and .response_effective_local_accept_enabled == true
    and .response_runtime_revocation_state_valid == true
    and .response_runtime_revocations_unresolved_active == 0
  ' "${health}" >/dev/null; then
    write_receipt true PASS
    jq '{
      verdict,
      authority_enabled,
      gate_verdict:.gate.verdict,
      eligible_for_local_accept:.gate.eligible_for_local_accept,
      active_packages:(.gate.response_authority.packages | length)
    }' "${RECEIPT}"
    exit 0
  fi
  sleep 0.1
done

jq '{
  gate_verdict:.verdict,
  eligible_for_local_accept,
  authority_registry_revision:.response_authority.registry_revision,
  authority_packages:(.response_authority.packages | length),
  gate_build_sha256:.response_authority.gate_build_sha256,
  runtime_build_sha256:.response_authority.runtime_build_sha256
}' "${final_gate}" >&2
jq '{
  admission_verdict,
  admission_fresh,
  local_accept_enabled,
  client_allow_local_accept,
  effective_local_accept_enabled,
  response_effective_local_accept_enabled,
  response_cache_error,
  response_registry_revision,
  response_active_profiles,
  response_admission_expires_at_unix,
  response_admission_seconds_remaining
}' "${health}" >&2
rollback
echo "gate passed but effective runtime authority did not become ready" >&2
exit 1
