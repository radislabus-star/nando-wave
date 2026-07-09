#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-/etc/nando-wave/phase-center.env}"
if [[ -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "${ENV_FILE}"
  set +a
fi

OPS_DIR="${NANDO_PHASE_CENTER_OPS_DIR:-/opt/nando-wave/ops/phase-center-test-server}"
BRIDGE_PY="${OPS_DIR}/bin/nando-provider-bridge.py"
ONBOARD_SCRIPT="${OPS_DIR}/bin/nando-phase-center-upstream-onboard.sh"
READINESS_SCRIPT="${OPS_DIR}/bin/nando-provider-bridge-upstream-readiness.sh"
OUT_JSON="${NANDO_PROVIDER_BRIDGE_UPSTREAM_ONBOARD_SMOKE_REPORT:-${NANDO_METRICS_DIR:-/var/lib/nando-wave/streaming/metrics}/nando-phase-center.provider-bridge-upstream-onboard-smoke.json}"

for script in "${BRIDGE_PY}" "${ONBOARD_SCRIPT}" "${READINESS_SCRIPT}"; do
  if [[ ! -x "${script}" ]]; then
    echo "required script missing or not executable: ${script}" >&2
    exit 1
  fi
done

mkdir -p "$(dirname "${OUT_JSON}")"
tmp_dir="$(mktemp -d)"
trap 'kill ${bridge_pid:-0} ${upstream_pid:-0} >/dev/null 2>&1 || true; rm -rf "${tmp_dir}"' EXIT

tmp_env="${tmp_dir}/phase-center.env"
cp "${ENV_FILE}" "${tmp_env}"
chmod 0600 "${tmp_env}"

before_sha="$(sha256sum "${ENV_FILE}" | awk '{print $1}')"

set_kv() {
  local key="$1"
  local value="$2"
  if grep -qE "^${key}=" "${tmp_env}"; then
    sed -i -E "s#^${key}=.*#${key}=${value}#" "${tmp_env}"
  else
    printf '%s=%s\n' "${key}" "${value}" >> "${tmp_env}"
  fi
}

read -r bridge_port upstream_port < <(python3 - <<'PY'
import socket

ports = []
for _ in range(2):
    sock = socket.socket()
    sock.bind(("127.0.0.1", 0))
    ports.append(sock.getsockname()[1])
    sock.close()
print(*ports)
PY
)

cat > "${tmp_dir}/upstream.py" <<'PY'
#!/usr/bin/env python3
import json
import sys
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

port = int(sys.argv[1])
hits_path = sys.argv[2]


class Handler(BaseHTTPRequestHandler):
    def log_message(self, _format, *_args):
        return

    def _send(self, payload, status=200):
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("x-request-id", "req_nando_upstream_onboard_smoke")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        self._send({"ok": True, "service": "nando-upstream-onboard-smoke"})

    def do_POST(self):
        length = int(self.headers.get("content-length") or 0)
        body = self.rfile.read(length)
        with open(hits_path, "a", encoding="utf-8") as handle:
            handle.write(json.dumps({"path": self.path, "bytes": len(body)}) + "\n")
        if self.path.endswith("/responses"):
            self._send({
                "id": "resp_upstream_onboard_smoke",
                "object": "response",
                "created_at": int(time.time()),
                "status": "completed",
                "model": "upstream-onboard-smoke",
                "output_text": "UPSTREAM_ONBOARD_OK",
                "usage": {"input_tokens": 7, "output_tokens": 4, "total_tokens": 11},
                "output": [{
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "UPSTREAM_ONBOARD_OK"}],
                }],
            })
            return
        self._send({
            "id": "chatcmpl-upstream-onboard-smoke",
            "object": "chat.completion",
            "created": int(time.time()),
            "model": "upstream-onboard-smoke",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "UPSTREAM_ONBOARD_OK"},
                "finish_reason": "stop",
            }],
            "usage": {"prompt_tokens": 7, "completion_tokens": 4, "total_tokens": 11},
        })


ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
PY
chmod 0755 "${tmp_dir}/upstream.py"

set_kv "NANDO_PROVIDER_BRIDGE_BIND" "127.0.0.1:${bridge_port}"
set_kv "NANDO_PROVIDER_BRIDGE_EVENTS_JSONL" "${tmp_dir}/bridge-events.jsonl"
set_kv "NANDO_PROVIDER_BRIDGE_DECISIONS_JSONL" "${tmp_dir}/bridge-decisions.jsonl"
set_kv "NANDO_PROVIDER_BRIDGE_BOUNDARY_EVENTS_JSONL" "${tmp_dir}/bridge-boundary.jsonl"
set_kv "NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_REPORT" "${tmp_dir}/upstream-readiness.json"
set_kv "NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_ALLOW_REAL_CALL" "0"
set_kv "NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_PROMPT" "ordinary_broad_prompt"
set_kv "NANDO_LOCAL_ACCEPT_ENABLED" "1"
set_kv "NANDO_CLIENT_ALLOW_LOCAL_ACCEPT" "1"
set_kv "NANDO_CLIENT_REQUIRE_VERIFIER" "1"
set_kv "NANDO_CLIENT_REQUIRE_FALSE_ACCEPTS_ZERO" "1"
set_kv "NANDO_CLIENT_KILL_SWITCH" "0"
set_kv "NANDO_GATEWAY_LOCAL_CMD" "${NANDO_GATEWAY_LOCAL_CMD:-${OPS_DIR}/bin/nando-llm-local-executor.sh}"

python3 "${tmp_dir}/upstream.py" "${upstream_port}" "${tmp_dir}/upstream-hits.jsonl" >"${tmp_dir}/upstream.log" 2>&1 &
upstream_pid=$!

