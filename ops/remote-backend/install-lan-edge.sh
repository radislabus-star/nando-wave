#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TEMPLATE="${ROOT_DIR}/ops/remote-backend/nginx-lan.conf.template"
UNIT_SOURCE="${ROOT_DIR}/ops/remote-backend/nando-transport-gateway.service"
NGINX_DIR="${NANDO_REMOTE_NGINX_DIR:-/etc/nando-gateway}"
SYSTEMD_DIR="${NANDO_REMOTE_SYSTEMD_DIR:-/etc/systemd/system}"
INSTALL_ONLY="${NANDO_REMOTE_INSTALL_ONLY:-0}"
LAN_BIND="${NANDO_REMOTE_LAN_BIND:-}"
LAN_ALLOW="${NANDO_REMOTE_LAN_ALLOW:-}"
DNS_RESOLVERS="${NANDO_REMOTE_DNS_RESOLVERS:-}"

usage() {
  cat <<'EOF'
Install the private-LAN Nando Nginx edge.

Usage:
  ops/remote-backend/install-lan-edge.sh \
    --bind 192.168.3.94:8787 \
    --allow 192.168.3.0/24

Environment:
  NANDO_REMOTE_INSTALL_ONLY=1   install and validate without starting
  NANDO_REMOTE_DNS_RESOLVERS    space-separated IPv4 resolvers
  NANDO_REMOTE_NGINX_DIR        default: /etc/nando-gateway
  NANDO_REMOTE_SYSTEMD_DIR      default: /etc/systemd/system
EOF
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
  default_dns_link=""
  if command -v resolvectl >/dev/null 2>&1; then
    default_dns_link="$(
      resolvectl domain 2>/dev/null \
        | sed -n 's/^Link [0-9][0-9]* (\([^)]*\)):.*~\..*$/\1/p' \
        | head -n 1
    )"
  fi
  if [[ -n "${default_dns_link}" ]]; then
    DNS_RESOLVERS="$(
      resolvectl dns "${default_dns_link}" 2>/dev/null \
        | cut -d: -f2- \
        | xargs
    )"
  fi
  if [[ -z "${DNS_RESOLVERS}" ]]; then
    resolver_file="/run/systemd/resolve/resolv.conf"
    if [[ ! -r "${resolver_file}" ]]; then
      resolver_file="/etc/resolv.conf"
    fi
    DNS_RESOLVERS="$(
      awk '/^nameserver[[:space:]]+/ && !seen[$2]++ {print $2}' "${resolver_file}" \
        | paste -sd ' ' -
    )"
  fi
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
if ! command -v nginx >/dev/null 2>&1; then
  echo "nginx is not installed" >&2
  exit 2
fi

rendered="$(mktemp)"
trap 'rm -f "${rendered}"' EXIT
sed \
  -e "s#@NANDO_LAN_BIND@#${LAN_BIND}#g" \
  -e "s#@NANDO_LAN_CIDR@#${LAN_ALLOW}#g" \
  -e "s#@NANDO_DNS_RESOLVERS@#${DNS_RESOLVERS}#g" \
  "${TEMPLATE}" > "${rendered}"

sudo -n install -d -o root -g root -m 0755 "${NGINX_DIR}" "${SYSTEMD_DIR}"
sudo -n install -d -o www-data -g www-data -m 0750 \
  /run/nando-gateway \
  /var/lib/nando-gateway \
  /var/lib/nando-gateway/client-body \
  /var/lib/nando-gateway/proxy
sudo -n touch /var/lib/nando-gateway/economics-access.jsonl
sudo -n chown www-data:www-data /var/lib/nando-gateway/economics-access.jsonl
sudo -n chmod 0640 /var/lib/nando-gateway/economics-access.jsonl
sudo -n install -m 0644 "${rendered}" "${NGINX_DIR}/nginx.conf"
sudo -n install -m 0644 \
  "${UNIT_SOURCE}" \
  "${SYSTEMD_DIR}/nando-transport-gateway.service"
sudo -n -u www-data nginx -t -c "${NGINX_DIR}/nginx.conf"
sudo -n rm -f /run/nando-gateway/nginx.pid
sudo -n chown www-data:www-data /run/nando-gateway
sudo -n systemctl disable --now nginx.service >/dev/null 2>&1 || true
sudo -n systemctl daemon-reload

if [[ "${INSTALL_ONLY}" == "1" ]]; then
  echo "LAN edge installed and validated; service start skipped"
  echo "DNS resolvers: ${DNS_RESOLVERS}"
  exit 0
fi

sudo -n systemctl enable --now nando-transport-gateway.service
curl -fsS --max-time 2 "http://${LAN_BIND}/health" >/dev/null
echo "LAN edge ready: http://${LAN_BIND}"
