#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
AUTHORITY_BINARY=""
CLEANUP_BINARY=""

while (($#)); do
  case "$1" in
    --authority-binary)
      AUTHORITY_BINARY="$2"
      shift 2
      ;;
    --cleanup-binary)
      CLEANUP_BINARY="$2"
      shift 2
      ;;
    *)
      printf 'unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
done

if [[ ! -x "${AUTHORITY_BINARY}" || ! -x "${CLEANUP_BINARY}" ]]; then
  printf 'both release binaries are required\n' >&2
  exit 2
fi

PREFIX=/opt/nando-wave/bin
SYSTEMD_DIR=/etc/systemd/system
KEY_DIR=/etc/nando-wave/certification
ANCHOR_DIR=/var/lib/nando-wave-certification-anchor
CLEANUP_RECEIPTS_DIR=/var/lib/nando-wave-cleanup-receipts-v1
STAGING_DIR=/var/lib/nando-wave/cleanup-verifier-staging
STATE_DIR=/var/lib/nando-wave/transition/multi-source-live-v2/ms4-closed-loop-v1
JOURNAL_DIR=operator-certification-journal-v1
GROUP=nando-certification
WORK="$(mktemp -d)"
COMMITTED=false
AUTHORITY_WAS_ENABLED=false
AUTHORITY_WAS_ACTIVE=false

if systemctl is-enabled --quiet nando-operator-certification-authority.service 2>/dev/null; then
  AUTHORITY_WAS_ENABLED=true
fi
if systemctl is-active --quiet nando-operator-certification-authority.service 2>/dev/null; then
  AUTHORITY_WAS_ACTIVE=true
fi

backup_path() {
  local path="$1"
  local name="$2"
  if sudo -n test -e "${path}"; then
    sudo -n cp -a "${path}" "${WORK}/${name}"
    printf 'present' >"${WORK}/${name}.state"
  else
    printf 'absent' >"${WORK}/${name}.state"
  fi
}

restore_path() {
  local path="$1"
  local name="$2"
  sudo -n rm -rf "${path}"
  if [[ "$(<"${WORK}/${name}.state")" == present ]]; then
    sudo -n mkdir -p "$(dirname "${path}")"
    sudo -n cp -a "${WORK}/${name}" "${path}"
  fi
}

rollback() {
  local status=$?
  if [[ "${COMMITTED}" == true ]]; then
    sudo -n rm -rf "${WORK}"
    return
  fi
  sudo -n systemctl stop nando-operator-certification-authority.service >/dev/null 2>&1 || true
  sudo -n systemctl disable nando-operator-certification-authority.service >/dev/null 2>&1 || true
  restore_path "${PREFIX}/nando-operator-certification-authority" authority-binary
  restore_path "${PREFIX}/nando-operator-cleanup-verifier" cleanup-binary
  restore_path "${SYSTEMD_DIR}/nando-operator-certification-authority.service" authority-unit
  restore_path "${SYSTEMD_DIR}/nando-operator-cleanup-verifier@.service" cleanup-unit
  restore_path "${SYSTEMD_DIR}/nando-response-learning.service.d/30-certification.conf" learning-dropin
  restore_path "${SYSTEMD_DIR}/nando-transition-serving.service.d/30-certification.conf" serving-dropin
  restore_path "${KEY_DIR}" key-dir
  restore_path "${ANCHOR_DIR}" anchor-dir
  restore_path "${STATE_DIR}/${JOURNAL_DIR}" journal-dir
  sudo -n systemctl daemon-reload >/dev/null 2>&1 || true
  if [[ "${AUTHORITY_WAS_ENABLED}" == true ]]; then
    sudo -n systemctl enable nando-operator-certification-authority.service >/dev/null 2>&1 || true
  fi
  if [[ "${AUTHORITY_WAS_ACTIVE}" == true ]]; then
    sudo -n systemctl start nando-operator-certification-authority.service >/dev/null 2>&1 || true
  fi
  sudo -n rm -rf "${WORK}"
  exit "${status}"
}
trap rollback EXIT

