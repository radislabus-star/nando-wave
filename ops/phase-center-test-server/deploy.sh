#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODE="${NANDO_DEPLOY_MODE:-system}"
PREFIX="${NANDO_DEPLOY_PREFIX:-/opt/nando-wave}"
ENV_DIR="${NANDO_DEPLOY_ENV_DIR:-/etc/nando-wave}"
STATE_DIR="${NANDO_DEPLOY_STATE_DIR:-/var/lib/nando-wave}"
LOG_DIR="${NANDO_DEPLOY_LOG_DIR:-/var/log/nando-wave}"
SYSTEMD_DIR="${NANDO_DEPLOY_SYSTEMD_DIR:-/etc/systemd/system}"
ROLE_ENV_DIR="${ENV_DIR}/roles"
if [[ -n "${NANDO_DEPLOY_CARGO_BIN:-}" ]]; then
  CARGO_BIN="${NANDO_DEPLOY_CARGO_BIN}"
elif [[ -x "${HOME}/.cargo/bin/cargo" ]]; then
  CARGO_BIN="${HOME}/.cargo/bin/cargo"
else
  CARGO_BIN="$(command -v cargo)"
fi
CARGO=("${CARGO_BIN}")
if [[ -n "${NANDO_DEPLOY_RUST_TOOLCHAIN:-}" ]]; then
  CARGO+=("+${NANDO_DEPLOY_RUST_TOOLCHAIN}")
fi

if [[ "${1:-}" == "--user" ]]; then
  MODE="user"
  PREFIX="${NANDO_DEPLOY_PREFIX:-${HOME}/.local/opt/nando-wave}"
  ENV_DIR="${NANDO_DEPLOY_ENV_DIR:-${HOME}/.config/nando-wave}"
  STATE_DIR="${NANDO_DEPLOY_STATE_DIR:-${HOME}/.local/state/nando-wave}"
  LOG_DIR="${NANDO_DEPLOY_LOG_DIR:-${HOME}/.local/state/nando-wave/log}"
  SYSTEMD_DIR="${NANDO_DEPLOY_SYSTEMD_DIR:-${HOME}/.config/systemd/user}"
  ROLE_ENV_DIR="${ENV_DIR}/roles"
fi

if [[ "${1:-}" == "--help" ]]; then
  cat <<'EOF'
nando phase-center deploy

Usage:
  ops/phase-center-test-server/deploy.sh          # system deploy via sudo
  ops/phase-center-test-server/deploy.sh --user   # user-mode deploy

Environment overrides:
  NANDO_DEPLOY_PREFIX
  NANDO_DEPLOY_ENV_DIR
  NANDO_DEPLOY_STATE_DIR
  NANDO_DEPLOY_LOG_DIR
  NANDO_DEPLOY_SYSTEMD_DIR
  NANDO_DEPLOY_NANDO_CLI_BIN
  NANDO_DEPLOY_OWNER
  NANDO_DEPLOY_GROUP
  NANDO_DEPLOY_CODEX_SESSIONS_DIR
  NANDO_DEPLOY_GATEWAY_STATE_DIR
  NANDO_DEPLOY_GATEWAY_CONTROL_STATE_DIR
  NANDO_DEPLOY_CARGO_BIN
  NANDO_DEPLOY_RUST_TOOLCHAIN
  NANDO_DEPLOY_INSTALL_ONLY=1

Safety defaults:
  NANDO_LOCAL_ACCEPT_ENABLED=0
  NANDO_CLIENT_ALLOW_LOCAL_ACCEPT=0
  NANDO_CLIENT_SAFETY_POLICY=shadow_only

Set NANDO_DEPLOY_OVERWRITE_ENV=1 only when you want to replace the server
policy file from the packaged example. By default an existing server policy is
preserved and only missing keys are appended.
EOF
  exit 0
fi

