#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TEMPLATE="${ROOT_DIR}/ops/remote-backend/nginx-lan.conf.template"
UNIT_SOURCE="${ROOT_DIR}/ops/remote-backend/nando-transport-gateway.service"
NGINX_DIR="${NANDO_REMOTE_NGINX_DIR:-/etc/nando-gateway}"
SYSTEMD_DIR="${NANDO_REMOTE_SYSTEMD_DIR:-/etc/systemd/system}"
INSTALL_ONLY="${NANDO_REMOTE_INSTALL_ONLY:-0}"
DISCOVER_DNS_ONLY="${NANDO_REMOTE_DISCOVER_DNS_ONLY:-0}"
LAN_BIND="${NANDO_REMOTE_LAN_BIND:-}"
LAN_ALLOW="${NANDO_REMOTE_LAN_ALLOW:-}"
DNS_RESOLVERS="${NANDO_REMOTE_DNS_RESOLVERS:-}"
DNS_PROBE_NAME="${NANDO_REMOTE_DNS_PROBE_NAME:-chatgpt.com}"
UPSTREAM_IPV4S="${NANDO_REMOTE_UPSTREAM_IPV4S:-}"
RESOLVED_STUB="${NANDO_REMOTE_RESOLVED_STUB:-/run/systemd/resolve/stub-resolv.conf}"
RESOLVER_FILE="${NANDO_REMOTE_RESOLVER_FILE:-/run/systemd/resolve/resolv.conf}"

usage() {
  cat <<'EOF'
Install the private-LAN Nando Nginx edge.

Usage:
  ops/remote-backend/install-lan-edge.sh \
    --bind 192.168.3.94:8787 \
    --allow 192.168.3.0/24

Environment:
  NANDO_REMOTE_INSTALL_ONLY=1   install and validate without starting
  NANDO_REMOTE_DISCOVER_DNS_ONLY=1
                                print the selected resolvers and exit
  NANDO_REMOTE_DNS_RESOLVERS    space-separated IPv4 resolvers
  NANDO_REMOTE_DNS_PROBE_NAME   default: chatgpt.com
  NANDO_REMOTE_UPSTREAM_IPV4S   two space-separated upstream IPv4 addresses
  NANDO_REMOTE_NGINX_DIR        default: /etc/nando-gateway
  NANDO_REMOTE_SYSTEMD_DIR      default: /etc/systemd/system
EOF
}

dns_query_works() {
  local resolver="$1"
  local answer

  if command -v dig >/dev/null 2>&1; then
    answer="$(
      dig "@${resolver}" "${DNS_PROBE_NAME}" A \
        +short +time=2 +tries=1 2>/dev/null
    )" || return 1
    grep -Eq '^[0-9]{1,3}(\.[0-9]{1,3}){3}$' <<<"${answer}"
    return
  fi

  if [[ "${resolver}" == "127.0.0.53" ]] \
    && command -v resolvectl >/dev/null 2>&1; then
    resolvectl query --type=A "${DNS_PROBE_NAME}" >/dev/null 2>&1
    return
  fi

  return 1
}

discover_dns_resolvers() {
  local resolver_file

  if command -v systemctl >/dev/null 2>&1 \
    && systemctl is-active --quiet systemd-resolved.service \
    && [[ -r "${RESOLVED_STUB}" ]] \
    && dns_query_works 127.0.0.53; then
    printf '%s\n' "127.0.0.53"
    return
  fi

  resolver_file="${RESOLVER_FILE}"
  if [[ ! -r "${resolver_file}" ]]; then
    resolver_file="/etc/resolv.conf"
  fi
  awk '/^nameserver[[:space:]]+/ && !seen[$2]++ {print $2}' "${resolver_file}" \
    | paste -sd ' ' -
}

discover_upstream_ipv4s() {
  local resolver="$1"

  if command -v dig >/dev/null 2>&1; then
    dig "@${resolver}" "${DNS_PROBE_NAME}" A \
      +short +time=2 +tries=1 2>/dev/null \
      | awk '
          count < 2 && /^[0-9]{1,3}(\.[0-9]{1,3}){3}$/ && !seen[$0]++ {
            print
            ++count
          }
        ' \
      | paste -sd ' ' -
    return
  fi

  getent ahostsv4 "${DNS_PROBE_NAME}" \
    | awk 'count < 2 && !seen[$1]++ {print $1; ++count}' \
    | paste -sd ' ' -
}

