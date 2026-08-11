#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

paper_commit=b3ee186d49d848b1917472f427d6afc59459c7cd
candidate_commit=a3ea27a49af397ef79e5c9ec80089ecf53a41d59
baseline_commit=663959064a37caf7eb917fc99dfedb6386355fa6
paper_manifest_root=ebb5067060f69722341120ae8105849cbd45f585611a30741e1db7d33ace3ab3
candidate_config=plans/effect-law-unification-v1/evidence/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1/transition-serving.env.candidate
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
[[ $(git rev-parse "$candidate_commit^{tree}") == 670d9c4ed170a76f107db13262abcd7cc035578e ]] || {
  printf 'candidate_tree_drift\n' >&2
  exit 2
}
[[ $(git rev-parse "$baseline_commit^{tree}") == 05460ccbc9c44ac8b7174318903c0211de709e2e ]] || {
  printf 'baseline_tree_drift\n' >&2
  exit 2
}
[[ $(sha256sum "$candidate_config" | awk '{print $1}') == 1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6 ]] || {
  printf 'candidate_config_drift\n' >&2
  exit 2
}
[[ $(sha256sum plans/effect-law-unification-v1/evidence/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1/SHA256SUMS | awk '{print $1}') == "$paper_manifest_root" ]] || {
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
transaction_id="${timestamp}-${paper_commit:0:12}-s1c3"
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
  ops/remote-backend/s1c3_remote_transaction_v1.py \
  ops/remote-backend/verify_s1c3_transaction_v1.py \
  "$candidate_config" \
  "$parity_source" \
  "$work/connector-before.json" \
  "$remote:$remote_upload/"

ssh "$remote" \
  "sudo -n python3 '$remote_upload/s1c3_remote_transaction_v1.py' prepare \
    --transaction-id '$transaction_id' \
    --transaction-directory '$remote_transaction' \
    --bundle '$remote_upload/source.bundle' \
    --candidate-config '$remote_upload/transition-serving.env.candidate' \
    --parity-source '$remote_upload/main.rs' \
    --connector-before '$remote_upload/connector-before.json'" \
  | tee "$local_dir/prepare-result.json"

rollback_armed=true
emergency_rollback() {
  local code=$?
  if [[ $rollback_armed == true ]]; then
    ssh "$remote" \
      "sudo -n python3 '$remote_upload/s1c3_remote_transaction_v1.py' rollback \
        --transaction-directory '$remote_transaction' --reason local_orchestrator_interrupted" \
      > "$local_dir/emergency-rollback.json" 2>&1 || true
  fi
  exit "$code"
}
trap emergency_rollback INT TERM HUP

set +e
ssh "$remote" \
  "sudo -n python3 '$remote_upload/s1c3_remote_transaction_v1.py' execute \
    --transaction-directory '$remote_transaction'" \
  > "$local_dir/execute-result.json" 2> "$local_dir/execute-error.json"
execute_code=$?
set -e

state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'")
if [[ $state == ROLLBACK_ARMED ]]; then
  set +e
  ssh "$remote" \
    "sudo -n python3 '$remote_upload/s1c3_remote_transaction_v1.py' rollback \
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
  "sudo -n python3 '$remote_upload/s1c3_remote_transaction_v1.py' finalize \
    --transaction-directory '$remote_transaction' \
    --connector-after '$remote_upload/connector-after.json'" \
  > "$local_dir/finalize-result.json" 2> "$local_dir/finalize-error.json"
finalize_code=$?
set -e
rollback_armed=false

for name in preparation.json resource-receipt.json parity-receipt.json deployment-receipt.json transaction-state.json; do
  ssh "$remote" "sudo -n cat '$remote_transaction/$name'" > "$local_dir/$name"
done
cp "$work/connector-before.json" "$local_dir/connector-before.json"
cp "$work/connector-after.json" "$local_dir/connector-after.json"

set +e
python3 ops/remote-backend/verify_s1c3_transaction_v1.py "$local_dir" \
  > "$local_dir/local-verification.json"
verify_code=$?
set -e

printf 'transaction_directory=%s\n' "$remote_transaction"
printf 'local_evidence=%s\n' "$local_dir"
printf 'execute_code=%s finalize_code=%s verify_code=%s\n' "$execute_code" "$finalize_code" "$verify_code"
if [[ $finalize_code -ne 0 || $verify_code -ne 0 ]]; then
  exit 3
fi