backup_path "${PREFIX}/nando-operator-certification-authority" authority-binary
backup_path "${PREFIX}/nando-operator-cleanup-verifier" cleanup-binary
backup_path "${SYSTEMD_DIR}/nando-operator-certification-authority.service" authority-unit
backup_path "${SYSTEMD_DIR}/nando-operator-cleanup-verifier@.service" cleanup-unit
backup_path "${SYSTEMD_DIR}/nando-response-learning.service.d/30-certification.conf" learning-dropin
backup_path "${SYSTEMD_DIR}/nando-transition-serving.service.d/30-certification.conf" serving-dropin
backup_path "${KEY_DIR}" key-dir
backup_path "${ANCHOR_DIR}" anchor-dir
backup_path "${STATE_DIR}/${JOURNAL_DIR}" journal-dir

sudo -n groupadd --system --force "${GROUP}"
learning_user="$(systemctl show -p User --value nando-response-learning.service)"
if [[ -z "${learning_user}" ]]; then
  printf 'nando-response-learning.service has no service user\n' >&2
  exit 1
fi

sudo -n install -d -o root -g root -m 0755 "${PREFIX}"
sudo -n install -d -o root -g "${GROUP}" -m 0750 "${KEY_DIR}"
sudo -n install -d -o root -g "${GROUP}" -m 0750 "${ANCHOR_DIR}"
sudo -n install -d -o root -g "${GROUP}" -m 0750 "${CLEANUP_RECEIPTS_DIR}"
sudo -n install -d -o root -g "${GROUP}" -m 0770 "${STAGING_DIR}"
if ! sudo -n test -d "${STATE_DIR}"; then
  sudo -n install -d -o "${learning_user}" -g "${learning_user}" -m 0700 "${STATE_DIR}"
fi
sudo -n install -m 0755 "${AUTHORITY_BINARY}" "${PREFIX}/nando-operator-certification-authority"
sudo -n install -m 0755 "${CLEANUP_BINARY}" "${PREFIX}/nando-operator-cleanup-verifier"

for key in authority-ed25519 cleanup-verifier-ed25519; do
  private="${KEY_DIR}/${key}.key"
  public="${KEY_DIR}/${key}.pub"
  if ! sudo -n test -s "${private}"; then
    sudo -n sh -c "umask 077; openssl rand -hex 32 > '${private}'"
  fi
  sudo -n "${PREFIX}/nando-operator-certification-authority" derive-public-key \
    --private "${private}" --output "${public}.new"
  sudo -n chmod 0600 "${private}"
  sudo -n chmod 0644 "${public}.new"
  sudo -n mv "${public}.new" "${public}"
done

sudo -n install -m 0644 \
  "${ROOT_DIR}/ops/remote-backend/nando-operator-certification-authority.service" \
  "${SYSTEMD_DIR}/nando-operator-certification-authority.service"
sudo -n install -m 0644 \
  "${ROOT_DIR}/ops/remote-backend/nando-operator-cleanup-verifier@.service" \
  "${SYSTEMD_DIR}/nando-operator-cleanup-verifier@.service"
sudo -n install -d -m 0755 "${SYSTEMD_DIR}/nando-response-learning.service.d"
sudo -n install -m 0644 \
  "${ROOT_DIR}/ops/remote-backend/nando-certification-client.conf" \
  "${SYSTEMD_DIR}/nando-response-learning.service.d/30-certification.conf"
sudo -n install -d -m 0755 "${SYSTEMD_DIR}/nando-transition-serving.service.d"
sudo -n install -m 0644 \
  "${ROOT_DIR}/ops/remote-backend/nando-certification-reader.conf" \
  "${SYSTEMD_DIR}/nando-transition-serving.service.d/30-certification.conf"

sudo -n systemd-analyze verify \
  "${SYSTEMD_DIR}/nando-operator-certification-authority.service" \
  "${SYSTEMD_DIR}/nando-operator-cleanup-verifier@.service"
sudo -n systemctl daemon-reload
sudo -n systemctl enable --now nando-operator-certification-authority.service

for _ in $(seq 1 50); do
  if systemctl is-active --quiet nando-operator-certification-authority.service \
    && sudo -n test -S /run/nando-operator-certification/authority.sock; then
    COMMITTED=true
    break
  fi
  sleep 0.1
done

if [[ "${COMMITTED}" != true ]]; then
  printf 'certification authority readiness failed\n' >&2
  exit 1
fi

sudo -n rm -rf "${WORK}"
trap - EXIT
printf 'operator certification authority install: PASS\n'
