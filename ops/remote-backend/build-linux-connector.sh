#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET="${NANDO_CONNECTOR_TARGET:-x86_64-unknown-linux-musl}"
TOOLCHAIN="${NANDO_CONNECTOR_RUST_TOOLCHAIN:-1.97.0}"
CARGO_BIN="${NANDO_CONNECTOR_CARGO_BIN:-${HOME}/.cargo/bin/cargo}"
OUTPUT_DIR="${NANDO_CONNECTOR_OUTPUT_DIR:-${ROOT}/dist/nando-connector}"
TARGET_BINARY="${ROOT}/target/${TARGET}/release/nando-connector"
OUTPUT_BINARY="${OUTPUT_DIR}/nando-connector-linux-x86_64"

"${CARGO_BIN}" "+${TOOLCHAIN}" build \
  --release \
  --locked \
  --target "${TARGET}" \
  -p nando-client-connector

install -d -m 0755 "${OUTPUT_DIR}"
install -m 0755 "${TARGET_BINARY}" "${OUTPUT_BINARY}"
strip --strip-unneeded "${OUTPUT_BINARY}"
sha256sum "${OUTPUT_BINARY}" >"${OUTPUT_BINARY}.sha256"

file "${OUTPUT_BINARY}"
stat -c '%n %s bytes' "${OUTPUT_BINARY}"
cat "${OUTPUT_BINARY}.sha256"
