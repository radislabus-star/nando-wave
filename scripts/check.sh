#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

TOTAL_STEPS=5
STEP=0

run() {
  STEP=$((STEP + 1))
  printf '\n[%02d/%02d] %s\n' "$STEP" "$TOTAL_STEPS" "$1"
  shift
  "$@"
}

run "fmt" cargo fmt --all --check
run "core tests" cargo test -p nando-core
run "cli check" cargo check -p nando-cli
run "architecture contracts" scripts/check-architecture.sh --contracts-only
run "tissue smoke" cargo run -q -p nando-cli -- live-tissue-diagnose 7

printf '\n[%02d/%02d] check passed\n' "$STEP" "$TOTAL_STEPS"
