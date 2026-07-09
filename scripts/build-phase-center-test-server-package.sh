#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

VERSION="${NANDO_PHASE_CENTER_PACKAGE_VERSION:-$(date -u +%Y%m%dT%H%M%SZ)}"
OUT_ROOT="${ROOT}/target/nando-wave/deploy"
PACKAGE_NAME="nando-phase-center-test-server-${VERSION}"
OUT_DIR="${OUT_ROOT}/${PACKAGE_NAME}"
TARBALL="${OUT_DIR}.tar.gz"
SHA256_PATH="${TARBALL}.sha256"
REPORT="${OUT_ROOT}/${PACKAGE_NAME}.package.json"

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  cat <<'EOF'
nando phase-center test-server release bundle

Usage:
  scripts/build-phase-center-test-server-package.sh

Environment:
  NANDO_PHASE_CENTER_PACKAGE_VERSION=YYYYmmddTHHMMSSZ
  NANDO_SKIP_BUILD=1
  NANDO_SKIP_RAM_GATE=1

Output:
  target/nando-wave/deploy/nando-phase-center-test-server-<version>.tar.gz
  target/nando-wave/deploy/nando-phase-center-test-server-<version>.tar.gz.sha256
  target/nando-wave/deploy/nando-phase-center-test-server-<version>.package.json
EOF
  exit 0
fi

if [[ "${NANDO_SKIP_BUILD:-0}" != "1" ]]; then
  echo "build release nando-cli..."
  cargo build --release -q -p nando-cli
fi

RAM_GATE_REPORT="${ROOT}/target/nando-wave/ram/rust-action-memory-gate.json"
RAM_GATE_USED=false
if [[ "${NANDO_SKIP_RAM_GATE:-0}" != "1" ]]; then
  echo "run rust-action-memory selector/quarantine gate..."
  "${ROOT}/scripts/rust-action-memory-gate.sh" >/dev/null
  RAM_GATE_USED=true
fi

if [[ ! -x "${ROOT}/target/release/nando-cli" ]]; then
  echo "missing release binary: ${ROOT}/target/release/nando-cli" >&2
  exit 2
fi

rm -rf "${OUT_DIR}" "${TARBALL}" "${SHA256_PATH}" "${REPORT}"
mkdir -p "${OUT_DIR}/bin" "${OUT_DIR}/data" "${OUT_DIR}/docs" "${OUT_DIR}/ops"

cp "${ROOT}/target/release/nando-cli" "${OUT_DIR}/bin/nando-cli"
cp -R "${ROOT}/ops/phase-center-test-server" "${OUT_DIR}/ops/phase-center-test-server"
if [[ -d "${ROOT}/data/real_traffic" ]]; then
  cp -R "${ROOT}/data/real_traffic" "${OUT_DIR}/data/real_traffic"
fi
for doc in \
  docs/NANDA_CPU_COMPACT_LATENT_TRANSITION_ARCHITECTURE.md \
  docs/ARCHITECTURE_VERSION_REGISTRY.md \
  docs/EXECUTOR_REVIEW_NOTES.md
do
  if [[ -f "${ROOT}/${doc}" ]]; then
    cp "${ROOT}/${doc}" "${OUT_DIR}/docs/"
  fi
done
if [[ "${RAM_GATE_USED}" == "true" && -s "${RAM_GATE_REPORT}" ]]; then
  cp "${RAM_GATE_REPORT}" "${OUT_DIR}/docs/rust-action-memory-gate.json"
fi

find "${OUT_DIR}" -type d -name '__pycache__' -prune -exec rm -rf {} +
find "${OUT_DIR}" -type f -name '*.pyc' -delete
find "${OUT_DIR}/ops/phase-center-test-server/bin" -type f \( -name '*.sh' -o -name '*.py' \) -exec chmod 0755 {} \;
chmod 0755 "${OUT_DIR}/bin/nando-cli" "${OUT_DIR}/ops/phase-center-test-server/deploy.sh"

cat > "${OUT_DIR}/install-from-bundle.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

BUNDLE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export NANDO_DEPLOY_NANDO_CLI_BIN="${BUNDLE_ROOT}/bin/nando-cli"
exec "${BUNDLE_ROOT}/ops/phase-center-test-server/deploy.sh" "$@"
EOF
chmod 0755 "${OUT_DIR}/install-from-bundle.sh"

