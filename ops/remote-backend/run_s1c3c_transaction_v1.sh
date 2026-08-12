#!/usr/bin/env bash
# shellcheck disable=SC2029
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

paper_commit=2a1505055ce98b3f6bed5cb440a0faa345fb78cb
paper_tree=68a0dff858e5b49445997f09d17cc52d22e12511
paper_preregistration=plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_PREREGISTRATION_V1.md
paper_preregistration_sha256=d56289d4d67600786fe08c5e8d5478448b75bb1b9aeba9c0291da20d4a192492
paper_critique=plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_CRITIQUE_V1.md
paper_critique_sha256=2e34b55fccb0dadceec1e97bc9a4880d282308243bf9abb4faf418c6e81b2ff6
paper_verification=plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_PAPER_VERIFICATION_2026-08-12.md
paper_verification_sha256=cfa0e6cdb4176fb3f191d1f32afa28d4469505db76bffad0d6ad95d4f46b1ff2
paper_manifest=plans/effect-law-unification-v1/evidence/S1C3C_CAPTURE_INSTALLATION_PAPER_V1/SHA256SUMS
paper_manifest_sha256=913eefbb6a021fcedb53b5a788bc5369394c204fec6c5c5ab0077a1d04f08bfe
candidate_commit=03e3dd00c90206e2f705371318c50dd50537d6d8
candidate_tree=06a9df51797dffc127fec41672bddae29c38bb92
production_projection=crates/nando-transition-serving/src/grounded_decision_capture.rs
production_projection_sha256=10b2856687c0e22c47e43754d2a05ffa82641002b11d70d42edca1e4c797c316
candidate_config=plans/effect-law-unification-v1/evidence/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V7/transition-serving.env.candidate
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
head_tree=$(git rev-parse 'HEAD^{tree}')

verify_file() {
  local path=$1 expected=$2 label=$3
  [[ $(sha256sum "$path" | awk '{print $1}') == "$expected" ]] || {
    printf '%s_drift=%s\n' "$label" "$path" >&2
    exit 2
  }
}

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
verify_file "$paper_preregistration" "$paper_preregistration_sha256" paper_preregistration
verify_file "$paper_critique" "$paper_critique_sha256" paper_critique
verify_file "$paper_verification" "$paper_verification_sha256" paper_verification
verify_file "$paper_manifest" "$paper_manifest_sha256" paper_manifest
verify_file "$candidate_config" 1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6 candidate_config
verify_file "$parity_source" bc5a2255de62a05b44be677ba67331cfbf47b884557f8d8a0d3ac06e46c64b26 parity_source
verify_file "$oracle_lock" 498855d2a867ba80dba55ad1609bf937852aa61e9de97203d26f067a619da32b oracle_lock
verify_file ops/remote-backend/s1c3b_remote_transaction_v1.py 74fde9997bb14f4064aa01303cc67cd79e2dea826f39bfc50850e49394b70523 mechanism_executor
verify_file ops/remote-backend/verify_s1c3b_transaction_v1.py 72e29c6f52e3e29648a7f1bf13cc66b02ad5f0fe68db07cd9dbfa54ff86561dd mechanism_verifier
verify_file ops/remote-backend/s1c3_remote_transaction_v7.py d0a490d93cc5dbd488119d7cc721de0cf9609ab5d97c87efb8a1de69916ab971 legacy_executor
verify_file ops/remote-backend/verify_s1c3_transaction_v7.py 8e383844e7d945cd94829dfd772a68fffcb89457556a30edb41bb42b615162bc legacy_verifier

