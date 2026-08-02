#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPT="${ROOT}/deployment-receipt.sh"
WORK="$(mktemp -d)"

grep -Fq '/opt/nando-wave/bin/nando-operator-certification-authority' "${SCRIPT}"
grep -Fq '/opt/nando-wave/bin/nando-operator-cleanup-verifier' "${SCRIPT}"

cleanup() {
  chmod -R u+w "${WORK}" 2>/dev/null || true
  rm -rf "${WORK}"
}
trap cleanup EXIT

git -C "${WORK}" init -q source
git -C "${WORK}/source" config user.email nando@example.invalid
git -C "${WORK}/source" config user.name Nando
printf 'source\n' > "${WORK}/source/file"
git -C "${WORK}/source" add file
git -C "${WORK}/source" commit -qm source
commit="$(git -C "${WORK}/source" rev-parse HEAD)"
mkdir -p "${WORK}/installed" "${WORK}/state"
printf 'old binary\n' > "${WORK}/installed/server"
printf 'state\n' > "${WORK}/state/ledger"

common_env=(
  NANDO_DEPLOY_USE_SUDO=0
  NANDO_DEPLOY_RECEIPT_ROOT="${WORK}/receipts"
  NANDO_DEPLOY_RECEIPT_BINARIES="${WORK}/installed/server"
  NANDO_DEPLOY_RECEIPT_CONFIGS=
  NANDO_DEPLOY_RECEIPT_UNITS=
  NANDO_DEPLOY_RECEIPT_STATE_ROOTS="${WORK}/state"
  NANDO_DEPLOY_RECEIPT_ENDPOINTS=
  NANDO_DEPLOY_HOT_UNIT=
  NANDO_DEPLOY_NGINX_UNIT=
)
deployment_dir="$(env "${common_env[@]}" "${SCRIPT}" prepare \
  --source-dir "${WORK}/source" --rollback-commit "${commit}")"
printf 'new binary\n' > "${WORK}/installed/server"
result="$(env "${common_env[@]}" "${SCRIPT}" finalize \
  --source-dir "${WORK}/source" --deployment-dir "${deployment_dir}")"
receipt_path="${result%% *}"
receipt_root="${result##* }"

jq -e --arg commit "${commit}" --arg root "${receipt_root}" '
  .schema == "nando.deployment-receipt.v1"
  and .source.commit == $commit
  and .receipt_root_sha256 == $root
  and .invariants.hot_pid_unchanged == true
  and .invariants.nginx_pid_unchanged == true
  and (.artifacts | length) == 1
  and (.state_roots | length) == 1
' "${receipt_path}" >/dev/null
[[ -f "${deployment_dir}/rollback${WORK}/installed/server" ]]
[[ "$(cat "${deployment_dir}/rollback${WORK}/installed/server")" == "old binary" ]]
if (printf 'tamper\n' >> "${deployment_dir}/prepared.json") 2>/dev/null; then
  printf 'deployment receipt directory remained writable\n' >&2
  exit 1
fi
printf 'deployment receipt transaction tests: PASS\n'