GIT_COMMIT="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
GIT_DIRTY=false
if [[ -n "$(git status --short 2>/dev/null || true)" ]]; then
  GIT_DIRTY=true
fi
FILE_COUNT="$(find "${OUT_DIR}" -type f | wc -l | tr -d ' ')"
CREATED_UTC="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
RAM_RELEASE_ALLOWED=false
RAM_SELECTOR_VERDICT="SKIPPED"
RAM_QUARANTINED_CANDIDATES=0
RAM_DIAGNOSTICS_COUNT=0
if [[ "${RAM_GATE_USED}" == "true" && -s "${RAM_GATE_REPORT}" ]]; then
  RAM_RELEASE_ALLOWED="$(jq -r '.release_allowed // false' "${RAM_GATE_REPORT}")"
  RAM_SELECTOR_VERDICT="$(jq -r '.selector_verdict // "UNKNOWN"' "${RAM_GATE_REPORT}")"
  RAM_QUARANTINED_CANDIDATES="$(jq -r '.quarantined_candidates // 0' "${RAM_GATE_REPORT}")"
  RAM_DIAGNOSTICS_COUNT="$(jq -r '.diagnostics_count // 0' "${RAM_GATE_REPORT}")"
fi

jq -n \
  --arg report_kind "nando_phase_center_test_server_bundle_manifest_v1" \
  --arg version "${VERSION}" \
  --arg created_utc "${CREATED_UTC}" \
  --arg git_commit "${GIT_COMMIT}" \
  --argjson git_dirty "${GIT_DIRTY}" \
  --arg package_name "${PACKAGE_NAME}" \
  --arg install_entrypoint "./install-from-bundle.sh" \
  --arg product_path "phase-center .nwpc" \
  --arg rust_action_memory_gate "docs/rust-action-memory-gate.json" \
  --arg ram_selector_verdict "${RAM_SELECTOR_VERDICT}" \
  --argjson file_count "${FILE_COUNT}" \
  --argjson ram_release_allowed "${RAM_RELEASE_ALLOWED}" \
  --argjson ram_quarantined_candidates "${RAM_QUARANTINED_CANDIDATES}" \
  --argjson ram_diagnostics_count "${RAM_DIAGNOSTICS_COUNT}" \
  '{
    report_kind: $report_kind,
    version: $version,
    created_utc: $created_utc,
    git: {
      commit: $git_commit,
      dirty: $git_dirty
    },
    package_name: $package_name,
    install_entrypoint: $install_entrypoint,
    product_path: $product_path,
    included_file_count: $file_count,
    rust_action_memory_gate: {
      report: $rust_action_memory_gate,
      release_allowed: $ram_release_allowed,
      selector_verdict: $ram_selector_verdict,
      diagnostics_count: $ram_diagnostics_count,
      quarantined_candidates: $ram_quarantined_candidates
    },
    server_policy: {
      local_accept_controlled_by_server_env: true,
      upstream_controlled_by_server_env: true,
      provider_secret_printed: false
    },
    forbidden_flags: {
      nwrb_product_path_used: false,
      role_binding_backend_used: false,
      lookup_authority_used: false,
      target_id_or_proof_rule_id_authority_used: false,
      concrete_x_lookup_used: false,
      manual_local_out_t_used: false,
      local_accept_without_verifier_used: false,
      synthetic_money_claim_used: false
    },
    boundary: "boxed deployment bundle only: installs phase-center test-server services, prebuilt nando-cli, metrics/readiness/evidence scripts, provider bridge, and disabled/policy-gated local accept; it does not configure provider upstream secrets and does not unlock money claims"
  }' > "${OUT_DIR}/bundle-manifest.json"

cat > "${OUT_DIR}/README_DEPLOY.md" <<EOF
# Nando Phase-Center Test Server Bundle

version: ${VERSION}
created_utc: ${CREATED_UTC}
git_commit: ${GIT_COMMIT}
git_dirty: ${GIT_DIRTY}
rust_action_memory_release_allowed: ${RAM_RELEASE_ALLOWED}
rust_action_memory_selector_verdict: ${RAM_SELECTOR_VERDICT}
rust_action_memory_quarantined_candidates: ${RAM_QUARANTINED_CANDIDATES}

