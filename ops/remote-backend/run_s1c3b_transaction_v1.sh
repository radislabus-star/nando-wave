#!/usr/bin/env bash
# shellcheck disable=SC2029
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

paper_commit=36ffc0cbf56b72b2c07ff97c83bb5ac271ed5189
paper_tree=1f8a9c7fc0cdd572a3adf7ba4a7ad294b47031ad
paper_manifest_root=3b98c93828d5260397365373e742542869b2419f050c3d08a21947cbb207e5b6
paper_verification_sha256=6c5f87233fadbfac03671dab7f5a652d1597618434efa4fd16bfddb874ec8e26
candidate_commit=03e3dd00c90206e2f705371318c50dd50537d6d8
candidate_tree=06a9df51797dffc127fec41672bddae29c38bb92
production_projection=crates/nando-transition-serving/src/grounded_decision_capture.rs
production_projection_sha256=10b2856687c0e22c47e43754d2a05ffa82641002b11d70d42edca1e4c797c316
candidate_config=plans/effect-law-unification-v1/evidence/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V7/transition-serving.env.candidate
paper_manifest=plans/effect-law-unification-v1/evidence/S1C3B_PRODUCTION_LOAD_ABSOLUTE_GATE_V1/SHA256SUMS
paper_verification=plans/effect-law-unification-v1/S1C3B_PRODUCTION_LOAD_ABSOLUTE_GATE_PAPER_VERIFICATION_2026-08-12.md
parity_source=ops/remote-backend/s1c3-parity-oracle/main.rs
oracle_lock=plans/effect-law-unification-v1/evidence/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V7/oracle.Cargo.lock
remote=e@192.168.3.94

usage() {
  printf 'usage: %s OUTPUT_PARENT\n' "$0" >&2
  exit 2
}

