#!/usr/bin/env bash
# shellcheck disable=SC2029
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

paper_commit=1c50fea7119a123379bb7dca5a0eccbda63a9a7b
paper_tree=4cd54fd00b349d4fa70236cda6a90c1d1cf80a18
preflight_commit=be77371dff05b5eade9841e6612a59937648c2c8
candidate_commit=03e3dd00c90206e2f705371318c50dd50537d6d8
candidate_tree=06a9df51797dffc127fec41672bddae29c38bb92
candidate_config=plans/effect-law-unification-v1/evidence/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V7/transition-serving.env.candidate
remote=e@192.168.3.94
remote_repo=/home/e/projects/nando-wave-control-v14-66edd01
remote_target=/home/e/.cache/nando-wave-s1c3h-release-target
implementation_files=(
  ops/remote-backend/s1c3h_remote_transaction_v1.py
  ops/remote-backend/verify_s1c3h_transaction_v1.py
  ops/remote-backend/test_s1c3h_transaction_v1.py
  ops/remote-backend/run_s1c3h_transaction_v1.sh
)
dependency_files=(
  ops/remote-backend/s1c3g_remote_transaction_v1.py
  ops/remote-backend/verify_s1c3g_transaction_v1.py
  ops/remote-backend/s1c3f_remote_transaction_v1.py
  ops/remote-backend/verify_s1c3f_transaction_v1.py
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
git merge-base --is-ancestor "$preflight_commit" "$head"
[[ $(git rev-parse "$paper_commit^{tree}") == "$paper_tree" ]]
[[ $(git rev-parse "$candidate_commit^{tree}") == "$candidate_tree" ]]
[[ -z $(git status --porcelain --untracked-files=no) ]] || { printf 'tracked_worktree_dirty\n' >&2; exit 2; }
for path in "${implementation_files[@]}" "${dependency_files[@]}" "$candidate_config"; do
  git cat-file -e "HEAD:$path"
  [[ $(git show "HEAD:$path" | sha256sum | awk '{print $1}') == $(sha256sum "$path" | awk '{print $1}') ]]
done

test_modules=(
  test_s1c3h_transaction_v1.py
  test_s1c3g_transaction_v1.py
  test_s1c3f_transaction_v1.py
  test_s1c3e_transaction_v1.py
  test_s1c3d_transaction_v1.py
)
for test_module in "${test_modules[@]}"; do
  PYTHONPATH=ops/remote-backend python3 -m unittest \
    "ops/remote-backend/$test_module" >/dev/null
done
python3 -m py_compile "${implementation_files[@]:0:3}" "${dependency_files[@]}"
bash -n "${implementation_files[3]}"
if command -v shellcheck >/dev/null; then
  shellcheck "${implementation_files[3]}"
fi
remote_head=$(git ls-remote origin "refs/heads/$branch" | awk '{print $1}')
[[ $remote_head == "$head" ]] || { printf 'implementation_head_not_pushed\n' >&2; exit 2; }

timestamp=$(date -u +%Y%m%dT%H%M%SZ)
transaction_id="${timestamp}-${head:0:12}-s1c3h-v1"
local_dir="$output_parent/$transaction_id"
remote_upload="/home/e/.cache/${transaction_id}-upload"
remote_transaction="/var/lib/nando-wave/deployments/$transaction_id"
remote_build="/home/e/.cache/${transaction_id}-build"
install -d -m 0700 "$local_dir"
work=$(mktemp -d)
rollback_armed=false

cleanup() {
  rm -rf "$work"
  ssh "$remote" "git -C '$remote_repo' worktree remove --force '$remote_build/source' >/dev/null 2>&1 || true; git -C '$remote_repo' worktree prune; rm -rf '$remote_upload' '$remote_build'" >/dev/null 2>&1 || true
}

connector_snapshot() {
  local label=$1 destination=$2 pid active nrestarts route_failures command_sha
  active=$(systemctl --user show nando-client-connector.service -p ActiveState --value)
  pid=$(systemctl --user show nando-client-connector.service -p MainPID --value)
  nrestarts=$(systemctl --user show nando-client-connector.service -p NRestarts --value)
  route_failures=$(curl -fsS --max-time 4 http://127.0.0.1:18786/metrics | jq -er '.route_receipt_failures')
  command_sha=$(tr '\0' ' ' < "/proc/$pid/cmdline" | sha256sum | awk '{print $1}')
  jq -nS --arg schema nando.s1c3h-connector-snapshot.v1 --arg label "$label" \
    --arg active_state "$active" --arg command_sha256 "$command_sha" \
    --argjson main_pid "$pid" --argjson nrestarts "$nrestarts" \
    --argjson route_receipt_failures "$route_failures" \
    '{schema:$schema,label:$label,active_state:$active_state,main_pid:$main_pid,
      nrestarts:$nrestarts,route_receipt_failures:$route_receipt_failures,
      command_sha256:$command_sha256}' > "$destination"
}

mirror_remote() {
  rm -rf "$local_dir/remote-mirror"
  mkdir -p "$local_dir/remote-mirror"
  ssh "$remote" "sudo -n tar --exclude=rollback --exclude=.mutation.lock -C '$remote_transaction' -cf - ." |
    tar -C "$local_dir/remote-mirror" -xf -
}

finish_rollback() {
  connector_snapshot after "$work/connector-after.json"
  scp -q "$work/connector-after.json" "$remote:$remote_upload/connector-after.json"
  ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3h_remote_transaction_v1.py' finalize \
    --transaction-directory '$remote_transaction' --connector-before '$remote_upload/connector-before.json' \
    --connector-after '$remote_upload/connector-after.json'" > "$local_dir/finalize-result.json"
}

emergency() {
  local code=$? state
  trap - EXIT INT TERM HUP
  set +e
  if [[ $rollback_armed == true ]]; then
    state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'" 2>/dev/null)
    if [[ $state == PREPARED ]]; then
      ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3h_remote_transaction_v1.py' abort-predeployment \
        --transaction-directory '$remote_transaction' --reason orchestrator_interrupted_before_mutation" \
        > "$local_dir/emergency-preflight-abort.json" 2>&1
      state=COMPLETE
    fi
    case "$state" in
      ROLLBACK_ARMED|MUTATION_STARTED|AUTHORITY_INSTALLED|RUNTIME_INSTALLED|FINALIZE_PENDING|FINAL_VERIFICATION_PENDING)
        ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3h_remote_transaction_v1.py' rollback \
          --transaction-directory '$remote_transaction' --reason orchestrator_interrupted" \
          > "$local_dir/emergency-rollback.json" 2>&1
        state=ROLLBACK_PENDING
        ;;
    esac
    [[ $state != ROLLBACK_PENDING ]] || finish_rollback
  fi
  cleanup
  exit "$code"
}
trap cleanup EXIT

