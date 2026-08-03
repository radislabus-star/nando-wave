#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPT="${ROOT}/deployment-receipt.sh"
WORK="$(mktemp -d)"

grep -Fq '/opt/nando-wave/bin/nando-operator-certification-authority' "${SCRIPT}"
grep -Fq '/opt/nando-wave/bin/nando-operator-cleanup-verifier' "${SCRIPT}"
grep -Fq 'nando-operator-certification-authority.service' "${SCRIPT}"
grep -Fq 'nando-operator-cleanup-verifier@.service' "${SCRIPT}"
grep -Fq '/opt/nando-wave/ops/phase-center-test-server/bin/nando-live-transition-gate' "${SCRIPT}"
if grep -Fq 'nando-live-transition-gate.timer' "${SCRIPT}"; then
  printf 'obsolete live transition gate timer remains in receipt contract\n' >&2
  exit 1
fi

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
  and .invariants.hot_restart_allowed == false
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

mkdir -p "${WORK}/fake-bin"
# The expressions belong to the generated fixture.
# shellcheck disable=SC2016
printf '%s\n' \
  '#!/usr/bin/env bash' \
  'if [[ "$1" == "show" && "$2" == "-p" && "$3" == "MainPID" ]]; then' \
  '  cat "${NANDO_TEST_PID_ROOT}/$5.pid"' \
  'elif [[ "$1" == "show" && "$2" == "-p" && "$3" == "FragmentPath" ]]; then' \
  '  printf "\\n"' \
  'elif [[ "$1" == "is-active" ]]; then' \
  '  printf "active\\n"' \
  'elif [[ "$1" == "is-enabled" ]]; then' \
  '  printf "enabled\\n"' \
  'fi' > "${WORK}/fake-bin/systemctl"
chmod +x "${WORK}/fake-bin/systemctl"
printf '101\n' > "${WORK}/hot.service.pid"
printf '202\n' > "${WORK}/nginx.service.pid"

denied_env=(
  "${common_env[@]}"
  PATH="${WORK}/fake-bin:${PATH}"
  NANDO_TEST_PID_ROOT="${WORK}"
  NANDO_DEPLOY_RECEIPT_ROOT="${WORK}/denied-restart-receipts"
  NANDO_DEPLOY_HOT_UNIT=hot.service
  NANDO_DEPLOY_NGINX_UNIT=nginx.service
)
denied_dir="$(env "${denied_env[@]}" "${SCRIPT}" prepare \
  --source-dir "${WORK}/source" --rollback-commit "${commit}")"
printf '303\n' > "${WORK}/hot.service.pid"
if env "${denied_env[@]}" "${SCRIPT}" finalize \
  --source-dir "${WORK}/source" --deployment-dir "${denied_dir}"; then
  printf 'unexpected hot restart was accepted\n' >&2
  exit 1
fi

printf '101\n' > "${WORK}/hot.service.pid"
restart_env=(
  "${common_env[@]}"
  PATH="${WORK}/fake-bin:${PATH}"
  NANDO_TEST_PID_ROOT="${WORK}"
  NANDO_DEPLOY_RECEIPT_ROOT="${WORK}/restart-receipts"
  NANDO_DEPLOY_HOT_UNIT=hot.service
  NANDO_DEPLOY_NGINX_UNIT=nginx.service
  NANDO_DEPLOY_ALLOW_HOT_RESTART=1
)
restart_dir="$(env "${restart_env[@]}" "${SCRIPT}" prepare \
  --source-dir "${WORK}/source" --rollback-commit "${commit}")"
printf '303\n' > "${WORK}/hot.service.pid"
restart_result="$(env "${restart_env[@]}" "${SCRIPT}" finalize \
  --source-dir "${WORK}/source" --deployment-dir "${restart_dir}")"
restart_receipt="${restart_result%% *}"
jq -e '
  .invariants.hot_pid_before == 101
  and .invariants.hot_pid_after == 303
  and .invariants.hot_pid_unchanged == false
  and .invariants.hot_restart_allowed == true
  and .invariants.nginx_pid_before == 202
  and .invariants.nginx_pid_after == 202
  and .invariants.nginx_pid_unchanged == true
' "${restart_receipt}" >/dev/null

# The expressions belong to the generated fixture.
# shellcheck disable=SC2016
printf '%s\n' \
  '#!/usr/bin/env bash' \
  'count=0' \
  '[[ ! -f "${NANDO_TEST_CURL_COUNT}" ]] || count="$(cat "${NANDO_TEST_CURL_COUNT}")"' \
  'count=$((count + 1))' \
  'printf "%s\n" "${count}" > "${NANDO_TEST_CURL_COUNT}"' \
  'if (( count == 1 )); then exit 22; fi' \
  'printf "{\"ready\":true}\n"' > "${WORK}/fake-bin/curl"
chmod +x "${WORK}/fake-bin/curl"
retry_env=(
  "${common_env[@]}"
  PATH="${WORK}/fake-bin:${PATH}"
  NANDO_TEST_CURL_COUNT="${WORK}/curl.count"
  NANDO_DEPLOY_RECEIPT_ROOT="${WORK}/retry-receipts"
  NANDO_DEPLOY_RECEIPT_ENDPOINTS="probe=http://127.0.0.1/retry"
  NANDO_DEPLOY_RUNTIME_SNAPSHOT_ATTEMPTS=2
  NANDO_DEPLOY_RUNTIME_SNAPSHOT_SLEEP_SECONDS=0
)
retry_dir="$(env "${retry_env[@]}" "${SCRIPT}" prepare \
  --source-dir "${WORK}/source" --rollback-commit "${commit}")"
env "${retry_env[@]}" "${SCRIPT}" finalize \
  --source-dir "${WORK}/source" --deployment-dir "${retry_dir}" >/dev/null
[[ "$(cat "${WORK}/curl.count")" == 2 ]]
printf 'deployment receipt transaction tests: PASS\n'
