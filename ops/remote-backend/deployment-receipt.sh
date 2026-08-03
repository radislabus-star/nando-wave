#!/usr/bin/env bash
set -euo pipefail

RECEIPT_ROOT="${NANDO_DEPLOY_RECEIPT_ROOT-/var/lib/nando-wave/deployments}"
USE_SUDO="${NANDO_DEPLOY_USE_SUDO-1}"
BINARIES="${NANDO_DEPLOY_RECEIPT_BINARIES-/opt/nando-wave/bin/nando-transition-serving /opt/nando-wave/bin/nando-operator-certification-authority /opt/nando-wave/bin/nando-operator-cleanup-verifier /opt/nando-wave/bin/nando-gateway-control /opt/nando-wave/bin/nando-response-admission}"
UNITS="${NANDO_DEPLOY_RECEIPT_UNITS-nando-transition-serving.service nando-response-learning.service nando-operator-certification-authority.service nando-operator-cleanup-verifier@.service nando-gateway-control.service nando-transport-gateway.service nando-live-transition-gate.service nando-live-transition-gate.path nando-live-transition-gate.timer nando-response-admission.service nando-response-admission.path nando-response-admission.timer}"
STATE_ROOTS="${NANDO_DEPLOY_RECEIPT_STATE_ROOTS-/var/lib/nando-wave/transition/multi-source-live-v2}"
CONFIG_PATHS="${NANDO_DEPLOY_RECEIPT_CONFIGS-/etc/nando-gateway/nginx.conf /etc/nando-wave/roles/response-learning.env}"
RUNTIME_ENDPOINTS="${NANDO_DEPLOY_RECEIPT_ENDPOINTS-hot_health=http://127.0.0.1:18789/health cold_health=http://127.0.0.1:18790/health control_health=http://127.0.0.1:18788/health edge_health=http://192.168.3.94:8787/health acquisition=http://127.0.0.1:18790/v2/multi-source/ms3-linked-frame-acquisition generation=http://127.0.0.1:18790/v2/multi-source/ms3-generation-registry ms4=http://127.0.0.1:18790/v2/multi-source/ms4-closed-loop}"
HOT_UNIT="${NANDO_DEPLOY_HOT_UNIT-nando-transition-serving.service}"
NGINX_UNIT="${NANDO_DEPLOY_NGINX_UNIT-nando-transport-gateway.service}"
ALLOW_HOT_RESTART="${NANDO_DEPLOY_ALLOW_HOT_RESTART-0}"

if [[ "${ALLOW_HOT_RESTART}" != "0" && "${ALLOW_HOT_RESTART}" != "1" ]]; then
  printf 'NANDO_DEPLOY_ALLOW_HOT_RESTART must be 0 or 1\n' >&2
  exit 2
fi
if [[ "${ALLOW_HOT_RESTART}" == "1" ]]; then
  ALLOW_HOT_RESTART_JSON=true
else
  ALLOW_HOT_RESTART_JSON=false
fi

usage() {
  cat <<'EOF'
Create a durable, content-addressed Nando deployment receipt.

Usage:
  deployment-receipt.sh prepare \
    --source-dir /clean/release/checkout \
    --rollback-commit <currently-deployed-commit>

  deployment-receipt.sh finalize \
    --source-dir /clean/release/checkout \
    --deployment-dir /var/lib/nando-wave/deployments/<id>
EOF
}

as_root() {
  if [[ "${USE_SUDO}" == "1" ]]; then
    sudo -n "$@"
  else
    "$@"
  fi
}

sha_file() {
  as_root sha256sum "$1" | awk '{print $1}'
}

unit_pid() {
  local unit="$1"
  [[ -n "${unit}" ]] || { printf '0\n'; return; }
  systemctl show -p MainPID --value "${unit}" 2>/dev/null || printf '0\n'
}

copy_for_rollback() {
  local source="$1"
  local deployment_dir="$2"
  local destination="${deployment_dir}/rollback${source}"
  as_root install -d -m 0700 "$(dirname "${destination}")"
  as_root cp -a "${source}" "${destination}"
}

