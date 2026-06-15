#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

TOTAL_STEPS=63
STEP=0

progress() {
  STEP=$((STEP + 1))
  printf '\n[%02d/%02d] %s\n' "$STEP" "$TOTAL_STEPS" "$1"
}

run() {
  local label="$1"
  shift
  progress "$label"
  "$@"
}

run_shell() {
  local label="$1"
  shift
  progress "$label"
  bash -euo pipefail -c "$1"
}

run "fmt" cargo fmt --all --check
run "workspace tests" cargo test --workspace
run "workspace clippy" cargo clippy --workspace -- -D warnings
run "architecture contracts" scripts/check-architecture.sh --contracts-only
run "status" cargo run -p nando-cli -- status
run "live tissue diagnose" cargo run -p nando-cli -- live-tissue-diagnose 7
run "wave tick" cargo run -p nando-cli -- wave-tick 42 7
run "snapshot save" cargo run -p nando-cli -- snapshot-save 42 7 target/snapshots/check-stage2.nws1
run "snapshot read" cargo run -p nando-cli -- snapshot-read target/snapshots/check-stage2.nws1
run "eval one tick" cargo run -p nando-cli -- eval-one-tick 42 7
run "eval periodic" cargo run -p nando-cli -- eval-periodic 7 64 11 17
run "eval phase composition" cargo run -p nando-cli -- eval-phase-composition 13 64 19 23 5
run "eval phase holdout" cargo run -p nando-cli -- eval-phase-holdout 13 97 64
run "eval carrier control" cargo run -p nando-cli -- eval-carrier-control 13 97 64
run "eval bus transfer" cargo run -p nando-cli -- eval-bus-transfer 13 97 64
run "eval snapshot memory" cargo run -p nando-cli -- eval-snapshot-memory 13 97 64
run "eval snapshot transition" cargo run -p nando-cli -- eval-snapshot-transition 13 97 64
run "eval snapshot dynamics" cargo run -p nando-cli -- eval-snapshot-dynamics 13 97 64
run "eval snapshot multitick" cargo run -p nando-cli -- eval-snapshot-multitick 13 97 64
run "eval snapshot adapt" cargo run -p nando-cli -- eval-snapshot-adapt 13 97 64
run "eval snapshot decoder" cargo run -p nando-cli -- eval-snapshot-decoder 13 97 64
run "eval snapshot keyed" cargo run -p nando-cli -- eval-snapshot-keyed 13 97 64
run "eval snapshot keyed transition" cargo run -p nando-cli -- eval-snapshot-keyed-transition 13 97 64
run "eval snapshot noisy keyed transition" cargo run -p nando-cli -- eval-snapshot-noisy-keyed-transition 13 97 64
run "eval snapshot noisy keyed transition sweep" cargo run -p nando-cli -- eval-snapshot-noisy-keyed-transition-sweep 13 97 64
run "eval snapshot noisy keyed transition seed sweep" cargo run -p nando-cli -- eval-snapshot-noisy-keyed-transition-seed-sweep 64
run "eval byte context" cargo run -p nando-cli -- eval-byte-context 13 97 64
run "eval byte context centroid" cargo run -p nando-cli -- eval-byte-context-centroid 13 97 64
run "eval byte context offset centroid" cargo run -p nando-cli -- eval-byte-context-offset-centroid 13 97 64
run "eval byte context denoised centroid" cargo run -p nando-cli -- eval-byte-context-denoised-centroid 13 97 64
run "eval byte context relative centroid" cargo run -p nando-cli -- eval-byte-context-relative-centroid 13 97 64
run "eval byte context lexical carrier centroid" cargo run -p nando-cli -- eval-byte-context-lexical-carrier-centroid 13 97 64
run "eval byte context cellular carrier centroid" cargo run -p nando-cli -- eval-byte-context-cellular-carrier-centroid 13 97 64
run "eval byte context trained carrier centroid" cargo run -p nando-cli -- eval-byte-context-trained-carrier-centroid 13 97 64
run "eval byte context prompt carrier centroid" cargo run -p nando-cli -- eval-byte-context-prompt-carrier-centroid 13 97 64
run "eval byte context prompt carrier diverse centroid" cargo run -p nando-cli -- eval-byte-context-prompt-carrier-diverse-centroid 13 97 64
run "eval byte context centroid seed sweep" cargo run -p nando-cli -- eval-byte-context-centroid-seed-sweep 64
run "eval byte context offset centroid seed sweep" cargo run -p nando-cli -- eval-byte-context-offset-centroid-seed-sweep 64
run "eval byte context denoised centroid seed sweep" cargo run -p nando-cli -- eval-byte-context-denoised-centroid-seed-sweep 64
run "eval byte context relative centroid seed sweep" cargo run -p nando-cli -- eval-byte-context-relative-centroid-seed-sweep 64
run "eval byte context lexical carrier centroid seed sweep" cargo run -p nando-cli -- eval-byte-context-lexical-carrier-centroid-seed-sweep 64
run "eval byte context cellular carrier centroid seed sweep" cargo run -p nando-cli -- eval-byte-context-cellular-carrier-centroid-seed-sweep 64
run "eval byte context trained carrier centroid seed sweep" cargo run -p nando-cli -- eval-byte-context-trained-carrier-centroid-seed-sweep 64
run "eval byte context prompt carrier centroid seed sweep" cargo run -p nando-cli -- eval-byte-context-prompt-carrier-centroid-seed-sweep 64
run "eval byte context prompt carrier diverse centroid seed sweep" cargo run -p nando-cli -- eval-byte-context-prompt-carrier-diverse-centroid-seed-sweep 64
run "eval byte context centroid ablation" cargo run -p nando-cli -- eval-byte-context-centroid-ablation 13 97 64
run "eval byte context cellular carrier ablation" cargo run -p nando-cli -- eval-byte-context-cellular-carrier-ablation 13 97 128
run "eval byte context trained carrier ablation" cargo run -p nando-cli -- eval-byte-context-trained-carrier-ablation 13 97 128
run "eval byte context prompt carrier ablation" cargo run -p nando-cli -- eval-byte-context-prompt-carrier-ablation 13 97 128
run "eval byte context prompt carrier diverse ablation" cargo run -p nando-cli -- eval-byte-context-prompt-carrier-diverse-ablation 13 97 128
run "eval chat0" cargo run -p nando-cli -- eval-chat0 13 97 128
run "eval settle word" cargo run -p nando-cli -- eval-settle-word 13 97 128
run "eval settle word seed sweep" cargo run -p nando-cli -- eval-settle-word-seed-sweep 128
run "eval chat0 route" cargo run -p nando-cli -- eval-chat0-route 13 97 128
run "chat0 once" cargo run -p nando-cli -- chat0-once "cmd ping #1 answer: " pong target/chat0-traces/check.trace
run_shell "chat0 shell smoke" "printf 'manual:2: ping? || pong\n:quit\n' | cargo run -p nando-cli -- chat0-shell target/chat0-traces/shell-check target/chat0-feedback/chat0-shell-check.log"
run "prepare chat0 feedback dir" mkdir -p target/chat0-feedback
run_shell "write chat0 promote feedback" "printf 'prompt=manual:2: ping? response=pong expected=help feedback_correct=Some(false) route=prompt_cloud_lock_bank predicted_task=ping coherence=0.000000 spectral_entropy=0.000000\n' > target/chat0-feedback/chat0-promote-check.log"
run "eval chat0 promote" cargo run -p nando-cli -- eval-chat0-promote target/chat0-feedback/chat0-promote-check.log 13 97 128
run "chat0 promote save" cargo run -p nando-cli -- chat0-promote-save target/chat0-feedback/chat0-promote-check.log target/chat0-feedback/chat0-promoted-check.nwps 13 97 128
run "chat0 once promoted" cargo run -p nando-cli -- chat0-once-promoted target/chat0-feedback/chat0-promoted-check.nwps "manual:2: ping?" help target/chat0-traces/promoted-check.trace
run_shell "write chat0 promoted holdout feedback" "{
  printf 'prompt=manual:2: ping? response=? expected=pong feedback_correct=Some(false) route=manual predicted_task=ping coherence=0.000000 spectral_entropy=0.000000\n'
  printf 'prompt=manual:2: name? response=? expected=nando feedback_correct=Some(false) route=manual predicted_task=name coherence=0.000000 spectral_entropy=0.000000\n'
  printf 'prompt=manual:2: time? response=? expected=now feedback_correct=Some(false) route=manual predicted_task=time coherence=0.000000 spectral_entropy=0.000000\n'
  printf 'prompt=manual:2: help? response=? expected=help feedback_correct=Some(false) route=manual predicted_task=help coherence=0.000000 spectral_entropy=0.000000\n'
  printf 'prompt=manual:2: echo? response=? expected=echo feedback_correct=Some(false) route=manual predicted_task=echo coherence=0.000000 spectral_entropy=0.000000\n'
  printf 'prompt=manual:2: save? response=? expected=saved feedback_correct=Some(false) route=manual predicted_task=save coherence=0.000000 spectral_entropy=0.000000\n'
  printf 'prompt=manual:2: open? response=? expected=opened feedback_correct=Some(false) route=manual predicted_task=open coherence=0.000000 spectral_entropy=0.000000\n'
  printf 'prompt=manual:2: close? response=? expected=closed feedback_correct=Some(false) route=manual predicted_task=close coherence=0.000000 spectral_entropy=0.000000\n'
} > target/chat0-feedback/chat0-promoted-holdout-check.log"
run "eval chat0 promoted holdout" cargo run -p nando-cli -- eval-chat0-promoted-holdout target/chat0-feedback/chat0-promoted-holdout-check.log 13 97 128

printf '\n[%02d/%02d] check passed\n' "$STEP" "$TOTAL_STEPS"