prior=$(ssh "$remote" "sudo -n find /var/lib/nando-wave/deployments -mindepth 1 -maxdepth 1 -type d -name '*-${head:0:12}-s1c3h-v1' -print | wc -l")
[[ $prior == 0 ]] || { printf 's1c3h_identity_consumed=%s\n' "$prior" >&2; exit 2; }
connector_snapshot before "$work/connector-before.json"
PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3h_transaction_v1.py create-freeze \
  --source-commit "$head" --source-tree "$tree" > "$work/implementation-freeze.json"

ssh "$remote" "set -euo pipefail
  rm -rf '$remote_build'
  install -d -m 0700 '$remote_build'
  git -C '$remote_repo' worktree add --detach '$remote_build/source' '$candidate_commit'
  CARGO_TARGET_DIR='$remote_target' /home/e/.cargo/bin/cargo build --release --locked \
    --manifest-path '$remote_build/source/Cargo.toml' \
    -p nando-transition-serving --bin nando-transition-serving \
    -p nando-response-actor --bin nando-response-admission
  install -m 0500 '$remote_target/release/nando-transition-serving' '$remote_build/nando-transition-serving'
  install -m 0500 '$remote_target/release/nando-response-admission' '$remote_build/nando-response-admission'
  git -C '$remote_repo' worktree remove --force '$remote_build/source'"
scp -q "$candidate_config" "$remote:$remote_build/transition-serving.env.candidate"

ssh "$remote" "rm -rf '$remote_upload'; install -d -m 0700 '$remote_upload'"
scp -q "${implementation_files[@]}" "${dependency_files[@]}" \
  "$work/connector-before.json" "$work/implementation-freeze.json" "$remote:$remote_upload/"
ssh "$remote" "env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3h_transaction_v1.py' create-build-receipt \
  --transition '$remote_build/nando-transition-serving' \
  --authority '$remote_build/nando-response-admission' \
  --config '$remote_build/transition-serving.env.candidate'" > "$work/candidate-build-receipt.json"
scp -q "$work/candidate-build-receipt.json" "$remote:$remote_upload/candidate-build-receipt.json"