wait_http_ready() {
  local url="$1"
  local attempt
  local attempts="${NANDO_REMOTE_READINESS_ATTEMPTS:-20}"
  local sleep_seconds="${NANDO_REMOTE_READINESS_SLEEP_SECONDS:-0.25}"

  for ((attempt = 1; attempt <= attempts; attempt++)); do
    if curl -fsS --max-time 1 "${url}" >/dev/null 2>&1; then
      return 0
    fi
    sleep "${sleep_seconds}"
  done
  printf 'HTTP readiness timed out: %s\n' "${url}" >&2
  return 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bind)
      LAN_BIND="${2:-}"
      shift 2
      ;;
    --allow)
      LAN_ALLOW="${2:-}"
      shift 2
      ;;
    --resolver)
      DNS_RESOLVERS="${2:-}"
      shift 2
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ ! "${LAN_BIND}" =~ ^[0-9]{1,3}(\.[0-9]{1,3}){3}:[0-9]{2,5}$ ]]; then
  echo "--bind must be an IPv4 address and port" >&2
  exit 2
fi
if [[ ! "${LAN_ALLOW}" =~ ^[0-9]{1,3}(\.[0-9]{1,3}){3}/[0-9]{1,2}$ ]]; then
  echo "--allow must be an IPv4 CIDR" >&2
  exit 2
fi

if [[ -z "${DNS_RESOLVERS}" ]]; then
  DNS_RESOLVERS="$(discover_dns_resolvers)"
fi
for resolver in ${DNS_RESOLVERS}; do
  if [[ ! "${resolver}" =~ ^[0-9]{1,3}(\.[0-9]{1,3}){3}$ ]]; then
    echo "resolver must be an IPv4 address: ${resolver}" >&2
    exit 2
  fi
done
if [[ -z "${DNS_RESOLVERS}" ]]; then
  echo "no IPv4 DNS resolver found" >&2
  exit 2
fi
if [[ "${DISCOVER_DNS_ONLY}" == "1" ]]; then
  printf '%s\n' "${DNS_RESOLVERS}"
  exit 0
fi
if [[ -z "${UPSTREAM_IPV4S}" ]]; then
  UPSTREAM_IPV4S="$(discover_upstream_ipv4s "${DNS_RESOLVERS%% *}")"
fi
read -r upstream_primary upstream_secondary upstream_extra <<<"${UPSTREAM_IPV4S}"
if [[ -n "${upstream_extra:-}" ]] \
  || [[ ! "${upstream_primary:-}" =~ ^[0-9]{1,3}(\.[0-9]{1,3}){3}$ ]] \
  || [[ ! "${upstream_secondary:-}" =~ ^[0-9]{1,3}(\.[0-9]{1,3}){3}$ ]] \
  || [[ "${upstream_primary}" == "${upstream_secondary}" ]]; then
  printf '%s\n' "exactly two distinct upstream IPv4 addresses are required" >&2
  exit 2
fi
if ! command -v nginx >/dev/null 2>&1; then
  echo "nginx is not installed" >&2
  exit 2
fi

rendered="$(mktemp)"
candidate_config="${NGINX_DIR}/.nginx.conf.candidate.$$"
candidate_unit="${SYSTEMD_DIR}/.nando-transport-gateway.service.candidate.$$"
backup_config="${NGINX_DIR}/.nginx.conf.rollback.$$"
backup_unit="${SYSTEMD_DIR}/.nando-transport-gateway.service.rollback.$$"
had_config=0
had_unit=0
rollback_armed=0
service_was_active=0
service_was_enabled=0

cleanup() {
  set +e
  sudo -n rm -f \
    "${candidate_config}" \
    "${candidate_unit}" \
    "${backup_config}" \
    "${backup_unit}"
  rm -f "${rendered}"
}

rollback() {
  local rc="${1:-1}"

  trap - ERR INT TERM EXIT
  set +e

  if [[ "${rollback_armed}" == "1" ]]; then
    if [[ "${service_was_active}" == "0" ]]; then
      sudo -n systemctl stop nando-transport-gateway.service
    fi
    if [[ "${service_was_enabled}" == "0" ]]; then
      sudo -n systemctl disable nando-transport-gateway.service
    fi

    if [[ "${had_config}" == "1" ]]; then
      sudo -n mv -f "${backup_config}" "${NGINX_DIR}/nginx.conf"
    else
      sudo -n rm -f "${NGINX_DIR}/nginx.conf"
    fi
    if [[ "${had_unit}" == "1" ]]; then
      sudo -n mv -f \
        "${backup_unit}" \
        "${SYSTEMD_DIR}/nando-transport-gateway.service"
    else
      sudo -n rm -f "${SYSTEMD_DIR}/nando-transport-gateway.service"
    fi

    sudo -n systemctl daemon-reload
    if [[ "${service_was_active}" == "1" ]]; then
      sudo -n systemctl reload nando-transport-gateway.service
    fi
    printf '%s\n' "LAN edge deployment failed; previous configuration restored" >&2
  fi

  cleanup
  exit "${rc}"
}