source_dir=""
rollback_commit=""
deployment_dir=""
mode="${1:-}"
[[ -n "${mode}" ]] || { usage >&2; exit 2; }
shift
while [[ $# -gt 0 ]]; do
  case "$1" in
    --source-dir)
      source_dir="${2:-}"
      shift 2
      ;;
    --rollback-commit)
      rollback_commit="${2:-}"
      shift 2
      ;;
    --deployment-dir)
      deployment_dir="${2:-}"
      shift 2
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      printf 'unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
done

if [[ ! -d "${source_dir}/.git" && ! -f "${source_dir}/.git" ]]; then
  printf 'source checkout is not a Git worktree: %s\n' "${source_dir}" >&2
  exit 2
fi
source_commit="$(git -C "${source_dir}" rev-parse HEAD)"
source_tree="$(git -C "${source_dir}" rev-parse 'HEAD^{tree}')"
if [[ -n "$(git -C "${source_dir}" status --porcelain --untracked-files=no)" ]]; then
  printf 'source checkout has tracked changes\n' >&2
  exit 2
fi
if [[ "${USE_SUDO}" == "1" ]]; then
  sudo -n true
fi
command -v jq >/dev/null

if [[ "${mode}" == "prepare" ]]; then
  [[ "${rollback_commit}" =~ ^[0-9a-f]{7,64}$ ]] || {
    printf 'rollback commit is invalid\n' >&2
    exit 2
  }
  deployment_id="$(date -u +%Y%m%dT%H%M%SZ)-${source_commit:0:12}"
  deployment_dir="${RECEIPT_ROOT}/${deployment_id}"
  if as_root test -e "${deployment_dir}"; then
    printf 'deployment directory already exists: %s\n' "${deployment_dir}" >&2
    exit 2
  fi
  as_root install -d -m 0700 "${deployment_dir}/rollback"
  for path in ${BINARIES} ${CONFIG_PATHS}; do
    if as_root test -e "${path}"; then
      copy_for_rollback "${path}" "${deployment_dir}"
    fi
  done
  for unit in ${UNITS}; do
    fragment="$(systemctl show -p FragmentPath --value "${unit}" 2>/dev/null || true)"
    if [[ -n "${fragment}" ]] && as_root test -f "${fragment}"; then
      copy_for_rollback "${fragment}" "${deployment_dir}"
    fi
  done

  work="$(mktemp -d)"
  trap 'rm -rf "${work}"' EXIT
  : > "${work}/rollback-manifest.sha256"
  while IFS= read -r -d '' path; do
    printf '%s  %s\n' "$(sha_file "${path}")" "${path}" \
      >> "${work}/rollback-manifest.sha256"
  done < <(as_root find "${deployment_dir}/rollback" -type f -print0 | sort -z)
  rollback_manifest_root="$(sha256sum "${work}/rollback-manifest.sha256" | awk '{print $1}')"
  hot_pid_before="$(unit_pid "${HOT_UNIT}")"
  nginx_pid_before="$(unit_pid "${NGINX_UNIT}")"
  jq -nS \
    --arg schema 'nando.deployment-preparation.v1' \
    --arg deployment_id "${deployment_id}" \
    --arg source_commit "${source_commit}" \
    --arg source_tree "${source_tree}" \
    --arg rollback_commit "${rollback_commit}" \
    --arg rollback_manifest_root_sha256 "${rollback_manifest_root}" \
    --argjson hot_pid_before "${hot_pid_before:-0}" \
    --argjson nginx_pid_before "${nginx_pid_before:-0}" \
    --argjson hot_restart_allowed "${ALLOW_HOT_RESTART_JSON}" \
    '{schema:$schema,deployment_id:$deployment_id,source_commit:$source_commit,
      source_tree:$source_tree,rollback_commit:$rollback_commit,
      rollback_manifest_root_sha256:$rollback_manifest_root_sha256,
      hot_pid_before:$hot_pid_before,nginx_pid_before:$nginx_pid_before,
      hot_restart_allowed:$hot_restart_allowed}' \
    > "${work}/prepared.json"
  as_root install -m 0600 "${work}/rollback-manifest.sha256" \
    "${deployment_dir}/rollback-manifest.sha256"
  as_root install -m 0600 "${work}/prepared.json" "${deployment_dir}/prepared.json"
  printf '%s\n' "${deployment_dir}"
  exit 0
fi