if [[ "${MODE}" == "system" ]]; then
  SUDO=(sudo -n)
  SYSTEMCTL=(sudo -n systemctl)
  INSTALL_BIN_DIR="/usr/local/bin"
  DEPLOY_OWNER="${NANDO_DEPLOY_OWNER:-${SUDO_USER:-$(id -un)}}"
  DEPLOY_GROUP="${NANDO_DEPLOY_GROUP:-$(id -gn "${DEPLOY_OWNER}" 2>/dev/null || id -gn)}"
  GATEWAY_STATE_DIR="${NANDO_DEPLOY_GATEWAY_STATE_DIR:-/var/lib/nando-gateway}"
  GATEWAY_CONTROL_STATE_DIR="${NANDO_DEPLOY_GATEWAY_CONTROL_STATE_DIR:-/var/lib/nando-gateway-control}"
else
  SUDO=()
  SYSTEMCTL=(systemctl --user)
  INSTALL_BIN_DIR="${HOME}/.local/bin"
  DEPLOY_OWNER="$(id -un)"
  DEPLOY_GROUP="$(id -gn)"
  GATEWAY_STATE_DIR="${NANDO_DEPLOY_GATEWAY_STATE_DIR:-${STATE_DIR}/gateway}"
  GATEWAY_CONTROL_STATE_DIR="${NANDO_DEPLOY_GATEWAY_CONTROL_STATE_DIR:-${STATE_DIR}/gateway-control}"
fi

DEPLOY_HOME="$(getent passwd "${DEPLOY_OWNER}" 2>/dev/null | cut -d: -f6)"
DEPLOY_HOME="${DEPLOY_HOME:-${HOME}}"
CODEX_SESSIONS_DIR="${NANDO_DEPLOY_CODEX_SESSIONS_DIR:-${DEPLOY_HOME}/.codex/sessions}"

if [[ ! -x "${CARGO_BIN}" ]]; then
  echo "cargo is not executable: ${CARGO_BIN}" >&2
  exit 2
fi

echo "build release typed-transition runtime with $("${CARGO[@]}" --version)..."
"${CARGO[@]}" build --release -q -p nando-transition-inducer --bins
"${CARGO[@]}" build --release -q -p nando-transition-serving -p nando-response-actor -p nando-gateway-control --bins
NANDO_TRANSITION_EXEC_SRC="${ROOT_DIR}/target/release/nando-transition-live-exec"
NANDO_TRANSITION_INSPECT_SRC="${ROOT_DIR}/target/release/nando-transition-admission-inspect"
NANDO_TRANSITION_SERVING_SRC="${ROOT_DIR}/target/release/nando-transition-serving"
NANDO_GATEWAY_CONTROL_SRC="${ROOT_DIR}/target/release/nando-gateway-control"

echo "nando phase-center deploy:"
echo "  mode=${MODE}"
echo "  root=${ROOT_DIR}"
echo "  prefix=${PREFIX}"
echo "  env_dir=${ENV_DIR}"
echo "  state_dir=${STATE_DIR}"
echo "  systemd_dir=${SYSTEMD_DIR}"
echo "  writable_owner=${DEPLOY_OWNER}:${DEPLOY_GROUP}"
echo "  codex_sessions_dir=${CODEX_SESSIONS_DIR}"

if [[ -n "${NANDO_DEPLOY_NANDO_CLI_BIN:-}" ]]; then
  if [[ ! -x "${NANDO_DEPLOY_NANDO_CLI_BIN}" ]]; then
    echo "NANDO_DEPLOY_NANDO_CLI_BIN is not executable: ${NANDO_DEPLOY_NANDO_CLI_BIN}" >&2
    exit 2
  fi
  NANDO_CLI_SRC="${NANDO_DEPLOY_NANDO_CLI_BIN}"
  echo "use prebuilt nando-cli: ${NANDO_CLI_SRC}"
else
  echo "build release nando-cli..."
  "${CARGO[@]}" build --release -q -p nando-cli
  NANDO_CLI_SRC="${ROOT_DIR}/target/release/nando-cli"
fi

INSTALL_ONLY="${NANDO_DEPLOY_INSTALL_ONLY:-0}"

