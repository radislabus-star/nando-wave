#!/usr/bin/env bash
set -euo pipefail

BINARY_SOURCE=""
CLIENT_KEY_SOURCE=""
SERVICE="nando-response-learning.service"
INSTALL_BINARY="${NANDO_REMOTE_SERVING_BINARY:-/opt/nando-wave/bin/nando-transition-serving}"
ROLE_ENV="${NANDO_REMOTE_LEARNING_ENV:-/etc/nando-wave/roles/response-learning.env}"
KEY_DIRECTORY="${NANDO_REMOTE_EVIDENCE_KEYS:-/etc/nando-wave/evidence-clients}"
SPOOL_DIRECTORY="${NANDO_REMOTE_EVIDENCE_SPOOL:-/var/lib/nando-wave/transition/multi-source-live-v2/remote-evidence-spool-v1}"
LEARNING_HEALTH="${NANDO_REMOTE_LEARNING_HEALTH:-http://127.0.0.1:18790/health}"
HOT_HEALTH="${NANDO_REMOTE_HOT_HEALTH:-http://127.0.0.1:18789/health}"
LEARNING_STATE="${NANDO_REMOTE_LEARNING_STATE:-/var/lib/nando-wave/transition/multi-source-live-v2}"
READINESS_ATTEMPTS="${NANDO_REMOTE_EVIDENCE_READINESS_ATTEMPTS:-12}"
READINESS_SLEEP_SECONDS="${NANDO_REMOTE_EVIDENCE_READINESS_SLEEP_SECONDS:-0.5}"
LEARNING_HEALTH_TIMEOUT_SECONDS="${NANDO_REMOTE_EVIDENCE_HEALTH_TIMEOUT_SECONDS:-10}"

usage() {
  cat <<'EOF'
Transactionally enable the authenticated remote evidence spool.

Usage:
  ops/remote-backend/install-remote-evidence-spool.sh \
    --binary /path/to/nando-transition-serving \
    --client-key /path/to/raw-32-byte-client.key

Only the cold learner is stopped. The hot serving process and Nginx stay up.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary)
      BINARY_SOURCE="${2:-}"
      shift 2
      ;;
    --client-key)
      CLIENT_KEY_SOURCE="${2:-}"
      shift 2
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      printf 'unknown argument: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ ! -x "${BINARY_SOURCE}" ]]; then
  printf 'serving binary is not executable: %s\n' "${BINARY_SOURCE}" >&2
  exit 2
fi
if [[ ! -f "${CLIENT_KEY_SOURCE}" ]] \
  || [[ "$(stat -c '%s' "${CLIENT_KEY_SOURCE}")" != "32" ]]; then
  printf 'client key must be a raw 32-byte file\n' >&2
  exit 2
fi
if [[ ! -f "${INSTALL_BINARY}" || ! -f "${ROLE_ENV}" ]]; then
  printf 'installed serving binary or learner role env is missing\n' >&2
  exit 2
fi
if ! sudo -n true; then
  printf 'passwordless sudo is required\n' >&2
  exit 2
fi

service_user="$(systemctl show -p User --value "${SERVICE}")"
service_group="$(systemctl show -p Group --value "${SERVICE}")"
service_user="${service_user:-$(id -un)}"
service_group="${service_group:-$(id -gn "${service_user}")}"
client_id="$(sha256sum "${CLIENT_KEY_SOURCE}")"
client_id="${client_id%% *}"
key_path="${KEY_DIRECTORY}/${client_id}.key"
client_key_already_installed=0
if [[ -e "${key_path}" && "${CLIENT_KEY_SOURCE}" -ef "${key_path}" ]]; then
  client_key_already_installed=1
fi

work="$(mktemp -d)"
candidate_binary="${work}/nando-transition-serving"
candidate_env="${work}/response-learning.env"
backup_binary="${work}/previous-binary"
backup_env="${work}/previous-env"
rollback_env="${work}/rollback-env"
backup_key="${work}/previous-key"
key_existed=0
learner_was_active=0
rollback_armed=0

set_env_value_in_file() {
  local file="$1"
  local key="$2"
  local value="$3"
  if grep -qE "^${key}=" "${file}"; then
    sed -i "s#^${key}=.*#${key}=${value}#" "${file}"
  else
    printf '%s=%s\n' "${key}" "${value}" >> "${file}"
  fi
}