set +e
ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3h_remote_transaction_v1.py' prepare \
  --transaction-id '$transaction_id' --transaction-directory '$remote_transaction' \
  --candidate-transition '$remote_build/nando-transition-serving' \
  --candidate-authority '$remote_build/nando-response-admission' \
  --candidate-config '$remote_build/transition-serving.env.candidate' \
  --build-receipt '$remote_upload/candidate-build-receipt.json' \
  --implementation-freeze '$remote_upload/implementation-freeze.json'" \
  > "$local_dir/prepare-result.json" 2> "$local_dir/prepare-error.json"
prepare_code=$?
set -e
if [[ $prepare_code -ne 0 ]]; then
  state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'")
  [[ $state == PREFLIGHT_FAILURE ]]
  ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3h_remote_transaction_v1.py' abort-predeployment \
    --transaction-directory '$remote_transaction' --reason prepare_failed" > "$local_dir/preflight-seal.json"
  mirror_remote
  printf 'transaction_directory=%s\nlocal_evidence=%s\nverdict=S1C3H_PREFLIGHT_FAILURE production_mutation=no\n' \
    "$remote_transaction" "$local_dir"
  exit 3
fi
rollback_armed=true
trap emergency EXIT INT TERM HUP
mirror_remote
PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3h_transaction_v1.py verify \
  "$local_dir/remote-mirror" --implementation-freeze "$local_dir/remote-mirror/implementation-freeze.json" \
  --predeployment > "$local_dir/s1c3h-predeployment.local.json"
scp -q "$local_dir/s1c3h-predeployment.local.json" "$remote:$remote_upload/predeployment-verification.json"
ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3h_transaction_v1.py' verify \
  '$remote_transaction' --implementation-freeze '$remote_transaction/implementation-freeze.json' --predeployment" \
  > "$local_dir/s1c3h-predeployment.remote.json"
cmp "$local_dir/s1c3h-predeployment.local.json" "$local_dir/s1c3h-predeployment.remote.json"

set +e
ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3h_remote_transaction_v1.py' execute \
  --transaction-directory '$remote_transaction' --predeployment-verification '$remote_upload/predeployment-verification.json'" \
  > "$local_dir/execute-result.json" 2> "$local_dir/execute-error.json"
execute_code=$?
set -e
state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'")
case "$state" in
  PREPARED)
    ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3h_remote_transaction_v1.py' abort-predeployment \
      --transaction-directory '$remote_transaction' --reason execute_failed_before_mutation" > "$local_dir/preflight-abort.json"
    rollback_armed=false
    trap cleanup EXIT
    printf 'verdict=S1C3H_PREFLIGHT_FAILURE execute_code=%s production_mutation=no\n' "$execute_code"
    exit 3
    ;;
  ROLLBACK_ARMED|MUTATION_STARTED|AUTHORITY_INSTALLED|RUNTIME_INSTALLED)
    ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3h_remote_transaction_v1.py' rollback \
      --transaction-directory '$remote_transaction' --reason execute_session_aborted" > "$local_dir/recovery-rollback.json"
    state=ROLLBACK_PENDING
    ;;
esac
[[ $state == FINALIZE_PENDING || $state == ROLLBACK_PENDING ]]

connector_snapshot after "$work/connector-after.json"
scp -q "$work/connector-after.json" "$remote:$remote_upload/connector-after.json"
ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3h_remote_transaction_v1.py' finalize \
  --transaction-directory '$remote_transaction' --connector-before '$remote_upload/connector-before.json' \
  --connector-after '$remote_upload/connector-after.json'" > "$local_dir/finalize-result.json"
mirror_remote
PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3h_transaction_v1.py verify \
  "$local_dir/remote-mirror" --implementation-freeze "$local_dir/remote-mirror/implementation-freeze.json" \
  > "$local_dir/s1c3h-final.local.json"
scp -q "$local_dir/s1c3h-final.local.json" "$remote:$remote_upload/final-verification.json"
ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3h_transaction_v1.py' verify \
  '$remote_transaction' --implementation-freeze '$remote_transaction/implementation-freeze.json'" \
  > "$local_dir/s1c3h-final.remote.json"
cmp "$local_dir/s1c3h-final.local.json" "$local_dir/s1c3h-final.remote.json"
ssh "$remote" "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3h_remote_transaction_v1.py' seal \
  --transaction-directory '$remote_transaction' --final-verification '$remote_upload/final-verification.json'" \
  > "$local_dir/seal-result.json"
rollback_armed=false
trap cleanup EXIT
mirror_remote
verdict=$(jq -er .verdict "$local_dir/remote-mirror/s1c3h-state.json")
printf 'transaction_directory=%s\nlocal_evidence=%s\nexecute_code=%s verdict=%s\n' \
  "$remote_transaction" "$local_dir" "$execute_code" "$verdict"
[[ $verdict == S1C3H_DEPLOYMENT_PASS ]]