echo "install files..."
"${SUDO[@]}" mkdir -p \
  "${PREFIX}/bin" \
  "${PREFIX}/crates/nando-transition-inducer/src" \
  "${PREFIX}/ops" \
  "${PREFIX}/data" \
  "${ENV_DIR}" \
  "${ROLE_ENV_DIR}" \
  "${STATE_DIR}/streaming" \
  "${STATE_DIR}/transition/package-inbox" \
  "${STATE_DIR}/provider-export-drop" \
  "${LOG_DIR}" \
  "${SYSTEMD_DIR}" \
  "${INSTALL_BIN_DIR}"

"${SUDO[@]}" chown -R "${DEPLOY_OWNER}:${DEPLOY_GROUP}" "${STATE_DIR}" "${LOG_DIR}"
"${SUDO[@]}" chmod -R u+rwX,go-rwx "${STATE_DIR}" "${LOG_DIR}"

NANDO_CLI_DEST="${PREFIX}/bin/nando-cli"
if [[ "$(readlink -f "${NANDO_CLI_SRC}")" != "$(readlink -f "${NANDO_CLI_DEST}" 2>/dev/null || true)" ]]; then
  "${SUDO[@]}" install -m 0755 "${NANDO_CLI_SRC}" "${NANDO_CLI_DEST}"
else
  echo "skip nando-cli install: source already at ${NANDO_CLI_DEST}"
fi
"${SUDO[@]}" install -m 0755 "${NANDO_TRANSITION_EXEC_SRC}" "${PREFIX}/bin/nando-transition-live-exec"
"${SUDO[@]}" install -m 0755 "${NANDO_TRANSITION_INSPECT_SRC}" "${PREFIX}/bin/nando-transition-admission-inspect"
"${SUDO[@]}" install -m 0755 "${NANDO_TRANSITION_SERVING_SRC}" "${PREFIX}/bin/nando-transition-serving"
"${SUDO[@]}" install -m 0755 "${NANDO_GATEWAY_CONTROL_SRC}" "${PREFIX}/bin/nando-gateway-control"
"${SUDO[@]}" rm -f \
  "${PREFIX}/bin/nando-transition-profile-daemon" \
  "${PREFIX}/bin/nando-response-miner" \
  "${PREFIX}/bin/nando-response-online-miner"
for proof_source in causal_proof.rs wave.rs induce.rs; do
  "${SUDO[@]}" install -m 0644 \
    "${ROOT_DIR}/crates/nando-transition-inducer/src/${proof_source}" \
    "${PREFIX}/crates/nando-transition-inducer/src/${proof_source}"
done
"${SUDO[@]}" rm -rf "${PREFIX}/ops/phase-center-test-server"
"${SUDO[@]}" mkdir -p "${PREFIX}/ops"
"${SUDO[@]}" cp -R "${ROOT_DIR}/ops/phase-center-test-server" "${PREFIX}/ops/"
"${SUDO[@]}" find "${PREFIX}/ops/phase-center-test-server" -type f -name '*.py' -delete
"${SUDO[@]}" find "${PREFIX}/ops/phase-center-test-server" -type d -name '__pycache__' -prune -exec rm -rf {} +
"${SUDO[@]}" rm -rf "${PREFIX}/data/real_traffic"
"${SUDO[@]}" mkdir -p "${PREFIX}/data"
if [[ -d "${ROOT_DIR}/data/real_traffic" ]]; then
  "${SUDO[@]}" cp -R "${ROOT_DIR}/data/real_traffic" "${PREFIX}/data/"
else
  "${SUDO[@]}" mkdir -p "${PREFIX}/data/real_traffic"
fi
"${SUDO[@]}" find "${PREFIX}/ops/phase-center-test-server/bin" -type f \( -name '*.sh' -o -name '*.py' -o -name 'nando-codex' -o -name 'nando-codex-hook' -o -name 'codex' \) -exec chmod 0755 {} \;
"${SUDO[@]}" ln -sf \
  "${PREFIX}/ops/phase-center-test-server/bin/nando-live-transition-gate" \
  "${PREFIX}/bin/nando-live-transition-gate"
