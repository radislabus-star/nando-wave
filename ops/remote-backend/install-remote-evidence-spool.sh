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

work="$(mktemp -d)"
candidate_binary="${work}/nando-transition-serving"
candidate_env="${work}/response-learning.env"
backup_binary="${work}/previous-binary"
backup_env="${work}/previous-env"
backup_key="${work}/previous-key"
state_backup="${LEARNING_STATE}.rollback.$$"
key_existed=0
state_existed=0
learner_was_active=0
rollback_armed=0

set_env_value() {
  local key="$1"
  local value="$2"
  if grep -qE "^${key}=" "${candidate_env}"; then
    sed -i "s#^${key}=.*#${key}=${value}#" "${candidate_env}"
  else
    printf '%s=%s\n' "${key}" "${value}" >> "${candidate_env}"
  fi
}

wait_learning_ready() {
  local _attempt
  for _attempt in $(seq 1 "${READINESS_ATTEMPTS}"); do
    if curl -fsS --max-time "${LEARNING_HEALTH_TIMEOUT_SECONDS}" "${LEARNING_HEALTH}" \
      | jq -e '
          .ok == true
          and .remote_evidence.enabled == true
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
  sudo -n rm -rf "${state_backup}"
  rm -rf "${work}"
}

rollback() {
  local rc="${1:-1}"
  trap - ERR INT TERM EXIT
  set +e
  if [[ "${rollback_armed}" == "1" ]]; then
    sudo -n systemctl stop "${SERVICE}"
    sudo -n install -m 0755 "${backup_binary}" "${INSTALL_BINARY}"
    sudo -n install -m 0644 "${backup_env}" "${ROLE_ENV}"
    if [[ "${key_existed}" == "1" ]]; then
      sudo -n install -o "${service_user}" -g "${service_group}" -m 0600 \
        "${backup_key}" "${key_path}"
    else
      sudo -n rm -f "${key_path}"
    fi
    if [[ "${state_existed}" == "1" && -d "${state_backup}" ]]; then
      sudo -n rm -rf "${LEARNING_STATE}"
      sudo -n mv "${state_backup}" "${LEARNING_STATE}"
      sudo -n chown -R "${service_user}:${service_group}" "${LEARNING_STATE}"
    elif [[ "${state_existed}" == "0" ]]; then
      sudo -n rm -rf "${LEARNING_STATE}"
    fi
    if [[ "${learner_was_active}" == "1" ]]; then
      sudo -n systemctl start "${SERVICE}"
    fi
    printf 'remote evidence spool install failed; learner and state restored\n' >&2
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
if sudo -n test -d "${LEARNING_STATE}"; then
  sudo -n cp -a --reflink=auto "${LEARNING_STATE}" "${state_backup}"
  state_existed=1
fi

rollback_armed=1
sudo -n install -d -o root -g "${service_group}" -m 0750 "${KEY_DIRECTORY}"
sudo -n install -o "${service_user}" -g "${service_group}" -m 0600 \
  "${CLIENT_KEY_SOURCE}" "${key_path}"
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
