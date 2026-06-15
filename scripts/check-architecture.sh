#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

mode="${1:-full}"
case "$mode" in
  full | --full)
    run_rust_gates=1
    run_holdout_gate=1
    ;;
  contracts-only | --contracts-only)
    run_rust_gates=0
    run_holdout_gate=0
    ;;
  *)
    printf 'usage: %s [--full|--contracts-only]\n' "$0" >&2
    exit 2
    ;;
esac

fail() {
  printf 'architecture check failed: %s\n' "$1" >&2
  exit 1
}

section() {
  printf '\n== %s ==\n' "$1"
}

require_contains() {
  local haystack="$1"
  local needle="$2"
  local label="$3"

  if ! grep -Fq "$needle" <<<"$haystack"; then
    fail "$label missing '$needle'"
  fi
}

require_file() {
  local path="$1"

  [[ -f "$path" ]] || fail "missing required file $path"
}

require_executable() {
  local path="$1"

  [[ -x "$path" ]] || fail "missing executable bit on $path"
}

section "required files"
require_file Cargo.toml
require_file crates/nando-core/src/wave.rs
require_file crates/nando-core/src/wave/learn.rs
require_file crates/nando-core/src/wave/tick.rs
require_file crates/nando-cli/src/live.rs
require_file data/corpus/organ128_train_v1.txt
require_file data/corpus/organ128_dialog_ru_en_v1.tsv
require_file README.md
require_executable scripts/check.sh
require_executable scripts/check-push.sh

if [[ "$run_rust_gates" -eq 1 ]]; then
  section "rust gates"
  cargo fmt --all --check
  cargo test -p nando-core
  cargo clippy -p nando-core -- -D warnings
  cargo clippy -p nando-cli -- -D warnings
else
  section "rust gates"
  printf 'skipped in contracts-only mode\n'
fi

section "status contract"
status_output="$(cargo run -q -p nando-cli -- status)"
printf '%s\n' "$status_output"
require_contains "$status_output" "rust_first: true" "status"
require_contains "$status_output" "cell32_bytes: 32768" "status"
require_contains "$status_output" "planned_organ128_bytes: 4194304" "status"
require_contains "$status_output" "planned_organ192_bytes: 196608" "status"
require_contains "$status_output" "tiny_live_byte_adapter" "status"

organ128_output="$(cargo run -q -p nando-cli -- organ128-plan)"
require_contains "$organ128_output" "organ128_cells: 128" "organ128-plan"
require_contains "$organ128_output" "l2_hot_cells_total: 32" "organ128-plan"
require_contains "$organ128_output" "l3_warm_cells_target: 128" "organ128-plan"

section "fixed packet contract"
rg -n "pub const CELL32_BYTES: usize = 32 \\* 1024" crates/nando-core/src/wave.rs >/dev/null \
  || fail "Cell32 size constant changed"
rg -n "pub const STAGE2_ORGAN_CELLS: usize = 6" crates/nando-core/src/wave.rs >/dev/null \
  || fail "Stage2 organism cell count changed"
rg -n "pub const PHASE_SLOTS: usize = 256" crates/nando-core/src/wave.rs >/dev/null \
  || fail "phase slot count changed"
rg -n "assert_eq!\\(size_of::<Cell32>\\(\\), CELL32_BYTES\\)" crates/nando-core/src/wave.rs crates/nando-core/src/lib.rs >/dev/null \
  || fail "Cell32 size assertion missing"

section "module boundary contract"
for module in bus cache_plan carrier cell learn math organ snapshot tick; do
  require_file "crates/nando-core/src/wave/${module}.rs"
  rg -n "mod ${module};" crates/nando-core/src/wave.rs >/dev/null \
    || fail "wave facade does not declare module ${module}"
done

wave_lines="$(wc -l < crates/nando-core/src/wave.rs)"
main_lines="$(wc -l < crates/nando-cli/src/main.rs)"
[[ "$wave_lines" -le 280 ]] || fail "wave.rs facade too large: ${wave_lines} lines"
[[ "$main_lines" -le 1120 ]] || fail "nando-cli main.rs grew past current dispatcher ceiling: ${main_lines} lines"