"${SUDO[@]}" ln -sf "${PREFIX}/ops/phase-center-test-server/bin/nando-llm-gateway.sh" "${INSTALL_BIN_DIR}/nando-llm-gateway"
"${SUDO[@]}" ln -sf "${PREFIX}/ops/phase-center-test-server/bin/nando-codex" "${INSTALL_BIN_DIR}/nando-codex"
"${SUDO[@]}" ln -sf "${PREFIX}/ops/phase-center-test-server/bin/codex" "${INSTALL_BIN_DIR}/codex"
"${SUDO[@]}" ln -sf "${PREFIX}/bin/nando-live-transition-gate" "${INSTALL_BIN_DIR}/nando-live-transition-gate"
"${SUDO[@]}" ln -sf "${PREFIX}/ops/phase-center-test-server/bin/nando-transition" "${INSTALL_BIN_DIR}/nando-transition"

ENV_FILE="${ENV_DIR}/phase-center.env"
tmp_env="$(mktemp)"
sed \
  -e "s#^NANDO_BIN=.*#NANDO_BIN=${PREFIX}/bin/nando-cli#" \
  -e "s#^NANDO_CODEX_SESSIONS_DIR=.*#NANDO_CODEX_SESSIONS_DIR=${CODEX_SESSIONS_DIR}#" \
  -e "s#^NANDO_STATE_DIR=.*#NANDO_STATE_DIR=${STATE_DIR}/streaming#" \
  -e "s#^NANDO_LOG_DIR=.*#NANDO_LOG_DIR=${LOG_DIR}#" \
  -e "s#^NANDO_PROVIDER_EXPORT_DROP_DIR=.*#NANDO_PROVIDER_EXPORT_DROP_DIR=${STATE_DIR}/provider-export-drop#" \
  -e "s#^NANDO_PROVIDER_EXPORT_JSONL=.*#NANDO_PROVIDER_EXPORT_JSONL=${STATE_DIR}/provider-export-drop/provider-export.external.jsonl#" \
  -e "s#^NANDO_PHASE_CENTER_OPS_DIR=.*#NANDO_PHASE_CENTER_OPS_DIR=${PREFIX}/ops/phase-center-test-server#" \
  -e "s#^NANDO_SYSTEMD_DIR=.*#NANDO_SYSTEMD_DIR=${SYSTEMD_DIR}#" \
  -e "s#^NANDO_GATEWAY_LOCAL_CMD=.*#NANDO_GATEWAY_LOCAL_CMD=${PREFIX}/ops/phase-center-test-server/bin/nando-llm-local-executor.sh#" \
  -e "s#^NANDO_TRANSITION_STATE_DIR=.*#NANDO_TRANSITION_STATE_DIR=${STATE_DIR}/transition#" \
  -e "s#^NANDO_TRANSITION_TRACE_JSONL=.*#NANDO_TRANSITION_TRACE_JSONL=${STATE_DIR}/transition/live-transitions.jsonl#" \
  -e "s#^NANDO_LEGACY_JSON_AUDIT_ENABLED=.*#NANDO_LEGACY_JSON_AUDIT_ENABLED=0#" \
  -e "s#^NANDO_RESPONSE_ONLINE_CHECKPOINT=.*#NANDO_RESPONSE_ONLINE_CHECKPOINT=${STATE_DIR}/transition/response-online-miner.checkpoint#" \
  -e "s#^NANDO_STREAMING_EVIDENCE_DIR=.*#NANDO_STREAMING_EVIDENCE_DIR=${STATE_DIR}/transition/streaming-evidence-v2#" \
  -e "s#^NANDO_TRANSITION_PACKAGE_INBOX=.*#NANDO_TRANSITION_PACKAGE_INBOX=${STATE_DIR}/transition/package-inbox#" \
  -e "s#^NANDO_TRANSITION_REGISTRY=.*#NANDO_TRANSITION_REGISTRY=${STATE_DIR}/transition/registry.json#" \
  -e "s#^NANDO_TRANSITION_METRICS=.*#NANDO_TRANSITION_METRICS=${STATE_DIR}/transition/metrics.json#" \
  -e "s#^NANDO_TRANSITION_EXECUTOR=.*#NANDO_TRANSITION_EXECUTOR=${PREFIX}/bin/nando-transition-live-exec#" \
  -e "s#^NANDO_ECONOMICS_SNAPSHOT_JSON=.*#NANDO_ECONOMICS_SNAPSHOT_JSON=${STATE_DIR}/transition/economics-live.json#" \
  "${ROOT_DIR}/ops/phase-center-test-server/nando-phase-center.env.example" > "${tmp_env}"

