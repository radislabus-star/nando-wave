#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

TOTAL_STEPS=8
STEP=0

run() {
  STEP=$((STEP + 1))
  printf '\n[%02d/%02d] %s\n' "$STEP" "$TOTAL_STEPS" "$1"
  shift
  "$@"
}

run "fmt" cargo fmt --all --check
run "goal contracts" scripts/check-goal.sh
run "core tests" cargo test -p nando-core
run "cli check" cargo check -p nando-cli
run "architecture contracts" scripts/check-architecture.sh --contracts-only
run "tissue smoke" cargo run -q -p nando-cli -- live-tissue-diagnose 7
run "modadd goal smoke" bash -c 'cargo run -q -p nando-cli -- organ128-modadd-eval 7 31 64 64 | tee /tmp/nando-modadd-goal.log && rg -q "mode_status: (organ128_modadd_key_mode_ablation_passed|organ128_modadd_settle_link_candidate|organ128_modadd_component_link_candidate|organ128_modadd_candidate|not_found_organ128_modadd)" /tmp/nando-modadd-goal.log && rg -q "label_shuffle_accuracy:" /tmp/nando-modadd-goal.log && rg -q "no_shortcut_control:" /tmp/nando-modadd-goal.log'
run "response gate smoke" bash -c 'cargo run -q -p nando-cli -- organ128-response-gate-eval 7 12 | tee /tmp/nando-response-gate.log && rg -q "mode_status: organ128_response_gate_candidate" /tmp/nando-response-gate.log'

printf '\n[%02d/%02d] check passed\n' "$STEP" "$TOTAL_STEPS"
