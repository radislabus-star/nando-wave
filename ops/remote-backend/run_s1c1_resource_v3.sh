#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 PROTOCOL_COMMIT OUTPUT_PARENT" >&2
  exit 2
fi

protocol_commit=$1
output_parent=$2
protocol_parent_commit=335696e903e58c3710e7f813ed79805fec5b26cc
protocol_epoch_root=2a21bc5d99a0dd8181ec105a2bdb449f66715674ffb109e3d8941a0bf9a47590
source_manifest_root=aa046add5048987c744ca25db89d1510d5f99105305d72bcfc4bed7be805b6b2
tracked_diff_sha256=283d566c531b87f16dde62f77f97a752fd1ccdabefa425c4453f396a47ea24f1
pre_action_sha256=3a22c7e2f7ba679f0294cc19fab460d28113f8dce5b5ec05fa8c88df2dfff3e9
pre_action_tests_sha256=879336edfaf0f837c503351a9184ff768b06c31f4e3b4069e180117f635b2615
capture_sha256=10aaf8ba40e0152ea205934729521adc76384b7a890acd2a8fc1c0f1e3f50486

if [[ ! $protocol_commit =~ ^[0-9a-f]{40}$ ]]; then
  echo "protocol_commit_invalid" >&2
  exit 2
fi
if [[ $(git rev-parse HEAD) != "$protocol_commit" ]]; then
  echo "protocol_commit_not_head" >&2
  exit 2
fi
if [[ $(git rev-parse HEAD^) != "$protocol_parent_commit" ]]; then
  echo "protocol_parent_commit_mismatch" >&2
  exit 2
fi
branch=$(git branch --show-current)
if [[ $(git ls-remote origin "refs/heads/$branch" | awk '{print $1}') != "$protocol_commit" ]]; then
  echo "protocol_commit_not_pushed" >&2
  exit 2
fi
if [[ $(git diff --binary | sha256sum | awk '{print $1}') != "$tracked_diff_sha256" ]]; then
  echo "tracked_candidate_identity_drift" >&2
  exit 2
fi
if [[ $(sha256sum crates/nando-operator-learning/src/grounded_decision/pre_action.rs | awk '{print $1}') != "$pre_action_sha256" ]]; then
  echo "pre_action_identity_drift" >&2
  exit 2
fi
if [[ $(sha256sum crates/nando-operator-learning/src/grounded_decision/pre_action_tests.rs | awk '{print $1}') != "$pre_action_tests_sha256" ]]; then
  echo "pre_action_tests_identity_drift" >&2
  exit 2
fi
if [[ $(sha256sum crates/nando-transition-serving/src/grounded_decision_capture.rs | awk '{print $1}') != "$capture_sha256" ]]; then
  echo "capture_identity_drift" >&2
  exit 2
fi

evidence_name="s1c1-v3-${protocol_commit:0:8}"
evidence_dir="$output_parent/$evidence_name"
remote_dir="/tmp/$evidence_name"
if [[ -e $evidence_dir ]]; then
  echo "local_evidence_directory_already_exists=$evidence_dir" >&2
  exit 2
fi
install -d -m 0700 "$evidence_dir"

connector_snapshot() {
  local label=$1
  local destination=$2
  local observed_at active_state main_pid nrestarts route_receipt_failures
  observed_at=$(date --iso-8601=ns)
  active_state=$(systemctl --user show nando-client-connector.service -p ActiveState --value)
  main_pid=$(systemctl --user show nando-client-connector.service -p MainPID --value)
  nrestarts=$(systemctl --user show nando-client-connector.service -p NRestarts --value)
  route_receipt_failures=$(
    curl -fsS --max-time 3 http://127.0.0.1:18786/metrics |
      jq -er '.route_receipt_failures'
  )
  jq -nS \
    --arg schema nando.s1c1-resource-v3.connector-snapshot.v1 \
    --arg label "$label" \
    --arg observed_at "$observed_at" \
    --arg protocol_commit "$protocol_commit" \
    --arg protocol_epoch_root "$protocol_epoch_root" \
    --arg active_state "$active_state" \
    --argjson main_pid "$main_pid" \
    --argjson nrestarts "$nrestarts" \
    --argjson route_receipt_failures "$route_receipt_failures" \
    '{schema:$schema,label:$label,observed_at:$observed_at,
      protocol_commit:$protocol_commit,protocol_epoch_root:$protocol_epoch_root,
      active_state:$active_state,main_pid:$main_pid,nrestarts:$nrestarts,
      route_receipt_failures:$route_receipt_failures}' > "$destination"
}

after_written=false
write_after_on_exit() {
  if [[ $after_written == false ]]; then
    connector_snapshot after "$evidence_dir/local_connector.after" || true
  fi
}
trap write_after_on_exit EXIT
connector_snapshot before "$evidence_dir/local_connector.before"