[[ -z $(git status --porcelain --untracked-files=no) ]] || {
  printf 'tracked_worktree_dirty\n' >&2
  exit 2
}
implementation_files=(
  ops/remote-backend/run_s1c3c_transaction_v1.sh
  ops/remote-backend/s1c3c_schema_preflight_v1.py
  ops/remote-backend/s1c3c_transaction_v1.py
  ops/remote-backend/verify_s1c3c_transaction_v1.py
  ops/remote-backend/test_s1c3c_transaction_v1.py
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
# This pure gate must precede timestamp creation, directories, SSH and locks.
schema_preflight=$(PYTHONPATH=ops/remote-backend \
  python3 ops/remote-backend/s1c3c_schema_preflight_v1.py)
jq -e '.valid == true and .authority == false and .side_effects == false and
  .remote_attempt_created == false and (.metric_families | length) == 4' \
  <<<"$schema_preflight" >/dev/null

remote_head=$(git ls-remote origin "refs/heads/$branch" | awk '{print $1}')
[[ $remote_head == "$head" ]] || {
  printf 'implementation_head_not_pushed local=%s remote=%s\n' "$head" "$remote_head" >&2
  exit 2
}

timestamp=$(date -u +%Y%m%dT%H%M%SZ)
transaction_id="${timestamp}-${paper_commit:0:12}-s1c3c-v1"
local_dir="$output_parent/$transaction_id"
remote_upload="/home/e/.cache/${transaction_id}-upload"
remote_transaction="/var/lib/nando-wave/deployments/$transaction_id"
[[ ! -e $local_dir ]] || {
  printf 'local_transaction_directory_exists=%s\n' "$local_dir" >&2
  exit 2
}
install -d -m 0700 "$local_dir"
printf '%s\n' "$schema_preflight" > "$local_dir/schema-preflight.json"
chmod 0400 "$local_dir/schema-preflight.json"
work=$(mktemp -d)
cleanup() {
  rm -rf "$work"
}
trap cleanup EXIT

prior_attempts=$(ssh "$remote" \
  "set -o pipefail; sudo -n find /var/lib/nando-wave/deployments -mindepth 1 -maxdepth 1 \
    -type d -name '*-${paper_commit:0:12}-s1c3c-v1' -print | wc -l")
[[ $prior_attempts == 0 ]] || {
  printf 's1c3c_attempt_already_exists count=%s\n' "$prior_attempts" >&2
  exit 2
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
PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3c_transaction_v1.py freeze \
  --source-commit "$head" \
  --source-tree "$head_tree" \
  --bundle "$work/source.bundle" \
  --implementation-directory ops/remote-backend \
  > "$local_dir/implementation-freeze.json"
jq -e '.schema == "nando.s1c3c-implementation-freeze.v1" and
  (.implementation_files | length) == 5' "$local_dir/implementation-freeze.json" >/dev/null
chmod 0400 "$local_dir/implementation-freeze.json"

ssh "$remote" "set -e; test ! -e '$remote_upload'; install -d -m 0700 '$remote_upload'"
scp -q \
  "$work/source.bundle" \
  ops/remote-backend/s1c3_remote_transaction_v7.py \
  ops/remote-backend/verify_s1c3_transaction_v7.py \
  ops/remote-backend/s1c3b_remote_transaction_v1.py \
  ops/remote-backend/verify_s1c3b_transaction_v1.py \
  ops/remote-backend/s1c3c_schema_preflight_v1.py \
  ops/remote-backend/s1c3c_transaction_v1.py \
  ops/remote-backend/verify_s1c3c_transaction_v1.py \
  ops/remote-backend/run_s1c3c_transaction_v1.sh \
  ops/remote-backend/test_s1c3c_transaction_v1.py \
  "$candidate_config" \
  "$parity_source" \
  "$oracle_lock" \
  "$local_dir/schema-preflight.json" \
  "$local_dir/implementation-freeze.json" \
  "$work/connector-before.json" \
  "$remote:$remote_upload/"

set +e
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3c_transaction_v1.py' prepare \
    --transaction-id '$transaction_id' \
    --transaction-directory '$remote_transaction' \
    --bundle '$remote_upload/source.bundle' \
    --candidate-config '$remote_upload/transition-serving.env.candidate' \
    --parity-source '$remote_upload/main.rs' \
    --oracle-lock '$remote_upload/oracle.Cargo.lock' \
    --connector-before '$remote_upload/connector-before.json' \
    --schema-preflight '$remote_upload/schema-preflight.json' \
    --implementation-freeze '$remote_upload/implementation-freeze.json'" \
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
    PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3c_transaction_v1.py verify \
      "$local_dir" --schema-preflight "$local_dir/schema-preflight.json" --allow-terminal \
      > "$local_dir/s1c3c-authority-envelope.candidate.json"
    jq -e '.valid == true and .authority == false and .verdict == "S1C3C_RESOURCE_VETO" and
      .production_mutation == false' "$local_dir/s1c3c-authority-envelope.candidate.json" >/dev/null
    scp -q "$local_dir/s1c3c-authority-envelope.candidate.json" \
      "$remote:$remote_upload/s1c3c-authority-envelope.json"
    ssh "$remote" \
      "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3c_transaction_v1.py' seal \
        --transaction-directory '$remote_transaction' \
        --schema-preflight '$remote_upload/schema-preflight.json' \
        --mechanism-final-verification '$remote_upload/unused.json' \
        --authority-envelope '$remote_upload/s1c3c-authority-envelope.json'" \
      > "$local_dir/seal-result.json"
    rm -rf "$local_dir/evidence"
    mirror_remote
    printf 'transaction_directory=%s\nlocal_evidence=%s\n' "$remote_transaction" "$local_dir"
    printf 'verdict=S1C3C_RESOURCE_VETO production_mutation=no\n'
    exit 3
  fi
  if [[ $state == PREFLIGHT_FAILURE ]]; then
    ssh "$remote" \
      "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3c_transaction_v1.py' abort-predeployment \
        --transaction-directory '$remote_transaction' \
        --schema-preflight '$remote_upload/schema-preflight.json' \
        --reason mechanism_prepare_failed" \
      > "$local_dir/predeployment-abort.json"
    rm -rf "$local_dir/evidence" "$local_dir/rollback"
    mirror_remote
    printf 'transaction_directory=%s\nlocal_evidence=%s\n' "$remote_transaction" "$local_dir"
    printf 'verdict=S1C3C_PREFLIGHT_FAILURE production_mutation=no\n'
    exit 3
  fi
  printf 'transaction_directory=%s\nlocal_evidence=%s\n' "$remote_transaction" "$local_dir"
  printf 'prepare_code=%s state=%s production_mutation=no\n' "$prepare_code" "$state"
  exit 3
fi

prearm_failure() {
  local code=$1
  trap - ERR
  set +e
  ssh "$remote" \
    "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3c_transaction_v1.py' abort-predeployment \
      --transaction-directory '$remote_transaction' \
      --schema-preflight '$remote_upload/schema-preflight.json' \
      --reason predeployment_verification_failed" \
    > "$local_dir/predeployment-abort.json" 2>&1
  rm -rf "$local_dir/evidence" "$local_dir/rollback"
  mirror_remote
  cp "$work/connector-before.json" "$local_dir/connector-before.json"
  printf 'transaction_directory=%s\nlocal_evidence=%s\n' "$remote_transaction" "$local_dir"
  printf 'verdict=S1C3C_PREFLIGHT_FAILURE production_mutation=no\n'
  cleanup
  exit "$code"
}
trap 'prearm_failure $?' ERR

set +e
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3b_transaction_v1.py' \
    '$remote_transaction' --pre-deployment" \
  > "$local_dir/mechanism-predeployment.remote.json" 2> "$local_dir/mechanism-predeployment.error"
predeploy_code=$?
set -e
[[ $predeploy_code -eq 0 ]] || prearm_failure "$predeploy_code"
mirror_remote
cp "$work/connector-before.json" "$local_dir/connector-before.json"
PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3b_transaction_v1.py \
  "$local_dir" --pre-deployment > "$local_dir/mechanism-predeployment.local.json"
cmp "$local_dir/mechanism-predeployment.local.json" "$local_dir/mechanism-predeployment.remote.json"

PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3c_transaction_v1.py verify \
  "$local_dir" --schema-preflight "$local_dir/schema-preflight.json" \
  --mechanism-verification "$local_dir/mechanism-predeployment.local.json" --pre-deployment \
  > "$local_dir/s1c3c-predeployment.local.json"
scp -q "$local_dir/mechanism-predeployment.local.json" \
  "$remote:$remote_upload/mechanism-predeployment.json"
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3c_transaction_v1.py' verify \
    '$remote_transaction' --schema-preflight '$remote_upload/schema-preflight.json' \
    --mechanism-verification '$remote_upload/mechanism-predeployment.json' --pre-deployment" \
  > "$local_dir/s1c3c-predeployment.remote.json" \
  2> "$local_dir/s1c3c-predeployment.error"
cmp "$local_dir/s1c3c-predeployment.local.json" "$local_dir/s1c3c-predeployment.remote.json"
jq -e '.valid == true and .authority == true and .scientific_authority == false and
  .verdict == "S1C3C_PREPARATION_PASS"' "$local_dir/s1c3c-predeployment.local.json" >/dev/null
scp -q "$local_dir/mechanism-predeployment.local.json" "$local_dir/s1c3c-predeployment.local.json" \
  "$remote:$remote_upload/"

trap - ERR
rollback_armed=true
emergency_rollback() {
  local code=$?
  trap - EXIT INT TERM HUP
  set +e
  if [[ $rollback_armed == true ]]; then
    rollback_armed=false
    local state
    state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'" 2>/dev/null)
    if [[ $state == PREPARED ]]; then
      ssh "$remote" \
        "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3c_transaction_v1.py' abort-predeployment \
          --transaction-directory '$remote_transaction' \
          --schema-preflight '$remote_upload/schema-preflight.json' \
          --reason execute_failed_before_mutation" \
        > "$local_dir/emergency-predeployment-abort.json" 2>&1
      state=PREFLIGHT_FAILURE
    fi
    if [[ $state == ROLLBACK_ARMED || $state == FINALIZE_PENDING || \
          $state == FINAL_VERIFICATION_PENDING ]]; then
      ssh "$remote" \
        "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3c_transaction_v1.py' rollback \
          --transaction-directory '$remote_transaction' --reason local_orchestrator_interrupted" \
        > "$local_dir/emergency-rollback.json" 2>&1
    fi
    state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'" 2>/dev/null)
    if [[ $state == ROLLBACK_PENDING ]]; then
      connector_snapshot after "$work/emergency-connector-after.json"
      scp -q "$work/emergency-connector-after.json" \
        "$remote:$remote_upload/emergency-connector-after.json"
      ssh "$remote" \
        "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3c_transaction_v1.py' finalize \
          --transaction-directory '$remote_transaction' \
          --connector-after '$remote_upload/emergency-connector-after.json'" \
        > "$local_dir/emergency-finalize.json" 2>&1
      ssh "$remote" \
        "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3b_transaction_v1.py' \
          '$remote_transaction' --allow-rollback > '$remote_upload/emergency-mechanism-final.json' && \
         sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3c_transaction_v1.py' verify \
          '$remote_transaction' --schema-preflight '$remote_upload/schema-preflight.json' \
          --mechanism-verification '$remote_upload/emergency-mechanism-final.json' --allow-terminal \
          > '$remote_upload/emergency-s1c3c-envelope.json' && \
         sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3c_transaction_v1.py' seal \
          --transaction-directory '$remote_transaction' \
          --schema-preflight '$remote_upload/schema-preflight.json' \
          --mechanism-final-verification '$remote_upload/emergency-mechanism-final.json' \
          --authority-envelope '$remote_upload/emergency-s1c3c-envelope.json'" \
        > "$local_dir/emergency-seal.json" 2>&1
    fi
    if [[ $state == COMPLETE ]]; then
      ssh "$remote" \
        "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3c_transaction_v1.py' seal \
          --transaction-directory '$remote_transaction' \
          --schema-preflight '$remote_upload/schema-preflight.json' \
          --mechanism-final-verification '$remote_upload/mechanism-final.local.json' \
          --authority-envelope '$remote_upload/s1c3c-authority-envelope.local.json'" \
        > "$local_dir/emergency-complete-seal.json" 2>&1
    fi
  fi
  cleanup
  exit "$code"
}
trap emergency_rollback EXIT INT TERM HUP

set +e
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3c_transaction_v1.py' execute \
    --transaction-directory '$remote_transaction' \
    --schema-preflight '$remote_upload/schema-preflight.json' \
    --mechanism-predeployment-verification '$remote_upload/mechanism-predeployment.local.json' \
    --authority-predeployment-envelope '$remote_upload/s1c3c-predeployment.local.json'" \
  > "$local_dir/execute-result.json" 2> "$local_dir/execute-error.json"
execute_code=$?
set -e

state=$(ssh "$remote" "sudo -n jq -r .state '$remote_transaction/transaction-state.json'")
if [[ $state == PREPARED ]]; then
  rollback_armed=false
  ssh "$remote" \
    "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3c_transaction_v1.py' abort-predeployment \
      --transaction-directory '$remote_transaction' \
      --schema-preflight '$remote_upload/schema-preflight.json' \
      --reason execute_failed_before_mutation" \
    > "$local_dir/execute-predeployment-abort.json"
  connector_snapshot after "$work/connector-after.json"
  cp "$work/connector-before.json" "$local_dir/connector-before.json"
  cp "$work/connector-after.json" "$local_dir/connector-after.json"
  rm -rf "$local_dir/evidence" "$local_dir/rollback"
  mirror_remote
  printf 'transaction_directory=%s\nlocal_evidence=%s\n' "$remote_transaction" "$local_dir"
  printf 'verdict=S1C3C_PREFLIGHT_FAILURE execute_code=%s production_mutation=no\n' "$execute_code"
  exit 3
fi
if [[ $state == ROLLBACK_ARMED ]]; then
  ssh "$remote" \
    "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3c_transaction_v1.py' rollback \
      --transaction-directory '$remote_transaction' --reason execute_session_aborted" \
    > "$local_dir/recovery-rollback.json"
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
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3c_transaction_v1.py' finalize \
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
  "$local_dir" --allow-rollback > "$local_dir/mechanism-final.local.json"
mechanism_verify_code=$?
ssh "$remote" \
  "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3b_transaction_v1.py' \
    '$remote_transaction' --allow-rollback" \
  > "$local_dir/mechanism-final.remote.json"
remote_mechanism_code=$?
set -e

final_verdict=UNVERIFIED
if [[ $finalize_code -eq 0 && $mechanism_verify_code -eq 0 && $remote_mechanism_code -eq 0 ]]; then
  cmp "$local_dir/mechanism-final.local.json" "$local_dir/mechanism-final.remote.json"
  PYTHONPATH=ops/remote-backend python3 ops/remote-backend/verify_s1c3c_transaction_v1.py verify \
    "$local_dir" --schema-preflight "$local_dir/schema-preflight.json" \
    --mechanism-verification "$local_dir/mechanism-final.local.json" --allow-terminal \
    > "$local_dir/s1c3c-authority-envelope.local.json"
  scp -q "$local_dir/mechanism-final.local.json" \
    "$remote:$remote_upload/mechanism-final.json"
  ssh "$remote" \
    "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/verify_s1c3c_transaction_v1.py' verify \
      '$remote_transaction' --schema-preflight '$remote_upload/schema-preflight.json' \
      --mechanism-verification '$remote_upload/mechanism-final.json' --allow-terminal" \
    > "$local_dir/s1c3c-authority-envelope.remote.json"
  cmp "$local_dir/s1c3c-authority-envelope.local.json" \
    "$local_dir/s1c3c-authority-envelope.remote.json"
  scp -q "$local_dir/mechanism-final.local.json" \
    "$local_dir/s1c3c-authority-envelope.local.json" \
    "$remote:$remote_upload/"
  ssh "$remote" \
    "sudo -n env PYTHONPATH='$remote_upload' python3 '$remote_upload/s1c3c_transaction_v1.py' seal \
      --transaction-directory '$remote_transaction' \
      --schema-preflight '$remote_upload/schema-preflight.json' \
      --mechanism-final-verification '$remote_upload/mechanism-final.local.json' \
      --authority-envelope '$remote_upload/s1c3c-authority-envelope.local.json'" \
    > "$local_dir/seal-result.json"
  rollback_armed=false
  rm -rf "$local_dir/evidence" "$local_dir/rollback"
  mirror_remote
  final_verdict=$(jq -er .verdict "$local_dir/s1c3c-authority-envelope.json")
fi

printf 'transaction_directory=%s\nlocal_evidence=%s\n' "$remote_transaction" "$local_dir"
printf 'execute_code=%s finalize_code=%s mechanism_verify_code=%s remote_mechanism_code=%s\n' \
  "$execute_code" "$finalize_code" "$mechanism_verify_code" "$remote_mechanism_code"
printf 'verdict=%s\n' "$final_verdict"
if [[ $finalize_code -ne 0 || $mechanism_verify_code -ne 0 || $remote_mechanism_code -ne 0 || \
      $final_verdict != S1C3C_DEPLOYMENT_PASS ]]; then
  exit 3
fi
