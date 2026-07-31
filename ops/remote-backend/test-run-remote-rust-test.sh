#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUNNER="${ROOT}/run-remote-rust-test.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

cat > "${WORK}/cargo-ok" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*"
EOF
chmod +x "${WORK}/cargo-ok"
output="$(NANDO_REMOTE_CARGO_BIN="${WORK}/cargo-ok" "${RUNNER}" --timeout 2 test --workspace)"
[[ "${output}" == "test --workspace" ]]

cat > "${WORK}/cargo-hang" <<EOF
#!/usr/bin/env bash
sleep 300 &
printf '%s\n' "\$!" > "${WORK}/child.pid"
wait
EOF
chmod +x "${WORK}/cargo-hang"
set +e
NANDO_REMOTE_CARGO_BIN="${WORK}/cargo-hang" \
NANDO_REMOTE_TEST_KILL_AFTER_SECONDS=1 \
  "${RUNNER}" --timeout 1 test >/dev/null 2>&1
rc=$?
set -e
[[ "${rc}" == "124" || "${rc}" == "137" ]]
child_pid="$(cat "${WORK}/child.pid")"
for _attempt in $(seq 1 20); do
  if ! kill -0 "${child_pid}" 2>/dev/null; then
    printf 'remote Rust timeout tests: PASS\n'
    exit 0
  fi
  sleep 0.1
done
printf 'timed out Cargo child survived: %s\n' "${child_pid}" >&2
exit 1
