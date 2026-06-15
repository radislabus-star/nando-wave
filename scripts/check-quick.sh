#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace -- -D warnings
scripts/check-architecture.sh --contracts-only
cargo run -q -p nando-cli -- status
cargo run -q -p nando-cli -- live-byte-holdout-seed-sweep