[[ $# -eq 1 ]] || usage
output_parent=$1
branch=$(git branch --show-current)
head=$(git rev-parse HEAD)

git merge-base --is-ancestor "$paper_commit" "$head" || {
  printf 'paper_commit_not_ancestor\n' >&2
  exit 2
}
[[ $(git rev-parse "$paper_commit^{tree}") == "$paper_tree" ]] || {
  printf 'paper_tree_drift\n' >&2
  exit 2
}
[[ $(git rev-parse "$candidate_commit^{tree}") == "$candidate_tree" ]] || {
  printf 'candidate_tree_drift\n' >&2
  exit 2
}
[[ $(git show "$candidate_commit:$production_projection" \
  | sed -n '1,/^#\[cfg(test)\]/p' \
  | sha256sum | awk '{print $1}') == "$production_projection_sha256" ]] || {
  printf 'candidate_production_projection_drift\n' >&2
  exit 2
}
[[ $(sha256sum "$paper_manifest" | awk '{print $1}') == "$paper_manifest_root" ]] || {
  printf 'paper_manifest_drift\n' >&2
  exit 2
}
[[ $(sha256sum "$paper_verification" | awk '{print $1}') == "$paper_verification_sha256" ]] || {
  printf 'paper_verification_drift\n' >&2
  exit 2
}
[[ $(sha256sum "$candidate_config" | awk '{print $1}') == 1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6 ]] || {
  printf 'candidate_config_drift\n' >&2
  exit 2
}
[[ $(sha256sum "$parity_source" | awk '{print $1}') == bc5a2255de62a05b44be677ba67331cfbf47b884557f8d8a0d3ac06e46c64b26 ]] || {
  printf 'parity_source_drift\n' >&2
  exit 2
}
[[ $(sha256sum "$oracle_lock" | awk '{print $1}') == 498855d2a867ba80dba55ad1609bf937852aa61e9de97203d26f067a619da32b ]] || {
  printf 'oracle_lock_drift\n' >&2
  exit 2
}
[[ -z $(git status --porcelain --untracked-files=no) ]] || {
  printf 'tracked_worktree_dirty\n' >&2
  exit 2
}
implementation_files=(
  ops/remote-backend/run_s1c3b_transaction_v1.sh
  ops/remote-backend/s1c3b_remote_transaction_v1.py
  ops/remote-backend/verify_s1c3b_transaction_v1.py
  ops/remote-backend/test_verify_s1c3b_transaction_v1.py
)
for implementation_file in "${implementation_files[@]}"; do
  git cat-file -e "HEAD:$implementation_file" || {
    printf 'implementation_file_not_committed=%s\n' "$implementation_file" >&2
    exit 2
  }
  [[ $(git show "HEAD:$implementation_file" | sha256sum | awk '{print $1}') == \
      $(sha256sum "$implementation_file" | awk '{print $1}') ]] || {
    printf 'implementation_file_differs_from_head=%s\n' "$implementation_file" >&2
    exit 2
  }
done
remote_head=$(git ls-remote origin "refs/heads/$branch" | awk '{print $1}')
[[ $remote_head == "$head" ]] || {
  printf 'implementation_head_not_pushed local=%s remote=%s\n' "$head" "$remote_head" >&2
  exit 2
}

timestamp=$(date -u +%Y%m%dT%H%M%SZ)
transaction_id="${timestamp}-${paper_commit:0:12}-s1c3b-v1"
local_dir="$output_parent/$transaction_id"
remote_upload="/home/e/.cache/${transaction_id}-upload"
remote_transaction="/var/lib/nando-wave/deployments/$transaction_id"
[[ ! -e $local_dir ]] || {
  printf 'local_transaction_directory_exists=%s\n' "$local_dir" >&2
  exit 2
}
install -d -m 0700 "$local_dir"
work=$(mktemp -d)
cleanup() {
  rm -rf "$work"
}
trap cleanup EXIT

prior_attempts=$(ssh "$remote" \
  "set -o pipefail; sudo -n find /var/lib/nando-wave/deployments -mindepth 1 -maxdepth 1 \
    -type d -name '*-${paper_commit:0:12}-s1c3b-v1' -print | wc -l")
[[ $prior_attempts == 0 ]] || {
  printf 's1c3b_attempt_already_exists count=%s\n' "$prior_attempts" >&2
  exit 2
}

connector_snapshot() {
  local label=$1
  local destination=$2
  local pid active nrestarts route_failures command_sha
  active=$(systemctl --user show nando-client-connector.service -p ActiveState --value)
  pid=$(systemctl --user show nando-client-connector.service -p MainPID --value)
  nrestarts=$(systemctl --user show nando-client-connector.service -p NRestarts --value)
  route_failures=$(curl -fsS --max-time 4 http://127.0.0.1:18786/metrics | jq -er '.route_receipt_failures')
  command_sha=$(tr '\0' ' ' < "/proc/$pid/cmdline" | sha256sum | awk '{print $1}')
  jq -nS \
    --arg schema nando.s1c3b-connector-snapshot.v1 \
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
  ssh "$remote" "sudo -n tar -C '$remote_transaction' -cf - ." \
    | tar -C "$local_dir" -xf -
}

connector_snapshot before "$work/connector-before.json"
git bundle create "$work/source.bundle" "$branch"
git bundle verify "$work/source.bundle" > "$local_dir/bundle-verify.txt"

ssh "$remote" "set -e; test ! -e '$remote_upload'; install -d -m 0700 '$remote_upload'"
scp -q \
  "$work/source.bundle" \
  ops/remote-backend/s1c3_remote_transaction_v7.py \
  ops/remote-backend/verify_s1c3_transaction_v7.py \
  ops/remote-backend/s1c3b_remote_transaction_v1.py \
  ops/remote-backend/verify_s1c3b_transaction_v1.py \
  "$candidate_config" \
  "$parity_source" \
  "$oracle_lock" \
  "$work/connector-before.json" \
  "$remote:$remote_upload/"

set +e
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3b_remote_transaction_v1.py' prepare \
    --transaction-id '$transaction_id' \
    --transaction-directory '$remote_transaction' \
    --bundle '$remote_upload/source.bundle' \
    --candidate-config '$remote_upload/transition-serving.env.candidate' \
    --parity-source '$remote_upload/main.rs' \
    --oracle-lock '$remote_upload/oracle.Cargo.lock' \
    --connector-before '$remote_upload/connector-before.json'" \
  > "$local_dir/prepare-result.json" 2> "$local_dir/prepare-error.json"
prepare_code=$?
set -e

if [[ $prepare_code -ne 0 ]]; then
  connector_snapshot after "$work/connector-after.json"
  mirror_remote
  cp "$work/connector-before.json" "$local_dir/connector-before.json"
  cp "$work/connector-after.json" "$local_dir/connector-after.json"
  state=UNKNOWN
  [[ ! -f $local_dir/transaction-state.json ]] || state=$(jq -r .state "$local_dir/transaction-state.json")
  if [[ $state == RESOURCE_VETO ]]; then
    PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3b_transaction_v1.py "$local_dir" \
      > "$local_dir/local-verification.json"
    jq -e '.valid == true and .authority == false and .verdict == "S1C3B_RESOURCE_VETO"' \
      "$local_dir/local-verification.json" >/dev/null
    printf 'transaction_directory=%s\nlocal_evidence=%s\n' "$remote_transaction" "$local_dir"
    printf 'verdict=S1C3B_RESOURCE_VETO production_mutation=no\n'
    exit 3
  fi
  printf 'transaction_directory=%s\nlocal_evidence=%s\n' "$remote_transaction" "$local_dir"
  printf 'prepare_code=%s state=%s production_mutation=no\n' "$prepare_code" "$state"
  exit 3
fi

set +e
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3b_transaction_v1.py' \
    '$remote_transaction' --pre-deployment" \
  > "$local_dir/pre-deployment-verification.json" \
  2> "$local_dir/pre-deployment-verification.error"
predeploy_code=$?
set -e
if [[ $predeploy_code -ne 0 ]]; then
  connector_snapshot after "$work/connector-after.json"
  mirror_remote
  cp "$work/connector-before.json" "$local_dir/connector-before.json"
  cp "$work/connector-after.json" "$local_dir/connector-after.json"
  printf 'predeployment_verification=%s production_mutation=no\n' "$predeploy_code"
  exit 3
fi
jq -e '.valid == true and .authority == true and .verdict == "S1C3B_PREPARATION_PASS"' \
  "$local_dir/pre-deployment-verification.json" >/dev/null

mirror_remote
cp "$work/connector-before.json" "$local_dir/connector-before.json"
PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3b_transaction_v1.py \
  "$local_dir" --pre-deployment > "$local_dir/local-pre-deployment-verification.json"
cmp "$local_dir/pre-deployment-verification.json" "$local_dir/local-pre-deployment-verification.json"
scp -q "$local_dir/pre-deployment-verification.json" \
  "$remote:$remote_upload/predeployment-verification.json"

rollback_armed=true
emergency_rollback() {
  local code=$?
  trap - EXIT INT TERM HUP
  set +e
  if [[ $rollback_armed == true ]]; then
    rollback_armed=false
    local state
    state=$(ssh "$remote" \
      "sudo -n jq -r .state '$remote_transaction/transaction-state.json'" 2>/dev/null)
    if [[ $state == ROLLBACK_ARMED || $state == FINALIZE_PENDING || \
          $state == FINAL_VERIFICATION_PENDING ]]; then
      ssh "$remote" \
        "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3b_remote_transaction_v1.py' rollback \
          --transaction-directory '$remote_transaction' --reason local_orchestrator_interrupted" \
        > "$local_dir/emergency-rollback.json" 2>&1
      state=$(ssh "$remote" \
        "sudo -n jq -r .state '$remote_transaction/transaction-state.json'" 2>/dev/null)
    fi
    if [[ $state == ROLLBACK_PENDING ]]; then
      connector_snapshot after "$work/emergency-connector-after.json"
      scp -q "$work/emergency-connector-after.json" \
        "$remote:$remote_upload/emergency-connector-after.json"
      ssh "$remote" \
        "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3b_remote_transaction_v1.py' finalize \
          --transaction-directory '$remote_transaction' \
          --connector-after '$remote_upload/emergency-connector-after.json'" \
        > "$local_dir/emergency-finalize.json" 2>&1
      ssh "$remote" \
        "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3b_transaction_v1.py' \
          '$remote_transaction' --allow-rollback > '$remote_upload/emergency-final-verification.json' && \
         sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3b_remote_transaction_v1.py' seal \
          --transaction-directory '$remote_transaction' \
          --final-verification '$remote_upload/emergency-final-verification.json'" \
        > "$local_dir/emergency-seal.json" 2>&1
    fi
  fi
  cleanup
  exit "$code"
}
trap emergency_rollback EXIT INT TERM HUP

set +e
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3b_remote_transaction_v1.py' execute \
    --transaction-directory '$remote_transaction' \
    --predeployment-verification '$remote_upload/predeployment-verification.json'" \
  > "$local_dir/execute-result.json" 2> "$local_dir/execute-error.json"
execute_code=$?
set -e

state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'")
if [[ $state == ROLLBACK_ARMED ]]; then
  set +e
  ssh "$remote" \
    "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3b_remote_transaction_v1.py' rollback \
      --transaction-directory '$remote_transaction' --reason execute_session_aborted" \
    > "$local_dir/recovery-rollback.json" 2>&1
  set -e
  state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'")
fi

connector_snapshot after "$work/connector-after.json"
scp -q "$work/connector-after.json" "$remote:$remote_upload/connector-after.json"

if [[ $state != FINALIZE_PENDING && $state != ROLLBACK_PENDING ]]; then
  printf 'transaction_not_finalizable state=%s execute_code=%s\n' "$state" "$execute_code" >&2
  exit 2
fi

set +e
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3b_remote_transaction_v1.py' finalize \
    --transaction-directory '$remote_transaction' \
    --connector-after '$remote_upload/connector-after.json'" \
  > "$local_dir/finalize-result.json" 2> "$local_dir/finalize-error.json"
finalize_code=$?
set -e

rm -rf "$local_dir/evidence" "$local_dir/rollback"
mirror_remote
cp "$work/connector-before.json" "$local_dir/connector-before.json"
cp "$work/connector-after.json" "$local_dir/connector-after.json"

set +e
PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3b_transaction_v1.py \
  "$local_dir" --allow-rollback > "$local_dir/local-verification.json"
verify_code=$?
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3b_transaction_v1.py' \
    '$remote_transaction' --allow-rollback" \
  > "$local_dir/remote-verification.json"
remote_verify_code=$?
set -e

final_verdict=UNVERIFIED
if [[ $finalize_code -eq 0 && $verify_code -eq 0 && $remote_verify_code -eq 0 ]]; then
  cmp "$local_dir/local-verification.json" "$local_dir/remote-verification.json"
  scp -q "$local_dir/local-verification.json" \
    "$remote:$remote_upload/final-verification.json"
  ssh "$remote" \
    "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3b_remote_transaction_v1.py' seal \
      --transaction-directory '$remote_transaction' \
      --final-verification '$remote_upload/final-verification.json'" \
    > "$local_dir/seal-result.json"
  [[ $(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'") \
      == COMPLETE ]]
  rollback_armed=false
  rm -rf "$local_dir/evidence" "$local_dir/rollback"
  mirror_remote
  final_verdict=$(jq -er .verdict "$local_dir/local-verification.json")
fi

printf 'transaction_directory=%s\nlocal_evidence=%s\n' "$remote_transaction" "$local_dir"
printf 'execute_code=%s finalize_code=%s verify_code=%s remote_verify_code=%s\n' \
  "$execute_code" "$finalize_code" "$verify_code" "$remote_verify_code"
printf 'verdict=%s\n' "$final_verdict"
if [[ $finalize_code -ne 0 || $verify_code -ne 0 || $remote_verify_code -ne 0 || \
      $final_verdict != S1C3B_DEPLOYMENT_PASS ]]; then
  exit 3
fi
