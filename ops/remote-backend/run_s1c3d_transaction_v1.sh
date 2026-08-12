#!/usr/bin/env bash
# shellcheck disable=SC2029
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

paper_commit=c3eaddc55dfcdb45060c0d61278fd115a6707639
paper_tree=4127d54552c418baa3a9c324451a37c989a3a98f
candidate_commit=03e3dd00c90206e2f705371318c50dd50537d6d8
candidate_tree=06a9df51797dffc127fec41672bddae29c38bb92
candidate_config=plans/effect-law-unification-v1/evidence/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V7/transition-serving.env.candidate
parity_source=ops/remote-backend/s1c3-parity-oracle/main.rs
oracle_lock=plans/effect-law-unification-v1/evidence/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V7/oracle.Cargo.lock
remote=e@192.168.3.94
implementation_files=(
  ops/remote-backend/s1c3d_remote_transaction_v1.py
  ops/remote-backend/verify_s1c3d_transaction_v1.py
  ops/remote-backend/s1c3d_transaction_v1.py
  ops/remote-backend/test_s1c3d_transaction_v1.py
  ops/remote-backend/test_verify_s1c3d_transaction_v1.py
  ops/remote-backend/run_s1c3d_transaction_v1.sh
)
dependency_files=(
  ops/remote-backend/s1c3b_remote_transaction_v1.py
  ops/remote-backend/verify_s1c3b_transaction_v1.py
  ops/remote-backend/s1c3_remote_transaction_v7.py
  ops/remote-backend/verify_s1c3_transaction_v7.py
)

