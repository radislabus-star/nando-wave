#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

OUT_DIR="${NANDO_RAM_GATE_OUT_DIR:-${ROOT}/target/nando-wave/ram}"
CARGO_JSON="${OUT_DIR}/cargo-check.jsonl"
SELECTOR_JSON="${OUT_DIR}/selector-report.json"
REVIEW_TXT="${OUT_DIR}/review.txt"
GATE_JSON="${OUT_DIR}/rust-action-memory-gate.json"

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  cat <<'EOF'
nando rust-action-memory gate

Usage:
  scripts/rust-action-memory-gate.sh

Output:
  target/nando-wave/ram/cargo-check.jsonl
  target/nando-wave/ram/selector-report.json
  target/nando-wave/ram/review.txt
  target/nando-wave/ram/rust-action-memory-gate.json

Release rule:
  cargo check must pass;
  diagnostics_count must be 0;
  quarantined_candidates must be 0.

Note:
  rust-action-memory selector verdict WATCH can be OK when the workspace is
  already clean and there are no policy-allowed fix candidates. Quarantine is
  the hard blocker, not "no candidate to fix".
EOF
  exit 0
fi

mkdir -p "${OUT_DIR}"

set +e
cargo check --message-format=json > "${CARGO_JSON}"
cargo_status=$?
set -e

rust-action-memory selector-report \
  --workspace . \
  --from-cargo-json "${CARGO_JSON}" \
  --format json > "${SELECTOR_JSON}"

rust-action-memory review --workspace . > "${REVIEW_TXT}"

diagnostics_count="$(jq -r '.diagnostics_count // 0' "${SELECTOR_JSON}")"
quarantined_candidates="$(jq -r '.quarantined_candidates // 0' "${SELECTOR_JSON}")"
policy_allowed_candidates="$(jq -r '.policy_allowed_candidates // 0' "${SELECTOR_JSON}")"
selector_verdict="$(jq -r '.verdict // "UNKNOWN"' "${SELECTOR_JSON}")"
selector_blocker="$(jq -r '.blocker // ""' "${SELECTOR_JSON}")"

release_allowed=false
blocked_by_quarantine=false
if [[ "${quarantined_candidates}" != "0" ]]; then
  blocked_by_quarantine=true
fi
if [[ "${cargo_status}" == "0" && "${diagnostics_count}" == "0" && "${quarantined_candidates}" == "0" ]]; then
  release_allowed=true
fi

jq -n \
  --arg report_kind "nando_rust_action_memory_gate_v1" \
  --arg cargo_json "${CARGO_JSON}" \
  --arg selector_report "${SELECTOR_JSON}" \
  --arg review_report "${REVIEW_TXT}" \
  --arg selector_verdict "${selector_verdict}" \
  --arg selector_blocker "${selector_blocker}" \
  --arg boundary "read-only Rust safety gate: cargo check + rust-action-memory selector-report + quarantine summary; no safe apply and no source mutation" \
  --argjson cargo_check_exit_code "${cargo_status}" \
  --argjson diagnostics_count "${diagnostics_count}" \
  --argjson quarantined_candidates "${quarantined_candidates}" \
  --argjson policy_allowed_candidates "${policy_allowed_candidates}" \
  --argjson blocked_by_quarantine "${blocked_by_quarantine}" \
  --argjson release_allowed "${release_allowed}" \
  '{
    report_kind: $report_kind,
    cargo_json: $cargo_json,
    selector_report: $selector_report,
    review_report: $review_report,
    cargo_check_exit_code: $cargo_check_exit_code,
    selector_verdict: $selector_verdict,
    selector_blocker: $selector_blocker,
    diagnostics_count: $diagnostics_count,
    policy_allowed_candidates: $policy_allowed_candidates,
    quarantined_candidates: $quarantined_candidates,
    blocked_by_quarantine: $blocked_by_quarantine,
    release_allowed: $release_allowed,
    workspace_mutated: false,
    safe_apply_used: false,
    boundary: $boundary
  }' > "${GATE_JSON}"

cat "${GATE_JSON}"

if [[ "${release_allowed}" != "true" ]]; then
  exit 1
fi