ssh e@192.168.3.94 bash -s -- \
  "$protocol_commit" "$protocol_parent_commit" "$protocol_epoch_root" \
  "$source_manifest_root" "$remote_dir" <<'REMOTE'
set -euo pipefail

protocol_commit=$1
protocol_parent_commit=$2
protocol_epoch_root=$3
source_manifest_root=$4
root=$5
baseline=/home/e/.cache/nando-wave-s1c1-baseline-target/release/deps/f7_generation_shadow_v3-257d2fa93e7c240e
candidate=/home/e/.cache/nando-wave-s1c1-target/release/deps/f7_generation_shadow_v3-257d2fa93e7c240e
targeted=/home/e/.cache/nando-wave-s1c1-target/release/deps/nando_response_actor-94c534b357a046f6
inherited_test=performance::full_generation_shadow_latency_stays_within_traffic_budget
targeted_test=package::tests::capture_disabled_compatibility_latency_stays_within_hot_budget
units=(
  nando-transport-gateway.service
  nando-transition-serving.service
  nando-response-learning.service
  nando-gateway-control.service
  nando-operator-certification-authority.service
)

if [[ -e $root ]]; then
  echo "remote_evidence_directory_already_exists=$root" >&2
  exit 2
fi
install -d -m 0700 "$root"

{
  printf 'protocol_commit=%s\n' "$protocol_commit"
  printf 'protocol_parent_commit=%s\n' "$protocol_parent_commit"
  printf 'protocol_epoch_root=%s\n' "$protocol_epoch_root"
  printf 'source_manifest_root=%s\n' "$source_manifest_root"
  date --iso-8601=ns
  printf 'boot_id='; cat /proc/sys/kernel/random/boot_id
  uname -a
  lscpu | grep -E '^Model name:'
  rustc -Vv
  printf 'loadavg='; cat /proc/loadavg
  sha256sum "$baseline" "$candidate" "$targeted"
} > "$root/environment.txt"

[[ $(sha256sum "$baseline" | awk '{print $1}') == ab31fde97776084de499e8d70ff3ade6d20a9d05dba912e69e5d069c777e6656 ]]
[[ $(sha256sum "$candidate" | awk '{print $1}') == 99c8b9fe8c8e192c418aa1057bec0380c568f666166d40674685aa2132982277 ]]
[[ $(sha256sum "$targeted" | awk '{print $1}') == dd785c1c96122aa1c6aa33f5f637d92636346b15d55902659cfe067c127a124b ]]
"$baseline" --list | grep -Fx "$inherited_test: test" >/dev/null
"$candidate" --list | grep -Fx "$inherited_test: test" >/dev/null
"$targeted" --list | grep -Fx "$targeted_test: test" >/dev/null

snapshot() {
  local label=$1
  {
    printf 'label=%s\n' "$label"
    date --iso-8601=ns
    printf 'loadavg='; cat /proc/loadavg
    for unit in "${units[@]}"; do
      printf 'unit=%s\n' "$unit"
      systemctl show "$unit" -p ActiveState -p MainPID -p NRestarts --no-pager
    done
    curl -fsS --max-time 3 http://192.168.3.94:8787/cpu-health |
      jq -c '{ok,mode,admission_verdict,transition_false_accepts,
              response_runtime_parity_mismatches,response_active_profiles}'
  } > "$root/$label.snapshot"
}

run_one() {
  local label=$1
  local binary=$2
  local test_name=$3
  local code
  snapshot "${label}.before"
  set +e
  RUST_TEST_THREADS=1 taskset -c 4 "$binary" --ignored --exact "$test_name" \
    --nocapture --test-threads=1 > "$root/$label.log" 2>&1
  code=$?
  set -e
  printf '%s\n' "$code" > "$root/$label.exit"
  snapshot "${label}.after"
  sleep 2
}

run_one T1 "$targeted" "$targeted_test"
run_one P1B "$baseline" "$inherited_test"
run_one P1C "$candidate" "$inherited_test"
run_one T2 "$targeted" "$targeted_test"
run_one P2C "$candidate" "$inherited_test"
run_one P2B "$baseline" "$inherited_test"
run_one T3 "$targeted" "$targeted_test"
run_one P3B "$baseline" "$inherited_test"
run_one P3C "$candidate" "$inherited_test"
snapshot final
REMOTE

scp -q -p "e@192.168.3.94:$remote_dir/"* "$evidence_dir/"
connector_snapshot after "$evidence_dir/local_connector.after"
after_written=true
trap - EXIT

file_count=$(find "$evidence_dir" -maxdepth 1 -type f | wc -l)
if [[ $file_count -ne 40 ]]; then
  echo "evidence_file_count_invalid=$file_count" >&2
  exit 2
fi
printf 'evidence_directory=%s\n' "$evidence_dir"