if [[ "${mode}" != "finalize" || -z "${deployment_dir}" ]]; then
  usage >&2
  exit 2
fi
prepared="$(as_root cat "${deployment_dir}/prepared.json")"
[[ "$(jq -r '.source_commit' <<<"${prepared}")" == "${source_commit}" ]]
[[ "$(jq -r '.source_tree' <<<"${prepared}")" == "${source_tree}" ]]
expected_rollback_root="$(jq -r '.rollback_manifest_root_sha256' <<<"${prepared}")"
actual_rollback_root="$(as_root cat "${deployment_dir}/rollback-manifest.sha256" | sha256sum | awk '{print $1}')"
[[ "${actual_rollback_root}" == "${expected_rollback_root}" ]]
while read -r expected path; do
  [[ "$(sha_file "${path}")" == "${expected}" ]]
done < <(as_root cat "${deployment_dir}/rollback-manifest.sha256")

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT
: > "${work}/artifacts.jsonl"
for path in ${BINARIES} ${CONFIG_PATHS}; do
  if as_root test -f "${path}"; then
    jq -nS --arg path "${path}" --arg sha "$(sha_file "${path}")" \
      --argjson size "$(as_root stat -c '%s' "${path}")" \
      '{path:$path,sha256:$sha,size_bytes:$size}' >> "${work}/artifacts.jsonl"
  fi
done
jq -sS '.' "${work}/artifacts.jsonl" > "${work}/artifacts.json"

: > "${work}/units.jsonl"
for unit in ${UNITS}; do
  fragment="$(systemctl show -p FragmentPath --value "${unit}" 2>/dev/null || true)"
  active="$(systemctl is-active "${unit}" 2>/dev/null || true)"
  enabled="$(systemctl is-enabled "${unit}" 2>/dev/null || true)"
  pid="$(unit_pid "${unit}")"
  fragment_sha=""
  if [[ -n "${fragment}" ]] && as_root test -f "${fragment}"; then
    fragment_sha="$(sha_file "${fragment}")"
  fi
  jq -nS --arg unit "${unit}" --arg active "${active}" --arg enabled "${enabled}" \
    --arg fragment_path "${fragment}" --arg fragment_sha256 "${fragment_sha}" \
    --argjson main_pid "${pid:-0}" \
    '{unit:$unit,active:$active,enabled:$enabled,main_pid:$main_pid,
      fragment_path:$fragment_path,fragment_sha256:$fragment_sha256}' \
    >> "${work}/units.jsonl"
done
jq -sS '.' "${work}/units.jsonl" > "${work}/units.json"

as_root install -d -m 0700 "${deployment_dir}/evidence"
: > "${work}/state-roots.jsonl"
for state_root in ${STATE_ROOTS}; do
  if as_root test -d "${state_root}"; then
    manifest="${work}/state-$(printf '%s' "${state_root}" | sha256sum | cut -c1-12).sha256"
    : > "${manifest}"
    while IFS= read -r -d '' path; do
      printf '%s  %s\n' "$(sha_file "${path}")" "${path}" >> "${manifest}"
    done < <(as_root find "${state_root}" -type f -print0 | sort -z)
    tree_root="$(sha256sum "${manifest}" | awk '{print $1}')"
    evidence_name="$(basename "${manifest}")"
    as_root install -m 0600 "${manifest}" "${deployment_dir}/evidence/${evidence_name}"
    jq -nS --arg path "${state_root}" --arg tree_root_sha256 "${tree_root}" \
      --arg manifest "evidence/${evidence_name}" \
      '{path:$path,tree_root_sha256:$tree_root_sha256,manifest:$manifest}' \
      >> "${work}/state-roots.jsonl"
  fi
done
jq -sS '.' "${work}/state-roots.jsonl" > "${work}/state-roots.json"

: > "${work}/runtime.jsonl"
for endpoint in ${RUNTIME_ENDPOINTS}; do
  label="${endpoint%%=*}"
  url="${endpoint#*=}"
  snapshot="${work}/${label}.json"
  curl -fsS --max-time 10 "${url}" | jq -S '.' > "${snapshot}"
  snapshot_sha="$(sha256sum "${snapshot}" | awk '{print $1}')"
  as_root install -m 0600 "${snapshot}" "${deployment_dir}/evidence/${label}.json"
  jq -nS --arg snapshot_label "${label}" --arg url "${url}" --arg sha "${snapshot_sha}" \
    --arg path "evidence/${label}.json" \
    '{label:$snapshot_label,url:$url,sha256:$sha,path:$path}' >> "${work}/runtime.jsonl"