set_env_value() {
  local key="$1"
  local value="$2"
  set_env_value_in_file "${candidate_env}" "${key}" "${value}"
}

wait_learning_ready() {
  local _attempt
  for _attempt in $(seq 1 "${READINESS_ATTEMPTS}"); do
    if curl -fsS --max-time "${LEARNING_HEALTH_TIMEOUT_SECONDS}" "${LEARNING_HEALTH}" \
      | jq -e '
          .ok == true
          and .remote_evidence.enabled == true
          and .remote_evidence.transport_ready == true
          and .remote_evidence.configured_clients >= 1
          and .learning_health.serving_healthy == true
          and .learning_health.authority_ready == false
          and .learning_health.phase_mutation_allowed == false
        ' >/dev/null 2>&1; then
      return 0
    fi
    sleep "${READINESS_SLEEP_SECONDS}"
  done
  return 1
}

cleanup() {
  set +e
  rm -rf "${work}"
}

rollback() {
  local rc="${1:-1}"
  trap - ERR INT TERM EXIT
  set +e
  if [[ "${rollback_armed}" == "1" ]]; then
    sudo -n systemctl stop "${SERVICE}"
    sudo -n install -m 0755 "${backup_binary}" "${INSTALL_BINARY}"
    cp -a "${backup_env}" "${rollback_env}"
    set_env_value_in_file \
      "${rollback_env}" NANDO_K1_NATURAL_SCHEDULER_ENABLED 0
    sudo -n install -m 0644 "${rollback_env}" "${ROLE_ENV}"
    if [[ "${key_existed}" == "1" ]]; then
      sudo -n install -o "${service_user}" -g "${service_group}" -m 0600 \
        "${backup_key}" "${key_path}"
    else
      sudo -n rm -f "${key_path}"
    fi
    if [[ "${learner_was_active}" == "1" ]]; then
      sudo -n systemctl start "${SERVICE}"
    fi
    printf '%s\n' \
      'remote evidence spool install failed; binary/config restored, K1 disabled, append-only state preserved' >&2
  fi
  cleanup
  exit "${rc}"
}

trap 'rollback $?' ERR
trap 'rollback 130' INT
trap 'rollback 143' TERM
trap cleanup EXIT

curl -fsS --max-time 2 "${HOT_HEALTH}" | jq -e '.ok == true' >/dev/null
install -m 0755 "${BINARY_SOURCE}" "${candidate_binary}"
cp -a "${ROLE_ENV}" "${candidate_env}"
set_env_value NANDO_REMOTE_EVIDENCE_SPOOL_ENABLED 1
set_env_value NANDO_REMOTE_EVIDENCE_SPOOL "${SPOOL_DIRECTORY}"
set_env_value NANDO_REMOTE_EVIDENCE_CLIENT_KEYS "${KEY_DIRECTORY}"

cp -a "${INSTALL_BINARY}" "${backup_binary}"
cp -a "${ROLE_ENV}" "${backup_env}"
if sudo -n test -f "${key_path}"; then
  sudo -n cp -a "${key_path}" "${backup_key}"
  key_existed=1
fi
if systemctl is-active --quiet "${SERVICE}"; then
  learner_was_active=1
  sudo -n systemctl stop "${SERVICE}"
fi
rollback_armed=1
sudo -n install -d -o root -g "${service_group}" -m 0750 "${KEY_DIRECTORY}"
if [[ "${client_key_already_installed}" == "0" ]]; then
  sudo -n install -o "${service_user}" -g "${service_group}" -m 0600 \
    "${CLIENT_KEY_SOURCE}" "${key_path}"
fi
sudo -n install -m 0644 "${candidate_env}" "${ROLE_ENV}"
sudo -n install -m 0755 "${candidate_binary}" "${INSTALL_BINARY}.candidate.$$"
sudo -n mv -f "${INSTALL_BINARY}.candidate.$$" "${INSTALL_BINARY}"
sudo -n install -d -o "${service_user}" -g "${service_group}" -m 0700 \
  "${SPOOL_DIRECTORY}"
sudo -n systemctl start "${SERVICE}"

wait_learning_ready
curl -fsS --max-time 2 "${HOT_HEALTH}" | jq -e '.ok == true' >/dev/null

rollback_armed=0
printf 'remote evidence spool ready; hot serving stayed online\n'