if [[ ! -f "${ENV_FILE}" || "${NANDO_DEPLOY_OVERWRITE_ENV:-0}" == "1" ]]; then
  if [[ "${MODE}" == "system" ]]; then
    "${SUDO[@]}" install -m 0600 "${tmp_env}" "${ENV_FILE}"
  else
    install -m 0600 "${tmp_env}" "${ENV_FILE}"
  fi
else
  merged_env="$(mktemp)"
  if [[ "${MODE}" == "system" ]]; then
    "${SUDO[@]}" cat "${ENV_FILE}" > "${merged_env}"
  else
    cp "${ENV_FILE}" "${merged_env}"
  fi
  while IFS='=' read -r key value; do
    [[ -z "${key}" || "${key}" == \#* ]] && continue
    if ! grep -qE "^${key}=" "${merged_env}"; then
      printf '%s=%s\n' "${key}" "${value}" >> "${merged_env}"
    fi
  done < "${tmp_env}"
  for key in \
    NANDO_CODEX_SESSIONS_DIR \
    NANDO_LEGACY_JSON_AUDIT_ENABLED \
    NANDO_RESPONSE_ONLINE_CHECKPOINT \
    NANDO_STREAMING_EVIDENCE_DIR \
    NANDO_ECONOMICS_SNAPSHOT_JSON
  do
    value="$(grep -E "^${key}=" "${tmp_env}" | tail -n 1 | cut -d= -f2-)"
    if grep -qE "^${key}=" "${merged_env}"; then
      sed -i "s#^${key}=.*#${key}=${value}#" "${merged_env}"
    else
      printf '%s=%s\n' "${key}" "${value}" >> "${merged_env}"
    fi
  done
  sed -i \
    -e '/^NANDO_TRANSITION_EXECUTION_EVENTS_JSONL=/d' \
    -e '/^NANDO_ECONOMICS_LEDGER_JSONL=/d' \
    -e '/^NANDO_DETERMINISTIC_EVIDENCE_LEDGER=/d' \
    -e '/^NANDO_M3_WINDOW_EVALUATOR=/d' \
    "${merged_env}"
  if [[ "${MODE}" == "system" ]]; then
    "${SUDO[@]}" install -m 0600 "${merged_env}" "${ENV_FILE}"
  else
    install -m 0600 "${merged_env}" "${ENV_FILE}"
  fi
  rm -f "${merged_env}"
fi
rm -f "${tmp_env}"

env_value() {
  if [[ "${MODE}" == "system" ]]; then
    "${SUDO[@]}" grep -E "^$1=" "${ENV_FILE}" 2>/dev/null | tail -n 1 | cut -d= -f2-
  else
    grep -E "^$1=" "${ENV_FILE}" | tail -n 1 | cut -d= -f2-
  fi
}

random_hex_key() {
  if command -v openssl >/dev/null 2>&1; then
    openssl rand -hex 32
  else
    od -An -N32 -tx1 /dev/urandom | tr -d ' \n'
  fi
}

ensure_secret_env_key() {
  local key="$1"
  local current
  current="$(env_value "${key}")"
  if [[ -n "${current}" ]]; then
    return
  fi
  local value merged_env
  value="$(random_hex_key)"
  merged_env="$(mktemp)"
  if [[ "${MODE}" == "system" ]]; then
    "${SUDO[@]}" cat "${ENV_FILE}" > "${merged_env}"
  else
    cp "${ENV_FILE}" "${merged_env}"
  fi
  if grep -qE "^${key}=" "${merged_env}"; then
    sed -i "s#^${key}=.*#${key}=${value}#" "${merged_env}"
  else
    printf '%s=%s\n' "${key}" "${value}" >> "${merged_env}"
  fi
  if [[ "${MODE}" == "system" ]]; then
    "${SUDO[@]}" install -m 0600 "${merged_env}" "${ENV_FILE}"
  else
    install -m 0600 "${merged_env}" "${ENV_FILE}"
  fi
  rm -f "${merged_env}"
}

ensure_secret_env_key "NANDO_STATUS_DASHBOARD_KEY"

role_envs=(
  gateway-control.env
  response-learning.env
  transition-serving.env
)

for role_env in "${role_envs[@]}"; do
  rendered_role_env="$(mktemp)"
  sed \
    -e "s#/opt/nando-wave#${PREFIX}#g" \
    -e "s#/var/lib/nando-gateway-control#${GATEWAY_CONTROL_STATE_DIR}#g" \
    -e "s#/var/lib/nando-gateway#${GATEWAY_STATE_DIR}#g" \
    -e "s#/var/lib/nando-wave#${STATE_DIR}#g" \
    "${ROOT_DIR}/ops/phase-center-test-server/roles/${role_env}" \
    > "${rendered_role_env}"
  "${SUDO[@]}" install -m 0644 \
    "${rendered_role_env}" \
    "${ROLE_ENV_DIR}/${role_env}"
  rm -f "${rendered_role_env}"
done

WATERMARK_TRACE_JSONL="$(env_value NANDO_WATERMARK_TRACE_JSONL)"
APPEND_JSONL="$(env_value NANDO_APPEND_JSONL)"
if [[ -n "${WATERMARK_TRACE_JSONL}" && -n "${APPEND_JSONL}" ]]; then
  "${SUDO[@]}" mkdir -p "$(dirname "${WATERMARK_TRACE_JSONL}")" "$(dirname "${APPEND_JSONL}")"
  "${SUDO[@]}" touch "${WATERMARK_TRACE_JSONL}" "${APPEND_JSONL}"
  "${SUDO[@]}" chown "${DEPLOY_OWNER}:${DEPLOY_GROUP}" "${WATERMARK_TRACE_JSONL}" "${APPEND_JSONL}"
  "${SUDO[@]}" chmod 0600 "${WATERMARK_TRACE_JSONL}" "${APPEND_JSONL}"
fi

echo "install systemd units..."
obsolete_units=(
  nando-phase-center-appender.service
  nando-phase-center-live-tail.service
  nando-phase-center-codex-capture.service
  nando-phase-center-codex-capture.timer
  nando-transition-profile-daemon.service
  nando-response-miner.service
  nando-response-miner.timer
  nando-response-online-miner.service
  nando-live-transition-gate.service
  nando-live-transition-gate.timer
  nando-live-transition-gate.path
  nando-phase-center-metrics-snapshot.service
  nando-phase-center-metrics-snapshot.timer
  nando-phase-center-provider-evidence-snapshot.timer
  nando-phase-center-provider-export-contract-pack.timer
  nando-phase-center-readiness-snapshot.timer
  nando-phase-center-test-server-verify.timer
  nando-phase-center-status.timer
  nando-phase-center-local-accept-promotion-gate.timer
  nando-phase-center-provider-activation-gate.timer
  nando-phase-center-provider-export-watch.timer
)
if [[ "${INSTALL_ONLY}" != "1" ]]; then
  for unit in "${obsolete_units[@]}"; do
    "${SYSTEMCTL[@]}" disable --now "${unit}" >/dev/null 2>&1 || true
  done
fi
service_units=(
  nando-gateway-control.service
  nando-response-learning.service
  nando-transition-serving.service
)

install_service_unit() {
  local unit="$1"
  local source="${ROOT_DIR}/ops/phase-center-test-server/systemd/${unit}"
  local destination="${SYSTEMD_DIR}/${unit}"

  "${SUDO[@]}" install -m 0644 "${source}" "${destination}"
  if [[ "${MODE}" == "system" ]]; then
    "${SUDO[@]}" sed -i \
      -e "s#^User=.*#User=${DEPLOY_OWNER}#" \
      -e "s#^Group=.*#Group=${DEPLOY_GROUP}#" \
      -e "s#/etc/nando-wave/phase-center.env#${ENV_FILE}#g" \
      -e "s#/etc/nando-wave/roles#${ROLE_ENV_DIR}#g" \
      -e "s#/etc/nando-wave/authority.env#${ENV_DIR}/authority.env#g" \
      -e "s#^ReadOnlyPaths=.*\\.codex/sessions#ReadOnlyPaths=-${CODEX_SESSIONS_DIR}#" \
      "${destination}"
  else
    sed -i \
      -e '/^User=/d' \
      -e '/^Group=/d' \
      -e '/^SupplementaryGroups=/d' \
      -e "s#/etc/nando-wave/phase-center.env#${ENV_FILE}#g" \
      -e "s#/etc/nando-wave/roles#${ROLE_ENV_DIR}#g" \
      -e "s#/etc/nando-wave/authority.env#${ENV_DIR}/authority.env#g" \
      -e "s#WorkingDirectory=/opt/nando-wave#WorkingDirectory=${PREFIX}#" \
      -e "s#/var/lib/nando-gateway-control#${STATE_DIR}/gateway-control#g" \
      -e "s#/var/lib/nando-wave#${STATE_DIR}#g" \
      -e "s#^ReadOnlyPaths=.*\\.codex/sessions#ReadOnlyPaths=-${CODEX_SESSIONS_DIR}#" \
      -e "s#/opt/nando-wave#${PREFIX}#g" \
      "${destination}"
  fi
}

for unit in "${service_units[@]}"; do
  install_service_unit "${unit}"
done

if [[ "${MODE}" == "system" ]]; then
  for unit in "${obsolete_units[@]}"; do
    "${SUDO[@]}" rm -f "${SYSTEMD_DIR}/${unit}"
  done
  if [[ "${INSTALL_ONLY}" != "1" ]]; then
    "${SYSTEMCTL[@]}" daemon-reload
  fi
else
  if [[ "${INSTALL_ONLY}" != "1" ]]; then
    "${SYSTEMCTL[@]}" daemon-reload
  fi
fi

if [[ "${INSTALL_ONLY}" == "1" ]]; then
  echo "install-only mode: systemd enable/start and smoke skipped"
  echo "installed:"
  echo "  env=${ENV_FILE}"
  echo "  gateway=${INSTALL_BIN_DIR}/nando-llm-gateway"
  echo "  nando_cli=${PREFIX}/bin/nando-cli"
  exit 0
fi

echo "enable services..."
units=(
  nando-gateway-control.service
  nando-response-learning.service
  nando-transition-serving.service
)

for unit in "${units[@]}"; do
  "${SYSTEMCTL[@]}" enable --now "${unit}" >/dev/null
done

long_running_services=(
  nando-gateway-control.service
  nando-response-learning.service
  nando-transition-serving.service
)

for unit in "${long_running_services[@]}"; do
  "${SYSTEMCTL[@]}" restart "${unit}" >/dev/null
done

BRIDGE_BIND_VALUE="$(env_value NANDO_PROVIDER_BRIDGE_BIND)"
BRIDGE_BIND_VALUE="${BRIDGE_BIND_VALUE:-127.0.0.1:8787}"
if command -v curl >/dev/null 2>&1; then
  for _attempt in $(seq 1 30); do
    if curl -fsS --max-time 1 "http://${BRIDGE_BIND_VALUE}/health" >/dev/null 2>&1; then
      break
    fi
    sleep 0.2
  done
fi

echo "run smoke..."
curl -fsS --max-time 2 "http://${BRIDGE_BIND_VALUE}/health" >/dev/null
curl -fsS --max-time 2 "http://${BRIDGE_BIND_VALUE}/cpu-health" >/dev/null

echo "deployed:"
echo "  env=${ENV_FILE}"
echo "  gateway=${INSTALL_BIN_DIR}/nando-llm-gateway"
echo "  client_env=$("${SUDO[@]}" "${PREFIX}/ops/phase-center-test-server/bin/nando-phase-center-client-env.sh" "${ENV_FILE}" status | jq -r '.openai_base_url')"
echo "  verify=${STATE_DIR}/streaming/metrics/nando-phase-center.test-server-verify.json"
echo "  local_accept_default=$(env_value NANDO_LOCAL_ACCEPT_ENABLED)"
echo "  client_allow_local_accept=$(env_value NANDO_CLIENT_ALLOW_LOCAL_ACCEPT)"