done
jq -sS '.' "${work}/runtime.jsonl" > "${work}/runtime.json"

hot_pid_before="$(jq -r '.hot_pid_before' <<<"${prepared}")"
nginx_pid_before="$(jq -r '.nginx_pid_before' <<<"${prepared}")"
hot_restart_allowed="$(jq -r '.hot_restart_allowed // false' <<<"${prepared}")"
hot_pid_after="$(unit_pid "${HOT_UNIT}")"
nginx_pid_after="$(unit_pid "${NGINX_UNIT}")"
hot_unchanged=true
nginx_unchanged=true
if [[ -n "${HOT_UNIT}" && "${hot_pid_before}" != "${hot_pid_after}" ]]; then hot_unchanged=false; fi
if [[ -n "${NGINX_UNIT}" && "${nginx_pid_before}" != "${nginx_pid_after}" ]]; then nginx_unchanged=false; fi
[[ "${nginx_unchanged}" == true ]]
[[ "${hot_unchanged}" == true || "${hot_restart_allowed}" == true ]]

generated_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
jq -nS \
  --arg schema 'nando.deployment-receipt.v1' \
  --arg deployment_id "$(jq -r '.deployment_id' <<<"${prepared}")" \
  --arg generated_at "${generated_at}" \
  --arg source_commit "${source_commit}" --arg source_tree "${source_tree}" \
  --arg rollback_commit "$(jq -r '.rollback_commit' <<<"${prepared}")" \
  --arg rollback_path "${deployment_dir}/rollback" \
  --arg rollback_manifest_root_sha256 "${actual_rollback_root}" \
  --argjson hot_pid_before "${hot_pid_before}" --argjson hot_pid_after "${hot_pid_after:-0}" \
  --argjson nginx_pid_before "${nginx_pid_before}" --argjson nginx_pid_after "${nginx_pid_after:-0}" \
  --argjson hot_pid_unchanged "${hot_unchanged}" \
  --argjson hot_restart_allowed "${hot_restart_allowed}" \
  --argjson nginx_pid_unchanged "${nginx_unchanged}" \
  --argjson artifacts "$(cat "${work}/artifacts.json")" \
  --argjson units "$(cat "${work}/units.json")" \
  --argjson state_roots "$(cat "${work}/state-roots.json")" \
  --argjson runtime_snapshots "$(cat "${work}/runtime.json")" \
  '{schema:$schema,deployment_id:$deployment_id,generated_at:$generated_at,
    source:{commit:$source_commit,tree:$source_tree},artifacts:$artifacts,units:$units,
    state_roots:$state_roots,runtime_snapshots:$runtime_snapshots,
    invariants:{hot_pid_before:$hot_pid_before,hot_pid_after:$hot_pid_after,
      hot_pid_unchanged:$hot_pid_unchanged,hot_restart_allowed:$hot_restart_allowed,
      nginx_pid_before:$nginx_pid_before,nginx_pid_after:$nginx_pid_after,
      nginx_pid_unchanged:$nginx_pid_unchanged},
    rollback:{source_commit:$rollback_commit,path:$rollback_path,
      manifest_root_sha256:$rollback_manifest_root_sha256}}' \
  > "${work}/receipt-payload.json"
receipt_root="$(sha256sum "${work}/receipt-payload.json" | awk '{print $1}')"
jq -S --arg root "${receipt_root}" '. + {receipt_root_sha256:$root}' \
  "${work}/receipt-payload.json" > "${work}/deployment-receipt.json"
as_root install -m 0600 "${work}/deployment-receipt.json" \
  "${deployment_dir}/deployment-receipt.json.candidate"
as_root mv -f "${deployment_dir}/deployment-receipt.json.candidate" \
  "${deployment_dir}/deployment-receipt.json"
as_root chmod -R a-w "${deployment_dir}"
printf '%s %s\n' "${deployment_dir}/deployment-receipt.json" "${receipt_root}"