trap 'rollback $?' ERR
trap 'rollback 130' INT
trap 'rollback 143' TERM
trap cleanup EXIT

sed \
  -e "s#@NANDO_LAN_BIND@#${LAN_BIND}#g" \
  -e "s#@NANDO_LAN_CIDR@#${LAN_ALLOW}#g" \
  -e "s#@NANDO_DNS_RESOLVERS@#${DNS_RESOLVERS}#g" \
  -e "s#@NANDO_CHATGPT_IPV4_PRIMARY@#${upstream_primary}#g" \
  -e "s#@NANDO_CHATGPT_IPV4_SECONDARY@#${upstream_secondary}#g" \
  -e "s#@NANDO_NGINX_DIR@#${NGINX_DIR}#g" \
  "${TEMPLATE}" > "${rendered}"

sudo -n install -d -o root -g root -m 0755 \
  "${NGINX_DIR}" \
  "${NGINX_DIR}/server.d" \
  "${SYSTEMD_DIR}"
sudo -n install -d -o www-data -g www-data -m 0750 \
  /run/nando-gateway \
  /run/nando-gateway/client-body \
  /run/nando-gateway/proxy \
  /var/lib/nando-gateway \
  /var/lib/nando-gateway/client-body \
  /var/lib/nando-gateway/proxy
sudo -n touch /var/lib/nando-gateway/economics-access.jsonl
sudo -n chown www-data:www-data /var/lib/nando-gateway/economics-access.jsonl
sudo -n chmod 0640 /var/lib/nando-gateway/economics-access.jsonl
sudo -n install -m 0644 \
  "${rendered}" \
  "${candidate_config}"
sudo -n install -m 0644 \
  "${UNIT_SOURCE}" \
  "${candidate_unit}"
sudo -n -u www-data nginx -t -c "${candidate_config}"
sudo -n chown www-data:www-data /run/nando-gateway

if sudo -n test -e "${NGINX_DIR}/nginx.conf"; then
  sudo -n cp -a "${NGINX_DIR}/nginx.conf" "${backup_config}"
  had_config=1
fi
if sudo -n test -e "${SYSTEMD_DIR}/nando-transport-gateway.service"; then
  sudo -n cp -a \
    "${SYSTEMD_DIR}/nando-transport-gateway.service" \
    "${backup_unit}"
  had_unit=1
fi
if sudo -n systemctl is-active --quiet nando-transport-gateway.service; then
  service_was_active=1
fi
if sudo -n systemctl is-enabled --quiet nando-transport-gateway.service; then
  service_was_enabled=1
fi

rollback_armed=1
sudo -n mv -f "${candidate_config}" "${NGINX_DIR}/nginx.conf"
sudo -n mv -f \
  "${candidate_unit}" \
  "${SYSTEMD_DIR}/nando-transport-gateway.service"
sudo -n systemctl daemon-reload

if [[ "${INSTALL_ONLY}" == "1" ]]; then
  rollback_armed=0
  sudo -n rm -f "${backup_config}" "${backup_unit}"
  echo "LAN edge installed and validated; service start skipped"
  echo "DNS resolvers: ${DNS_RESOLVERS}"
  exit 0
fi

if [[ "${service_was_active}" == "1" ]]; then
  sudo -n systemctl reload nando-transport-gateway.service
else
  sudo -n systemctl enable --now nando-transport-gateway.service
fi

wait_http_ready "http://${LAN_BIND}/health"
wait_http_ready "http://${LAN_BIND}/client-fallback-health"
wait_http_ready "http://${LAN_BIND}/cpu-health"
wait_http_ready "http://${LAN_BIND}/control-health"
dns_query_works "${DNS_RESOLVERS%% *}"
curl -sS -o /dev/null --connect-timeout 5 --max-time 10 \
  "https://${DNS_PROBE_NAME}/"

rollback_armed=0
sudo -n rm -f "${backup_config}" "${backup_unit}"
echo "LAN edge ready: http://${LAN_BIND}"
echo "DNS resolvers: ${DNS_RESOLVERS}"