[[ $# -eq 1 ]] || { printf 'usage: %s OUTPUT_PARENT\n' "$0" >&2; exit 2; }
output_parent=$1
branch=$(git branch --show-current)
head=$(git rev-parse HEAD)
head_tree=$(git rev-parse 'HEAD^{tree}')

git merge-base --is-ancestor "$paper_commit" "$head"
git merge-base --is-ancestor "$candidate_commit" "$head"
git merge-base --is-ancestor 663959064a37caf7eb917fc99dfedb6386355fa6 "$head"
[[ $(git rev-parse "$paper_commit^{tree}") == "$paper_tree" ]]
[[ $(git rev-parse "$candidate_commit^{tree}") == "$candidate_tree" ]]
[[ -z $(git status --porcelain --untracked-files=no) ]] || {
  printf 'tracked_worktree_dirty\n' >&2
  exit 2
}
for path in "${implementation_files[@]}"; do
  git cat-file -e "HEAD:$path"
  [[ $(git show "HEAD:$path" | sha256sum | awk '{print $1}') == \
      $(sha256sum "$path" | awk '{print $1}') ]] || {
    printf 'implementation_file_differs_from_head=%s\n' "$path" >&2
    exit 2
  }
done

# All local gates precede timestamp creation, SSH, locks, and remote directories.
PYTHONPATH=ops/remote-backend python3 -m unittest \
  ops/remote-backend/test_s1c3d_transaction_v1.py \
  ops/remote-backend/test_verify_s1c3d_transaction_v1.py >/dev/null
python3 -m py_compile "${implementation_files[@]:0:3}"
remote_head=$(git ls-remote origin "refs/heads/$branch" | awk '{print $1}')
[[ $remote_head == "$head" ]] || {
  printf 'implementation_head_not_pushed local=%s remote=%s\n' "$head" "$remote_head" >&2
  exit 2
}

timestamp=$(date -u +%Y%m%dT%H%M%SZ)
transaction_id="${timestamp}-${paper_commit:0:12}-s1c3d-v1"
local_dir="$output_parent/$transaction_id"
remote_upload="/home/e/.cache/${transaction_id}-upload"
remote_transaction="/var/lib/nando-wave/deployments/$transaction_id"
[[ ! -e $local_dir ]]
install -d -m 0700 "$local_dir"
work=$(mktemp -d)
rollback_armed=false

cleanup() {
  rm -rf "$work"
}

connector_snapshot() {
  local label=$1 destination=$2
  local pid active nrestarts route_failures command_sha
  active=$(systemctl --user show nando-client-connector.service -p ActiveState --value)
  pid=$(systemctl --user show nando-client-connector.service -p MainPID --value)
  nrestarts=$(systemctl --user show nando-client-connector.service -p NRestarts --value)
  route_failures=$(curl -fsS --max-time 4 http://127.0.0.1:18786/metrics | jq -er '.route_receipt_failures')
  command_sha=$(tr '\0' ' ' < "/proc/$pid/cmdline" | sha256sum | awk '{print $1}')
  jq -nS \
    --arg schema nando.s1c3d-connector-snapshot.v1 \
    --arg label "$label" \
    --arg observed_at "$(date --iso-8601=ns)" \
    --arg active_state "$active" \
    --arg command_sha256 "$command_sha" \
    --argjson main_pid "$pid" \
    --argjson nrestarts "$nrestarts" \
    --argjson route_receipt_failures "$route_failures" \
    '{schema:$schema,label:$label,observed_at:$observed_at,active_state:$active_state,
      main_pid:$main_pid,nrestarts:$nrestarts,route_receipt_failures:$route_receipt_failures,
      command_sha256:$command_sha256}' > "$destination"
}

mirror_remote() {
  rm -rf "$local_dir/remote-mirror"
  mkdir -p "$local_dir/remote-mirror"
  ssh "$remote" \
    "sudo -n tar --exclude=rollback --exclude=.mutation.lock \
      -C '$remote_transaction' -cf - ." | tar -C "$local_dir/remote-mirror" -xf -
}

abort_predeployment() {
  local reason=$1
  ssh "$remote" \
    "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3d_transaction_v1.py' abort-predeployment \
      --transaction-directory '$remote_transaction' --reason '$reason'" \
    > "$local_dir/predeployment-abort.json" 2>&1
  mirror_remote
}

seal_resource_veto() {
  mirror_remote
  PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3d_transaction_v1.py verify \
    "$local_dir/remote-mirror" \
    --implementation-freeze "$local_dir/remote-mirror/implementation-freeze.json" \
    > "$local_dir/s1c3d-terminal.local.json"
  scp -q "$local_dir/s1c3d-terminal.local.json" \
    "$remote:$remote_upload/terminal-verification.json"
  ssh "$remote" \
    "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3d_transaction_v1.py' verify \
      '$remote_transaction' --implementation-freeze '$remote_transaction/implementation-freeze.json' \
      --recorded-verification '$remote_upload/terminal-verification.json'" \
    > "$local_dir/s1c3d-terminal.remote.json"
  cmp "$local_dir/s1c3d-terminal.local.json" "$local_dir/s1c3d-terminal.remote.json"
  scp -q "$local_dir/s1c3d-terminal.local.json" \
    "$remote:$remote_upload/terminal-envelope.json"
  ssh "$remote" \
    "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3d_transaction_v1.py' seal-resource-veto \
      --transaction-directory '$remote_transaction' \
      --terminal-verification '$remote_upload/terminal-verification.json' \
      --authority-envelope '$remote_upload/terminal-envelope.json'" \
    > "$local_dir/terminal-seal-result.json"
  mirror_remote
}

emergency_rollback() {
  local code=$?
  trap - EXIT INT TERM HUP
  set +e
  if [[ $rollback_armed == true ]]; then
    local state
    state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'" 2>/dev/null)
    if [[ $state == PREPARED ]]; then
      abort_predeployment orchestrator_interrupted_before_mutation
      state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'" 2>/dev/null)
    fi
    if [[ $state == RESOURCE_VETO ]]; then
      seal_resource_veto
      state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'" 2>/dev/null)
    fi
    if [[ $state == ROLLBACK_ARMED || $state == FINALIZE_PENDING || \
          $state == FINAL_VERIFICATION_PENDING ]]; then
      ssh "$remote" \
        "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3d_transaction_v1.py' rollback \
          --transaction-directory '$remote_transaction' --reason orchestrator_interrupted" \
        > "$local_dir/emergency-rollback.json" 2>&1
      state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'" 2>/dev/null)
    fi
    if [[ $state == ROLLBACK_PENDING ]]; then
      connector_snapshot after "$work/emergency-connector-after.json"
      scp -q "$work/emergency-connector-after.json" \
        "$remote:$remote_upload/emergency-connector-after.json"
      ssh "$remote" \
        "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3d_transaction_v1.py' finalize \
          --transaction-directory '$remote_transaction' \
          --connector-after '$remote_upload/emergency-connector-after.json'" \
        > "$local_dir/emergency-finalize.json" 2>&1
      state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'" 2>/dev/null)
    fi
    if [[ $state == FINAL_VERIFICATION_PENDING ]]; then
      mirror_remote
      PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3d_transaction_v1.py verify \
        "$local_dir/remote-mirror" \
        --implementation-freeze "$local_dir/remote-mirror/implementation-freeze.json" \
        > "$local_dir/emergency-final.local.json"
      scp -q "$local_dir/emergency-final.local.json" \
        "$remote:$remote_upload/emergency-final-verification.json"
      ssh "$remote" \
        "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3d_transaction_v1.py' verify \
          '$remote_transaction' --implementation-freeze '$remote_transaction/implementation-freeze.json' \
          --recorded-verification '$remote_upload/emergency-final-verification.json'" \
        > "$local_dir/emergency-final.remote.json" 2>&1
      if cmp "$local_dir/emergency-final.local.json" "$local_dir/emergency-final.remote.json"; then
        scp -q "$local_dir/emergency-final.local.json" \
          "$remote:$remote_upload/emergency-final-envelope.json"
        ssh "$remote" \
          "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3d_transaction_v1.py' seal \
            --transaction-directory '$remote_transaction' \
            --final-verification '$remote_upload/emergency-final-verification.json' \
            --authority-envelope '$remote_upload/emergency-final-envelope.json'" \
          > "$local_dir/emergency-seal.json" 2>&1
      fi
    fi
  fi
  cleanup
  exit "$code"
}

trap cleanup EXIT

prior_attempts=$(ssh "$remote" \
  "sudo -n find /var/lib/nando-wave/deployments -mindepth 1 -maxdepth 1 \
    -type d -name '*-${paper_commit:0:12}-s1c3d-v1' -print | wc -l")
[[ $prior_attempts == 0 ]] || {
  printf 's1c3d_identity_already_consumed count=%s\n' "$prior_attempts" >&2
  exit 2
}

connector_snapshot before "$work/connector-before.json"
git bundle create "$work/source.bundle" "$branch"
git bundle verify "$work/source.bundle" > "$local_dir/bundle-verify.txt"
PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3d_transaction_v1.py create-freeze \
  --source-commit "$head" --source-tree "$head_tree" --bundle "$work/source.bundle" \
  > "$work/implementation-freeze.json"
jq -e '.schema == "nando.s1c3d-implementation-freeze.v1"' \
  "$work/implementation-freeze.json" >/dev/null

ssh "$remote" "rm -rf '$remote_upload'; install -d -m 0700 '$remote_upload'"
scp -q "$work/source.bundle" "$work/implementation-freeze.json" \
  "$work/connector-before.json" "$candidate_config" "$parity_source" "$oracle_lock" \
  "${implementation_files[@]}" "${dependency_files[@]}" \
  "$remote:$remote_upload/"

set +e
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3d_transaction_v1.py' prepare \
    --transaction-id '$transaction_id' \
    --transaction-directory '$remote_transaction' \
    --bundle '$remote_upload/source.bundle' \
    --candidate-config '$remote_upload/$(basename "$candidate_config")' \
    --parity-source '$remote_upload/$(basename "$parity_source")' \
    --oracle-lock '$remote_upload/$(basename "$oracle_lock")' \
    --connector-before '$remote_upload/connector-before.json' \
    --implementation-freeze '$remote_upload/implementation-freeze.json'" \
  > "$local_dir/prepare-result.json" 2> "$local_dir/prepare-error.json"
prepare_code=$?
set -e
rollback_armed=true
trap emergency_rollback EXIT INT TERM HUP
mirror_remote
cp "$work/connector-before.json" "$local_dir/connector-before.json"
if [[ $prepare_code -ne 0 ]]; then
  state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'" 2>/dev/null || printf UNKNOWN)
  if [[ $state == RESOURCE_VETO ]]; then
    seal_resource_veto
    rollback_armed=false
    trap cleanup EXIT
    verdict=$(jq -er .verdict "$local_dir/remote-mirror/s1c3d-state.json")
    printf 'transaction_directory=%s\nlocal_evidence=%s\n' "$remote_transaction" "$local_dir"
    printf 'verdict=%s production_mutation=no\n' "$verdict"
    exit 3
  fi
  abort_predeployment mechanism_prepare_failed
  printf 'transaction_directory=%s\nlocal_evidence=%s\n' "$remote_transaction" "$local_dir"
  printf 'verdict=S1C3D_PREFLIGHT_FAILURE production_mutation=no\n'
  exit 3
fi

PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3d_transaction_v1.py verify \
  "$local_dir/remote-mirror" \
  --implementation-freeze "$local_dir/remote-mirror/implementation-freeze.json" \
  --pre-deployment > "$local_dir/s1c3d-predeployment.local.json"
scp -q "$local_dir/s1c3d-predeployment.local.json" \
  "$remote:$remote_upload/predeployment-verification.json"
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3d_transaction_v1.py' verify \
    '$remote_transaction' --implementation-freeze '$remote_transaction/implementation-freeze.json' \
    --recorded-verification '$remote_upload/predeployment-verification.json' --pre-deployment" \
  > "$local_dir/s1c3d-predeployment.remote.json"
cmp "$local_dir/s1c3d-predeployment.local.json" "$local_dir/s1c3d-predeployment.remote.json"
jq -e '.valid == true and .authority == true and .scientific_authority == false and
  (.verdict == "S1C3D_PREPARATION_PASS" or
   .verdict == "S1C3D_PREPARATION_PASS_WITH_OPTIMIZATION_WATCH")' \
  "$local_dir/s1c3d-predeployment.local.json" >/dev/null

scp -q "$local_dir/s1c3d-predeployment.local.json" \
  "$remote:$remote_upload/predeployment-envelope.json"
set +e
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3d_transaction_v1.py' execute \
    --transaction-directory '$remote_transaction' \
    --predeployment-verification '$remote_upload/predeployment-verification.json' \
    --authority-envelope '$remote_upload/predeployment-envelope.json'" \
  > "$local_dir/execute-result.json" 2> "$local_dir/execute-error.json"
execute_code=$?
set -e

state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'")
if [[ $state == PREPARED ]]; then
  rollback_armed=false
  abort_predeployment execute_failed_before_mutation
  printf 'verdict=S1C3D_PREFLIGHT_FAILURE execute_code=%s production_mutation=no\n' "$execute_code"
  exit 3
fi
if [[ $state == ROLLBACK_ARMED ]]; then
  ssh "$remote" \
    "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3d_transaction_v1.py' rollback \
      --transaction-directory '$remote_transaction' --reason execute_session_aborted" \
    > "$local_dir/recovery-rollback.json"
  state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'")
fi
[[ $state == FINALIZE_PENDING || $state == ROLLBACK_PENDING ]]

connector_snapshot after "$work/connector-after.json"
scp -q "$work/connector-after.json" "$remote:$remote_upload/connector-after.json"
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3d_transaction_v1.py' finalize \
    --transaction-directory '$remote_transaction' \
    --connector-after '$remote_upload/connector-after.json'" \
  > "$local_dir/finalize-result.json"
mirror_remote
cp "$work/connector-after.json" "$local_dir/connector-after.json"

PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3d_transaction_v1.py verify \
  "$local_dir/remote-mirror" \
  --implementation-freeze "$local_dir/remote-mirror/implementation-freeze.json" \
  > "$local_dir/s1c3d-final.local.json"
scp -q "$local_dir/s1c3d-final.local.json" "$remote:$remote_upload/final-verification.json"
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3d_transaction_v1.py' verify \
    '$remote_transaction' --implementation-freeze '$remote_transaction/implementation-freeze.json' \
    --recorded-verification '$remote_upload/final-verification.json'" \
  > "$local_dir/s1c3d-final.remote.json"
cmp "$local_dir/s1c3d-final.local.json" "$local_dir/s1c3d-final.remote.json"
scp -q "$local_dir/s1c3d-final.local.json" "$remote:$remote_upload/final-envelope.json"
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3d_transaction_v1.py' seal \
    --transaction-directory '$remote_transaction' \
    --final-verification '$remote_upload/final-verification.json' \
    --authority-envelope '$remote_upload/final-envelope.json'" \
  > "$local_dir/seal-result.json"

rollback_armed=false
trap cleanup EXIT
mirror_remote
verdict=$(jq -er .verdict "$local_dir/remote-mirror/s1c3d-authority-envelope.json")
printf 'transaction_directory=%s\nlocal_evidence=%s\n' "$remote_transaction" "$local_dir"
printf 'execute_code=%s verdict=%s\n' "$execute_code" "$verdict"
[[ $verdict == S1C3D_DEPLOYMENT_PASS || \
   $verdict == S1C3D_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH ]]