section "no heavy runtime dependencies"
for pattern in 'pyo3' 'torch' 'tensorflow' 'candle' 'burn' 'tch'; do
  if rg -n "$pattern" Cargo.toml crates/*/Cargo.toml >/tmp/nando-wave-arch-rg.txt; then
    cat /tmp/nando-wave-arch-rg.txt >&2
    fail "forbidden heavy/runtime dependency marker found: $pattern"
  fi
done

section "live learner commands"
help_output="$(cargo run -q -p nando-cli -- --help)"
require_contains "$help_output" "live-byte-train" "help"
require_contains "$help_output" "organ128-plan" "help"
require_contains "$help_output" "organ128-train-generate" "help"
require_contains "$help_output" "organ128-dialog-generate" "help"
require_contains "$help_output" "organ128-settle-dialog" "help"
require_contains "$help_output" "organ128-wave-scorer-eval" "help"
require_contains "$help_output" "live-byte-learn" "help"
require_contains "$help_output" "live-byte-holdout" "help"
require_contains "$help_output" "live-byte-holdout-suite" "help"
require_contains "$help_output" "live-byte-holdout-seed-sweep" "help"
require_contains "$help_output" "live-cell-promote" "help"
require_contains "$help_output" "live-architecture-compare" "help"
require_contains "$help_output" "live-tissue-diagnose" "help"
require_contains "$help_output" "live-grok-trace" "help"
require_contains "$help_output" "live-grok-sweep" "help"
require_contains "$help_output" "bench-link-tissue" "help"

if [[ "$run_holdout_gate" -eq 1 ]]; then
  section "live holdout gate"
  suite_output="$(cargo run -q -p nando-cli -- live-byte-holdout-seed-sweep)"
  printf '%s\n' "$suite_output"
  require_contains "$suite_output" "code_like" "holdout seed sweep"
  require_contains "$suite_output" "ru_text" "holdout seed sweep"
  require_contains "$suite_output" "mixed_balanced" "holdout seed sweep"
  require_contains "$suite_output" "mixed_shift" "holdout seed sweep"
  require_contains "$suite_output" "oos" "holdout seed sweep"

  code_like_gap="$(awk '$1 == "code_like" { print $5 }' <<<"$suite_output")"
  ru_text_gap="$(awk '$1 == "ru_text" { print $5 }' <<<"$suite_output")"
  mixed_balanced_gap="$(awk '$1 == "mixed_balanced" { print $5 }' <<<"$suite_output")"
  mixed_shift_oos="$(awk '$1 == "mixed_shift" { print $7 }' <<<"$suite_output")"

  python3 - "$code_like_gap" "$ru_text_gap" "$mixed_balanced_gap" "$mixed_shift_oos" <<'PY'
import sys

code_like_gap, ru_text_gap, mixed_balanced_gap, mixed_shift_oos = map(float, sys.argv[1:])
if code_like_gap <= 0.0:
    raise SystemExit("code_like mean gap must stay positive")
if ru_text_gap <= 0.0:
    raise SystemExit("ru_text mean gap must stay positive")
if mixed_balanced_gap <= 0.0:
    raise SystemExit("mixed_balanced mean gap must stay positive")
if mixed_shift_oos < 0.5:
    raise SystemExit("mixed_shift must remain flagged as high-OOS")
PY
else
  section "live holdout gate"
  printf 'skipped in contracts-only mode\n'
fi

if [[ "$run_holdout_gate" -eq 1 ]]; then
  section "cell32 candidate promotion gate"
  promote_output="$(
    cargo run -q -p nando-cli -- live-cell-promote 7 \
      "let value = value + 1; let value = value + 2; let value = value + 3;"
  )"
  printf '%s\n' "$promote_output"
  require_contains "$promote_output" "candidate_accuracy" "live-cell-promote"
  require_contains "$promote_output" "accepted: true" "live-cell-promote"
  require_contains "$promote_output" "mode_status: cell32_candidate_promoted" "live-cell-promote"

  section "cellular topology compare gate"
  topology_output="$(cargo run -q -p nando-cli -- live-architecture-compare 7)"
  printf '%s\n' "$topology_output"
  require_contains "$topology_output" "cell3_wins_over_mono96" "live-architecture-compare"
  require_contains "$topology_output" "cell6_wins_over_mono192" "live-architecture-compare"
  require_contains "$topology_output" "pair_tissue_wins_over_cell6" "live-architecture-compare"
  require_contains "$topology_output" "triple_tissue_wins_over_pair" "live-architecture-compare"
  require_contains "$topology_output" "mode_status:" "live-architecture-compare"

  section "link tissue diagnose gate"
  tissue_output="$(cargo run -q -p nando-cli -- live-tissue-diagnose 7)"
  printf '%s\n' "$tissue_output"
  require_contains "$tissue_output" "typed_pair_wins_over_cell6" "live-tissue-diagnose"
  require_contains "$tissue_output" "typed_triple_wins_over_typed_pair" "live-tissue-diagnose"
  require_contains "$tissue_output" "positive_pair_ablation_cases" "live-tissue-diagnose"
  require_contains "$tissue_output" "mode_status:" "live-tissue-diagnose"

  section "grok trace gate"
  grok_output="$(cargo run -q -p nando-cli -- live-grok-trace 7 8 4)"
  printf '%s\n' "$grok_output"
  require_contains "$grok_output" "restr" "live-grok-trace"
  require_contains "$grok_output" "signal_legend" "live-grok-trace"
else
  section "cell32 candidate promotion gate"
  printf 'skipped in contracts-only mode\n'
  section "cellular topology compare gate"
  printf 'skipped in contracts-only mode\n'
  section "link tissue diagnose gate"
  printf 'skipped in contracts-only mode\n'
  section "grok trace gate"
  printf 'skipped in contracts-only mode\n'
fi

section "readme contract"
readme="$(cat README.md)"
require_contains "$readme" "live-byte-holdout-seed-sweep" "README"
require_contains "$readme" "live-cell-promote" "README"
require_contains "$readme" "live-architecture-compare" "README"
require_contains "$readme" "live-tissue-diagnose" "README"
require_contains "$readme" "LinkTissue" "README"
require_contains "$readme" "check-push.sh" "README"
require_contains "$readme" "OOS" "README"
require_contains "$readme" "Cell32 fixed 32 KB packet" "README"
require_contains "$readme" "data/corpus/organ128_train_v1.txt" "README"

section "architecture check passed"
