#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
AUTHORITY="${ROOT}/ops/remote-backend/nando-operator-certification-authority.service"
CLEANUP="${ROOT}/ops/remote-backend/nando-operator-cleanup-verifier@.service"
INSTALLER="${ROOT}/ops/remote-backend/install-operator-certification.sh"

require_line() {
  local file="$1"
  local line="$2"
  grep -Fxq "${line}" "${file}" || {
    printf 'missing unit contract in %s: %s\n' "${file}" "${line}" >&2
    exit 1
  }
}

for unit in "${AUTHORITY}" "${CLEANUP}"; do
  require_line "${unit}" 'NoNewPrivileges=true'
  require_line "${unit}" 'CapabilityBoundingSet=CAP_DAC_OVERRIDE'
  require_line "${unit}" 'AmbientCapabilities='
  require_line "${unit}" 'ProtectSystem=strict'
  require_line "${unit}" 'ProtectHome=true'
done

require_line "${AUTHORITY}" \
  'InaccessiblePaths=/etc/nando-wave/certification/cleanup-verifier-ed25519.key'
require_line "${CLEANUP}" \
  'InaccessiblePaths=/etc/nando-wave/certification/authority-ed25519.key'
require_line "${CLEANUP}" \
  'InaccessiblePaths=/var/lib/nando-wave/transition'

if grep -Eq '^AmbientCapabilities=.+$' "${AUTHORITY}" "${CLEANUP}"; then
  printf 'ambient capabilities must remain empty\n' >&2
  exit 1
fi

# The installer expression is checked literally.
# shellcheck disable=SC2016
require_line "${INSTALLER}" 'if [[ "${AUTHORITY_WAS_ACTIVE}" == true ]]; then'
require_line "${INSTALLER}" \
  '  sudo -n systemctl restart nando-operator-certification-authority.service'
require_line "${INSTALLER}" \
  '  sudo -n systemctl start nando-operator-certification-authority.service'

printf 'operator certification unit contracts: PASS\n'
