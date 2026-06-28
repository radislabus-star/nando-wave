#!/usr/bin/env bash
set -euo pipefail

seed="${1:-1}"
ticks="${2:-3000}"
cpu="${NANDO_BENCH_CPU:-2}"

RUSTFLAGS="-C target-cpu=native" \
  cargo build -p nando-cli --release

if command -v taskset >/dev/null 2>&1; then
  taskset -c "$cpu" target/release/nando-cli bench-symbol-l3 "$seed" "$ticks"
else
  target/release/nando-cli bench-symbol-l3 "$seed" "$ticks"
fi
