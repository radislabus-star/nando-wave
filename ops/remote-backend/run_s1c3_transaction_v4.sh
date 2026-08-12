#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

paper_commit=1def4272a46641f2c72a9c0efbd5818f93caa655
candidate_commit=03e3dd00c90206e2f705371318c50dd50537d6d8
baseline_commit=663959064a37caf7eb917fc99dfedb6386355fa6
production_projection=crates/nando-transition-serving/src/grounded_decision_capture.rs
production_projection_sha256=10b2856687c0e22c47e43754d2a05ffa82641002b11d70d42edca1e4c797c316
paper_manifest_root=805a93477295172dfae83d5dd91f68659d0c19fb28bb5c40aa00fb59beab48e0
candidate_config=plans/effect-law-unification-v1/evidence/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4/transition-serving.env.candidate
parity_source=ops/remote-backend/s1c3-parity-oracle/main.rs
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
[[ $(git rev-parse "$candidate_commit^{tree}") == 06a9df51797dffc127fec41672bddae29c38bb92 ]] || {
  printf 'candidate_tree_drift\n' >&2
  exit 2
}
[[ $(git rev-parse "$baseline_commit^{tree}") == 05460ccbc9c44ac8b7174318903c0211de709e2e ]] || {
  printf 'baseline_tree_drift\n' >&2
  exit 2
}
[[ $(git show "$candidate_commit:$production_projection" \
  | sed -n '1,/^#\[cfg(test)\]/p' \
  | sha256sum \
  | awk '{print $1}') == "$production_projection_sha256" ]] || {
  printf 'candidate_production_projection_drift\n' >&2
  exit 2
}
[[ $(sha256sum "$candidate_config" | awk '{print $1}') == 1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6 ]] || {
  printf 'candidate_config_drift\n' >&2
  exit 2
}
[[ $(sha256sum plans/effect-law-unification-v1/evidence/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4/SHA256SUMS | awk '{print $1}') == "$paper_manifest_root" ]] || {
  printf 'paper_manifest_drift\n' >&2
  exit 2
}
[[ -z $(git status --porcelain --untracked-files=no) ]] || {
  printf 'tracked_worktree_dirty\n' >&2
  exit 2
}
remote_head=$(git ls-remote origin "refs/heads/$branch" | awk '{print $1}')
[[ $remote_head == "$head" ]] || {
  printf 'implementation_head_not_pushed local=%s remote=%s\n' "$head" "$remote_head" >&2
  exit 2
}

timestamp=$(date -u +%Y%m%dT%H%M%SZ)
transaction_id="${timestamp}-${paper_commit:0:12}-s1c3v4"
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
    --arg schema nando.s1c3-connector-snapshot.v1 \
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

connector_snapshot before "$work/connector-before.json"
git bundle create "$work/source.bundle" "$branch"
git bundle verify "$work/source.bundle" > "$local_dir/bundle-verify.txt"

ssh "$remote" "set -e; test ! -e '$remote_upload'; install -d -m 0700 '$remote_upload'"
scp -q \
  "$work/source.bundle" \
  ops/remote-backend/s1c3_remote_transaction_v4.py \
  ops/remote-backend/verify_s1c3_transaction_v4.py \
  "$candidate_config" \
  "$parity_source" \
  "$work/connector-before.json" \
  "$remote:$remote_upload/"

set +e
ssh "$remote" \
  "sudo -n python3 '$remote_upload/s1c3_remote_transaction_v4.py' prepare \
    --transaction-id '$transaction_id' \
    --transaction-directory '$remote_transaction' \
    --bundle '$remote_upload/source.bundle' \
    --candidate-config '$remote_upload/transition-serving.env.candidate' \
    --parity-source '$remote_upload/main.rs' \
    --connector-before '$remote_upload/connector-before.json'" \
  > "$local_dir/prepare-result.json" 2> "$local_dir/prepare-error.json"
prepare_code=$?
set -e

if [[ $prepare_code -ne 0 ]]; then
  connector_snapshot after "$work/connector-after.json"
  ssh "$remote" "sudo -n tar -C '$remote_transaction' -cf - ." \
    | tar -C "$local_dir" -xf -
  cp "$work/connector-before.json" "$local_dir/connector-before.json"
  cp "$work/connector-after.json" "$local_dir/connector-after.json"
  printf 'transaction_directory=%s\n' "$remote_transaction"
  printf 'local_evidence=%s\n' "$local_dir"
  printf 'prepare_code=%s production_mutation=no\n' "$prepare_code"
  exit 3
fi

rollback_armed=true
emergency_rollback() {
  local code=$?
  if [[ $rollback_armed == true ]]; then
    ssh "$remote" \
      "sudo -n python3 '$remote_upload/s1c3_remote_transaction_v4.py' rollback \
        --transaction-directory '$remote_transaction' --reason local_orchestrator_interrupted" \
      > "$local_dir/emergency-rollback.json" 2>&1 || true
  fi
  exit "$code"
}
trap emergency_rollback INT TERM HUP

set +e
ssh "$remote" \
  "sudo -n python3 '$remote_upload/s1c3_remote_transaction_v4.py' execute \
    --transaction-directory '$remote_transaction'" \
  > "$local_dir/execute-result.json" 2> "$local_dir/execute-error.json"
execute_code=$?
set -e

state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'")
if [[ $state == ROLLBACK_ARMED ]]; then
  set +e
  ssh "$remote" \
    "sudo -n python3 '$remote_upload/s1c3_remote_transaction_v4.py' rollback \
      --transaction-directory '$remote_transaction' --reason execute_session_aborted" \
    > "$local_dir/recovery-rollback.json" 2>&1
  set -e
  state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'")
fi

connector_snapshot after "$work/connector-after.json"
scp -q "$work/connector-after.json" "$remote:$remote_upload/connector-after.json"

if [[ $state != FINALIZE_PENDING && $state != ROLLBACK_PENDING ]]; then
  rollback_armed=false
  printf 'transaction_not_finalizable state=%s execute_code=%s\n' "$state" "$execute_code" >&2
  exit 2
fi

set +e
ssh "$remote" \
  "sudo -n python3 '$remote_upload/s1c3_remote_transaction_v4.py' finalize \
    --transaction-directory '$remote_transaction' \
    --connector-after '$remote_upload/connector-after.json'" \
  > "$local_dir/finalize-result.json" 2> "$local_dir/finalize-error.json"
finalize_code=$?
set -e
rollback_armed=false

for name in oracle-ownership-receipt.json quiescence-receipt.json measurement-contamination-receipt.json preparation.json resource-receipt.json parity-receipt.json deployment-receipt.json transaction-state.json; do
  ssh "$remote" "sudo -n cat '$remote_transaction/$name'" > "$local_dir/$name"
  if [[ $name == oracle-ownership-receipt.json || $name == quiescence-receipt.json ]]; then
    chmod 0400 "$local_dir/$name"
  fi
done
cp "$work/connector-before.json" "$local_dir/connector-before.json"
cp "$work/connector-after.json" "$local_dir/connector-after.json"

set +e
python3 ops/remote-backend/verify_s1c3_transaction_v4.py "$local_dir" --allow-rollback \
  > "$local_dir/local-verification.json"
verify_code=$?
set -e

printf 'transaction_directory=%s\n' "$remote_transaction"
printf 'local_evidence=%s\n' "$local_dir"
printf 'execute_code=%s finalize_code=%s verify_code=%s\n' "$execute_code" "$finalize_code" "$verify_code"
if [[ $finalize_code -ne 0 || $verify_code -ne 0 ]]; then
  exit 3
fi
