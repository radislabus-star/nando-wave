#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

fail() {
  printf 'goal check failed: %s\n' "$1" >&2
  exit 1
}

require_file() {
  local path="$1"
  [[ -f "$path" ]] || fail "missing required file $path"
}

require_contains() {
  local path="$1"
  local needle="$2"
  grep -Fq "$needle" "$path" || fail "$path missing '$needle'"
}

require_absent_regex() {
  local path="$1"
  local pattern="$2"
  if rg -n "$pattern" "$path" >/tmp/nando-wave-goal-rg.txt; then
    cat /tmp/nando-wave-goal-rg.txt >&2
    fail "$path contains forbidden pattern: $pattern"
  fi
}

require_file docs/GOAL.md
require_file docs/ROADMAP.md
require_file README.md

require_contains README.md "docs/GOAL.md"
require_contains docs/ROADMAP.md "Главный goal-контракт проекта находится в \`docs/GOAL.md\`"

require_contains docs/GOAL.md "CPU-friendly wave cellular organism evidence package"
require_contains docs/GOAL.md "## Уровни победы"
require_contains docs/GOAL.md "## Формула найденной моды"
require_contains docs/GOAL.md "mode_exists ="
require_contains docs/GOAL.md "ensemble_gain >= 0.03"
require_contains docs/GOAL.md "key_ablation_drop >= 0.05"
require_contains docs/GOAL.md "seed_robustness >= 4/5 seed pairs"
require_contains docs/GOAL.md "## Experimental Protocol v0"
require_contains docs/GOAL.md "label_shuffle ломает результат"
require_contains docs/GOAL.md "runtime/readout не имеет права вызывать label formula"
require_contains docs/GOAL.md "scientific_pass:"
require_contains docs/GOAL.md "engineering_pass:"
require_contains docs/GOAL.md "organ128-modadd-eval"
require_contains docs/GOAL.md "organ128-modadd-seed-sweep"
require_contains docs/GOAL.md "Fourier phase control"
require_contains docs/GOAL.md "fourier_phase_accuracy"
require_contains docs/GOAL.md "cell32_phase_compose_accuracy"
require_contains docs/GOAL.md "cell32_structured_compose_accuracy"
require_contains docs/GOAL.md "cell32_fourier_census_accuracy"
require_contains docs/GOAL.md "cell32_component_bus_projection_accuracy"
require_contains docs/GOAL.md "cell32_component_link_projection_accuracy"
require_contains docs/GOAL.md "phase_compose_gain"
require_contains docs/GOAL.md "structured_compose_gain"
require_contains docs/GOAL.md "fourier_census_gain"
require_contains docs/GOAL.md "component_bus_projection_gain"
require_contains docs/GOAL.md "component_bus_shuffle_accuracy"
require_contains docs/GOAL.md "component_link_projection_gain"
require_contains docs/GOAL.md "component_link_shuffle_accuracy"
require_contains docs/GOAL.md "component_bus_a_drop"
require_contains docs/GOAL.md "component_bus_b_drop"
require_contains docs/GOAL.md "component_bus_phase_drop"
require_contains docs/GOAL.md "component_bus_amplitude_drop"
require_contains docs/GOAL.md "component_bus_wrong_pair_drop"
require_contains docs/GOAL.md "component_link_phase_drop"
require_contains docs/GOAL.md "component_link_amplitude_drop"
require_contains docs/GOAL.md "component_link_wrong_pair_drop"
require_contains docs/GOAL.md "wave_over_fourier_gap"
require_contains docs/GOAL.md "compose_over_fourier_gap"
require_contains docs/GOAL.md "structured_over_fourier_gap"
require_contains docs/GOAL.md "census_over_fourier_gap"
require_contains docs/GOAL.md "component_bus_over_fourier_gap"
require_contains docs/GOAL.md "component_bus_no_shortcut_control"
require_contains docs/GOAL.md "component_link_no_shortcut_control"
require_contains docs/GOAL.md "rejected as architectural direction v1"
require_contains docs/GOAL.md "rejected as architectural direction v2"
require_contains docs/GOAL.md "rejected as architectural direction v3"
require_contains docs/GOAL.md "candidate architectural direction v4"
require_contains docs/GOAL.md "strong candidate architectural direction v5"
require_contains docs/GOAL.md "organ128_modadd_component_link_candidate"
require_contains docs/GOAL.md "organ128_modadd_component_link_seed_sweep_candidate"
require_contains docs/GOAL.md "max_component_bus_shuffle_accuracy"
require_contains docs/GOAL.md "min_component_bus_wrong_pair_drop"
require_contains docs/GOAL.md "min_component_link_wrong_pair_drop"
require_contains docs/GOAL.md "реальный candidate circuit seed"
require_contains docs/GOAL.md "Межкомпонентная link-свертка"
require_contains docs/GOAL.md "Fourier census"
require_contains docs/GOAL.md "компонентные sin/cos каналы"
require_contains docs/GOAL.md "wrong_component_pair"
require_contains docs/GOAL.md "label_shuffle_accuracy"
require_contains docs/GOAL.md "no_shortcut_control"
require_contains README.md "organ128-modadd-eval"
require_contains README.md "organ128-modadd-seed-sweep"

require_absent_regex docs/GOAL.md "CPU-friendly wave cellular organism proof"
require_absent_regex docs/GOAL.md "финальн(ый|ое) proof"
require_absent_regex docs/ROADMAP.md "финальн(ый|ое) proof"
require_absent_regex docs/ROADMAP.md "пока не доказан"
require_absent_regex docs/ROADMAP.md "не доказано"

printf 'goal check passed\n'
