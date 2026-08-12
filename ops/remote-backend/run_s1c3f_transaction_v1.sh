#!/usr/bin/env bash
# shellcheck disable=SC2029
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

paper_commit=e6c733a243fb8c95920d971c17edf1c3cda65def
paper_tree=0e914870e08e132c93d9608c1e4873c0636ed5d4
remote=e@192.168.3.94
implementation_files=(
  ops/remote-backend/s1c3f_remote_transaction_v1.py
  ops/remote-backend/verify_s1c3f_transaction_v1.py
  ops/remote-backend/test_s1c3f_transaction_v1.py
  ops/remote-backend/run_s1c3f_transaction_v1.sh
)
dependency_files=(
  ops/remote-backend/s1c3e_remote_transaction_v1.py
  ops/remote-backend/s1c3b_remote_transaction_v1.py
  ops/remote-backend/s1c3_remote_transaction_v7.py
)

[[ $# -eq 1 ]] || { printf 'usage: %s OUTPUT_PARENT\n' "$0" >&2; exit 2; }
output_parent=$1
head=$(git rev-parse HEAD)
tree=$(git rev-parse 'HEAD^{tree}')
branch=$(git branch --show-current)
git merge-base --is-ancestor "$paper_commit" "$head"
[[ $(git rev-parse "$paper_commit^{tree}") == "$paper_tree" ]]
[[ -z $(git status --porcelain --untracked-files=no) ]] || { printf 'tracked_worktree_dirty\n' >&2; exit 2; }
for path in "${implementation_files[@]}" "${dependency_files[@]}"; do
  git cat-file -e "HEAD:$path"
  [[ $(git show "HEAD:$path" | sha256sum | awk '{print $1}') == $(sha256sum "$path" | awk '{print $1}') ]]
done

# All local gates precede timestamp creation and every remote side effect.
PYTHONPATH=ops/remote-backend python3 -m unittest ops/remote-backend/test_s1c3f_transaction_v1.py >/dev/null
python3 -m py_compile "${implementation_files[@]:0:3}"
bash -n "${implementation_files[3]}"
remote_head=$(git ls-remote origin "refs/heads/$branch" | awk '{print $1}')
[[ $remote_head == "$head" ]] || { printf 'implementation_head_not_pushed\n' >&2; exit 2; }

timestamp=$(date -u +%Y%m%dT%H%M%SZ)
transaction_id="${timestamp}-${head:0:12}-s1c3f-v1"
local_dir="$output_parent/$transaction_id"
remote_upload="/home/e/.cache/${transaction_id}-upload"
remote_transaction="/var/lib/nando-wave/deployments/$transaction_id"
install -d -m 0700 "$local_dir"
work=$(mktemp -d)
rollback_armed=false

cleanup() {
  rm -rf "$work"
  ssh "$remote" "rm -rf '$remote_upload'" >/dev/null 2>&1 || true
}

connector_snapshot() {
  local label=$1 destination=$2 pid active nrestarts route_failures command_sha
  active=$(systemctl --user show nando-client-connector.service -p ActiveState --value)
  pid=$(systemctl --user show nando-client-connector.service -p MainPID --value)
  nrestarts=$(systemctl --user show nando-client-connector.service -p NRestarts --value)
  route_failures=$(curl -fsS --max-time 4 http://127.0.0.1:18786/metrics | jq -er '.route_receipt_failures')
  command_sha=$(tr '\0' ' ' < "/proc/$pid/cmdline" | sha256sum | awk '{print $1}')
  jq -nS --arg schema nando.s1c3f-connector-snapshot.v1 --arg label "$label" \
    --arg active_state "$active" --arg command_sha256 "$command_sha" \
    --argjson main_pid "$pid" --argjson nrestarts "$nrestarts" \
    --argjson route_receipt_failures "$route_failures" \
    '{schema:$schema,label:$label,active_state:$active_state,main_pid:$main_pid,
      nrestarts:$nrestarts,route_receipt_failures:$route_receipt_failures,
      command_sha256:$command_sha256}' > "$destination"
}

mirror_remote() {
  rm -rf "$local_dir/remote-mirror"; mkdir -p "$local_dir/remote-mirror"
  ssh "$remote" "sudo -n tar --exclude=rollback --exclude=.mutation.lock -C '$remote_transaction' -cf - ." |
    tar -C "$local_dir/remote-mirror" -xf -
}

finish_rollback() {
  connector_snapshot after "$work/connector-after.json"
  scp -q "$work/connector-after.json" "$remote:$remote_upload/connector-after.json"
  ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3f_remote_transaction_v1.py' finalize \
    --transaction-directory '$remote_transaction' --connector-after '$remote_upload/connector-after.json'" \
    > "$local_dir/finalize-result.json"
}

emergency() {
  local code=$? state
  trap - EXIT INT TERM HUP; set +e
  if [[ $rollback_armed == true ]]; then
    state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'" 2>/dev/null)
    if [[ $state == PREPARED ]]; then
      ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3f_remote_transaction_v1.py' abort-predeployment \
        --transaction-directory '$remote_transaction' --reason orchestrator_interrupted_before_mutation" \
        > "$local_dir/emergency-preflight-abort.json" 2>&1
      state=PREFLIGHT_FAILURE
    fi
    if [[ $state == ROLLBACK_ARMED || $state == FINALIZE_PENDING || $state == FINAL_VERIFICATION_PENDING ]]; then
      ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3f_remote_transaction_v1.py' rollback \
        --transaction-directory '$remote_transaction' --reason orchestrator_interrupted" \
        > "$local_dir/emergency-rollback.json" 2>&1
      state=ROLLBACK_PENDING
    fi
    [[ $state != ROLLBACK_PENDING ]] || finish_rollback
  fi
  cleanup; exit "$code"
}
trap cleanup EXIT

prior=$(ssh "$remote" "sudo -n find /var/lib/nando-wave/deployments -mindepth 1 -maxdepth 1 -type d -name '*-${head:0:12}-s1c3f-v1' -print | wc -l")
[[ $prior == 0 ]] || { printf 's1c3f_identity_consumed=%s\n' "$prior" >&2; exit 2; }
connector_snapshot before "$work/connector-before.json"
PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3f_transaction_v1.py create-freeze \
  --source-commit "$head" --source-tree "$tree" > "$work/implementation-freeze.json"
ssh "$remote" "rm -rf '$remote_upload'; install -d -m 0700 '$remote_upload'"
scp -q "${implementation_files[@]}" "${dependency_files[@]}" \
  "$work/connector-before.json" "$work/implementation-freeze.json" "$remote:$remote_upload/"

ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3f_remote_transaction_v1.py' prepare \
  --transaction-id '$transaction_id' --transaction-directory '$remote_transaction' \
  --connector-before '$remote_upload/connector-before.json' \
  --implementation-freeze '$remote_upload/implementation-freeze.json'" > "$local_dir/prepare-result.json"
rollback_armed=true; trap emergency EXIT INT TERM HUP
mirror_remote
PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3f_transaction_v1.py verify \
  "$local_dir/remote-mirror" --implementation-freeze "$local_dir/remote-mirror/implementation-freeze.json" \
  --predeployment > "$local_dir/s1c3f-predeployment.local.json"
scp -q "$local_dir/s1c3f-predeployment.local.json" "$remote:$remote_upload/predeployment-verification.json"
ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3f_transaction_v1.py' verify \
  '$remote_transaction' --implementation-freeze '$remote_transaction/implementation-freeze.json' --predeployment" \
  > "$local_dir/s1c3f-predeployment.remote.json"
cmp "$local_dir/s1c3f-predeployment.local.json" "$local_dir/s1c3f-predeployment.remote.json"

set +e
ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3f_remote_transaction_v1.py' execute \
  --transaction-directory '$remote_transaction' --predeployment-verification '$remote_upload/predeployment-verification.json'" \
  > "$local_dir/execute-result.json" 2> "$local_dir/execute-error.json"
execute_code=$?; set -e
state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'")
if [[ $state == PREPARED ]]; then
  ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3f_remote_transaction_v1.py' abort-predeployment \
    --transaction-directory '$remote_transaction' --reason execute_failed_before_mutation" > "$local_dir/preflight-abort.json"
  rollback_armed=false; trap cleanup EXIT
  printf 'verdict=S1C3F_PREFLIGHT_FAILURE execute_code=%s production_mutation=no\n' "$execute_code"; exit 3
fi
if [[ $state == ROLLBACK_ARMED ]]; then
  ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3f_remote_transaction_v1.py' rollback \
    --transaction-directory '$remote_transaction' --reason execute_session_aborted" > "$local_dir/recovery-rollback.json"
  state=ROLLBACK_PENDING
fi
[[ $state == FINALIZE_PENDING || $state == ROLLBACK_PENDING ]]

connector_snapshot after "$work/connector-after.json"
scp -q "$work/connector-after.json" "$remote:$remote_upload/connector-after.json"
ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3f_remote_transaction_v1.py' finalize \
  --transaction-directory '$remote_transaction' --connector-after '$remote_upload/connector-after.json'" > "$local_dir/finalize-result.json"
mirror_remote
PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3f_transaction_v1.py verify \
  "$local_dir/remote-mirror" --implementation-freeze "$local_dir/remote-mirror/implementation-freeze.json" \
  > "$local_dir/s1c3f-final.local.json"
scp -q "$local_dir/s1c3f-final.local.json" "$remote:$remote_upload/final-verification.json"
ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3f_transaction_v1.py' verify \
  '$remote_transaction' --implementation-freeze '$remote_transaction/implementation-freeze.json'" \
  > "$local_dir/s1c3f-final.remote.json"
cmp "$local_dir/s1c3f-final.local.json" "$local_dir/s1c3f-final.remote.json"
ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3f_remote_transaction_v1.py' seal \
  --transaction-directory '$remote_transaction' --final-verification '$remote_upload/final-verification.json'" > "$local_dir/seal-result.json"
rollback_armed=false; trap cleanup EXIT; mirror_remote
verdict=$(jq -er .verdict "$local_dir/remote-mirror/s1c3f-state.json")
printf 'transaction_directory=%s\nlocal_evidence=%s\nexecute_code=%s verdict=%s\n' \
  "$remote_transaction" "$local_dir" "$execute_code" "$verdict"
[[ $verdict == S1C3F_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH ]]