wait_http() {
  local url="$1"
  for _ in $(seq 1 50); do
    if curl -fsS "${url}" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.05
  done
  return 1
}

wait_http "http://127.0.0.1:${upstream_port}/health"

printf '%s\n' "fake-onboard-smoke-key" | "${ONBOARD_SCRIPT}" "${tmp_env}" \
  --base-url "http://127.0.0.1:${upstream_port}" \
  --provider "fake_onboard_smoke" \
  --api-key-stdin \
  --allow-real-probe \
  --configure-only > "${tmp_dir}/onboard.json"

env -i PATH="${PATH}" NANDO_PHASE_CENTER_ENV="${tmp_env}" python3 "${BRIDGE_PY}" >"${tmp_dir}/bridge.log" 2>&1 &
bridge_pid=$!
if ! wait_http "http://127.0.0.1:${bridge_port}/health"; then
  echo "temporary bridge did not become healthy" >&2
  sed -n '1,120p' "${tmp_dir}/bridge.log" >&2 || true
  exit 1
fi

"${READINESS_SCRIPT}" "${tmp_env}" >/dev/null

health_body="${tmp_dir}/health.json"
local_body="${tmp_dir}/local.json"
broad_body="${tmp_dir}/broad.json"

curl -fsS "http://127.0.0.1:${bridge_port}/health" > "${health_body}"
curl -fsS \
  -H 'content-type: application/json' \
  --data '{"model":"nando-test","messages":[{"role":"user","content":"nando compression"}]}' \
  "http://127.0.0.1:${bridge_port}/v1/chat/completions" > "${local_body}"
curl -fsS \
  -H 'content-type: application/json' \
  --data '{"model":"nando-test","messages":[{"role":"user","content":"ordinary broad prompt"}]}' \
  "http://127.0.0.1:${bridge_port}/v1/chat/completions" > "${broad_body}"

after_sha="$(sha256sum "${ENV_FILE}" | awk '{print $1}')"
real_env_unchanged=false
if [[ "${before_sha}" == "${after_sha}" ]]; then
  real_env_unchanged=true
fi

upstream_hit_count=0
if [[ -s "${tmp_dir}/upstream-hits.jsonl" ]]; then
  upstream_hit_count="$(wc -l < "${tmp_dir}/upstream-hits.jsonl" | tr -d ' ')"
fi
boundary_event_count=0
boundary_total_tokens=0
if [[ -s "${tmp_dir}/bridge-boundary.jsonl" ]]; then
  boundary_event_count="$(wc -l < "${tmp_dir}/bridge-boundary.jsonl" | tr -d ' ')"
  boundary_total_tokens="$(jq -s '[.[] | .provider_total_tokens // 0] | add // 0' "${tmp_dir}/bridge-boundary.jsonl")"
fi

tmp_report="${tmp_dir}/report.json"
jq -n \
  --arg env_file "${ENV_FILE}" \
  --arg bridge_url "http://127.0.0.1:${bridge_port}" \
  --arg upstream_url "http://127.0.0.1:${upstream_port}" \
  --slurpfile onboard "${tmp_dir}/onboard.json" \
  --slurpfile readiness "${tmp_dir}/upstream-readiness.json" \
  --slurpfile health "${health_body}" \
  --slurpfile local "${local_body}" \
  --slurpfile broad "${broad_body}" \
  --argjson real_env_unchanged "${real_env_unchanged}" \
  --argjson upstream_hit_count "${upstream_hit_count}" \
  --argjson boundary_event_count "${boundary_event_count}" \
  --argjson boundary_total_tokens "${boundary_total_tokens}" \
  '{
    report_kind: "nando_phase_center_upstream_onboard_smoke_v1",
    env_file: $env_file,
    bridge_url: $bridge_url,
    upstream_url: $upstream_url,
    real_env_unchanged: $real_env_unchanged,
    onboard: ($onboard[0] // {}),
    readiness: ($readiness[0] // {}),
    health: ($health[0] // {}),
    local: ($local[0] // {}),
    broad: ($broad[0] // {}),
    upstream_hit_count: $upstream_hit_count,
    provider_boundary_event_count: $boundary_event_count,
    provider_boundary_total_tokens: $boundary_total_tokens,
    api_key_value_printed: false,
    provider_secret_printed: false,
    market_money_claim_allowed: false,
    boundary: "temporary onboarding smoke only: proves configure-only upstream onboarding, temporary bridge broad proxy, and provider-boundary capture without mutating real server policy or unlocking money claims"
  }
  | .pass = (
      .real_env_unchanged
      and (.onboard.configure_only == true)
      and (.onboard.upstream_configured == true)
      and (.onboard.api_key_present == true)
      and (.onboard.api_key_value_printed == false)
      and (.health.upstream_configured == true)
      and (.local.nando.local_accept == true)
      and (.broad.model == "upstream-onboard-smoke")
      and (.readiness.ready_for_broad_provider_traffic == true)
      and (.upstream_hit_count >= 2)
      and (.provider_boundary_event_count >= 2)
      and (.provider_boundary_total_tokens >= 22)
    )
  | .verdict = (if .pass then "NANDO_PHASE_CENTER_UPSTREAM_ONBOARD_SMOKE_PASS" else "NANDO_PHASE_CENTER_UPSTREAM_ONBOARD_SMOKE_FAIL" end)' > "${tmp_report}"

mv "${tmp_report}" "${OUT_JSON}"
jq -e '.pass == true' "${OUT_JSON}" >/dev/null
echo "${OUT_JSON}"