Install:

\`\`\`bash
tar -xzf ${PACKAGE_NAME}.tar.gz
cd ${PACKAGE_NAME}
./install-from-bundle.sh
\`\`\`

User-mode install:

\`\`\`bash
./install-from-bundle.sh --user
\`\`\`

Configure upstream later, without printing provider secrets:

\`\`\`bash
printf '%s\n' "\$OPENAI_API_KEY" | sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-provider-bridge-upstream-config.sh \\
  /etc/nando-wave/phase-center.env \\
  set --base-url https://api.openai.com --api-key-stdin --provider openai
\`\`\`

Verify:

\`\`\`bash
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-test-server-verify.sh /etc/nando-wave/phase-center.env
curl -s http://127.0.0.1:8787/health
jq .verdict /var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-upstream-readiness.json
\`\`\`

Boundary: this is the phase-center / .nwpc server package. It does not enable
fake market-money claims and does not make client windows carry provider
secrets. The system server policy env is secret material and is installed as
mode 0600; ordinary clients should use the HTTP bridge, not read that file.
EOF

(
  cd "${OUT_ROOT}"
  tar -czf "$(basename "${TARBALL}")" "${PACKAGE_NAME}"
)

sha256sum "${TARBALL}" > "${SHA256_PATH}"
SHA256="$(cut -d' ' -f1 "${SHA256_PATH}")"
TARBALL_BYTES="$(wc -c < "${TARBALL}" | tr -d ' ')"

jq -n \
  --arg report_kind "nando_phase_center_test_server_package_v2" \
  --arg version "${VERSION}" \
  --arg created_utc "${CREATED_UTC}" \
  --arg out_dir "${OUT_DIR}" \
  --arg tarball "${TARBALL}" \
  --arg sha256_path "${SHA256_PATH}" \
  --arg sha256 "${SHA256}" \
  --argjson tarball_bytes "${TARBALL_BYTES}" \
  --arg package_manifest "${OUT_DIR}/bundle-manifest.json" \
  --arg install_entrypoint "${OUT_DIR}/install-from-bundle.sh" \
  --arg product_path "phase-center .nwpc" \
  --arg rust_action_memory_gate "${OUT_DIR}/docs/rust-action-memory-gate.json" \
  --arg ram_selector_verdict "${RAM_SELECTOR_VERDICT}" \
  --argjson file_count "${FILE_COUNT}" \
  --argjson ram_release_allowed "${RAM_RELEASE_ALLOWED}" \
  --argjson ram_quarantined_candidates "${RAM_QUARANTINED_CANDIDATES}" \
  --argjson ram_diagnostics_count "${RAM_DIAGNOSTICS_COUNT}" \
  '{
    report_kind: $report_kind,
    version: $version,
    created_utc: $created_utc,
    out_dir: $out_dir,
    tarball: $tarball,
    sha256_path: $sha256_path,
    sha256: $sha256,
    tarball_bytes: $tarball_bytes,
    package_manifest: $package_manifest,
    install_entrypoint: $install_entrypoint,
    product_path: $product_path,
    included_file_count: $file_count,
    rust_action_memory_gate: {
      report: $rust_action_memory_gate,
      release_allowed: $ram_release_allowed,
      selector_verdict: $ram_selector_verdict,
      diagnostics_count: $ram_diagnostics_count,
      quarantined_candidates: $ram_quarantined_candidates
    },
    install_ready_artifact: true,
    upstream_configured_by_bundle: false,
    provider_secret_printed: false,
    market_money_claim_allowed: false,
    local_accept_changed_by_package_build: false,
    forbidden_flags: {
      nwrb_product_path_used: false,
      role_binding_backend_used: false,
      lookup_authority_used: false,
      target_id_or_proof_rule_id_authority_used: false,
      concrete_x_lookup_used: false,
      manual_local_out_t_used: false,
      local_accept_without_verifier_used: false,
      synthetic_money_claim_used: false
    },
    boundary: "release artifact for rapid server deployment; runtime safety and upstream provider secrets remain server policy"
  }' > "${REPORT}"

echo "${REPORT}"
