#!/usr/bin/env bash
set -euo pipefail

TIMEOUT_SECONDS="${NANDO_REMOTE_TEST_TIMEOUT_SECONDS:-1800}"
KILL_AFTER_SECONDS="${NANDO_REMOTE_TEST_KILL_AFTER_SECONDS:-15}"
CARGO_BIN="${NANDO_REMOTE_CARGO_BIN:-${HOME}/.cargo/bin/cargo}"

usage() {
  cat <<'EOF'
Run a remote Cargo command with a mandatory process-group timeout.

Usage:
  run-remote-rust-test.sh [--timeout SECONDS] <cargo arguments...>

Example:
  run-remote-rust-test.sh --timeout 1800 test --workspace
EOF
}

if [[ "${1:-}" == "--timeout" ]]; then
  TIMEOUT_SECONDS="${2:-}"
  shift 2
fi
if [[ $# -eq 0 || ! "${TIMEOUT_SECONDS}" =~ ^[1-9][0-9]*$ \
  || ! "${KILL_AFTER_SECONDS}" =~ ^[1-9][0-9]*$ ]]; then
  usage >&2
  exit 2
fi
if [[ ! -x "${CARGO_BIN}" ]]; then
  printf 'cargo binary is not executable: %s\n' "${CARGO_BIN}" >&2
  exit 2
fi

set +e
timeout --signal=TERM --kill-after="${KILL_AFTER_SECONDS}s" \
  "${TIMEOUT_SECONDS}s" "${CARGO_BIN}" "$@"
rc=$?
set -e

if [[ "${rc}" == "124" || "${rc}" == "137" ]]; then
  printf 'remote Rust command exceeded %ss; process group terminated\n' \
    "${TIMEOUT_SECONDS}" >&2
fi
exit "${rc}"
