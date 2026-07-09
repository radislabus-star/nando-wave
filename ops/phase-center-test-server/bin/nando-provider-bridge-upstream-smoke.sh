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
OUT_JSON="${NANDO_PROVIDER_BRIDGE_UPSTREAM_SMOKE_REPORT:-${NANDO_METRICS_DIR:-/var/lib/nando-wave/streaming/metrics}/nando-phase-center.provider-bridge-upstream-smoke.json}"

mkdir -p "$(dirname "${OUT_JSON}")"
tmp_dir="$(mktemp -d)"
case_rows="${tmp_dir}/cases.jsonl"
upstream_hits="${tmp_dir}/upstream-hits.jsonl"
trap 'kill ${bridge_pid:-0} ${upstream_pid:-0} >/dev/null 2>&1 || true; rm -rf "${tmp_dir}"' EXIT

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
        self.send_header("x-request-id", "req_nando_upstream_smoke")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        self._send({"ok": True, "service": "nando-upstream-smoke"})

    def do_POST(self):
        length = int(self.headers.get("content-length") or 0)
        body = self.rfile.read(length)
        with open(hits_path, "a", encoding="utf-8") as handle:
            handle.write(json.dumps({"path": self.path, "bytes": len(body)}) + "\n")
        if self.path.endswith("/responses"):
            self._send({
                "id": "resp_upstream_smoke",
                "object": "response",
                "created_at": int(time.time()),
                "status": "completed",
                "model": "upstream-smoke",
                "output_text": "UPSTREAM_OK",
                "usage": {"input_tokens": 7, "output_tokens": 3, "total_tokens": 10},
                "output": [{
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "UPSTREAM_OK"}],
                }],
            })
            return
        self._send({
            "id": "chatcmpl-upstream-smoke",
            "object": "chat.completion",
            "created": int(time.time()),
            "model": "upstream-smoke",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "UPSTREAM_OK"},
                "finish_reason": "stop",
            }],
            "usage": {"prompt_tokens": 7, "completion_tokens": 3, "total_tokens": 10},
        })


ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
PY
chmod 0755 "${tmp_dir}/upstream.py"

python3 "${tmp_dir}/upstream.py" "${upstream_port}" "${upstream_hits}" >"${tmp_dir}/upstream.log" 2>&1 &
upstream_pid=$!

env \
  NANDO_PHASE_CENTER_ENV="${ENV_FILE}" \
  NANDO_PROVIDER_BRIDGE_BIND="127.0.0.1:${bridge_port}" \
  NANDO_PROVIDER_UPSTREAM_BASE_URL="http://127.0.0.1:${upstream_port}" \
  NANDO_PROVIDER_UPSTREAM_API_KEY="smoke-key" \
  NANDO_PROVIDER_BRIDGE_EVENTS_JSONL="${tmp_dir}/bridge-events.jsonl" \
  NANDO_PROVIDER_BRIDGE_DECISIONS_JSONL="${tmp_dir}/bridge-decisions.jsonl" \
  NANDO_PROVIDER_BRIDGE_BOUNDARY_EVENTS_JSONL="${tmp_dir}/bridge-boundary.jsonl" \
  NANDO_LOCAL_ACCEPT_ENABLED=1 \
  NANDO_CLIENT_ALLOW_LOCAL_ACCEPT=1 \
  NANDO_CLIENT_REQUIRE_VERIFIER=1 \
  NANDO_CLIENT_REQUIRE_FALSE_ACCEPTS_ZERO=1 \
  NANDO_CLIENT_KILL_SWITCH=0 \
  NANDO_GATEWAY_LOCAL_CMD="${NANDO_GATEWAY_LOCAL_CMD:-${OPS_DIR}/bin/nando-llm-local-executor.sh}" \
  python3 "${BRIDGE_PY}" >"${tmp_dir}/bridge.log" 2>&1 &
bridge_pid=$!

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
wait_http "http://127.0.0.1:${bridge_port}/health"

curl_json() {
  local method="$1"
  local path="$2"
  local body="${3:-}"
  local out_file="$4"
  if [[ "${method}" == "GET" ]]; then
    curl -fsS "http://127.0.0.1:${bridge_port}${path}" > "${out_file}"
  else
    curl -fsS \
      -H 'content-type: application/json' \
      -X "${method}" \
      --data "${body}" \
      "http://127.0.0.1:${bridge_port}${path}" > "${out_file}"
  fi
}

record_case() {
  local name="$1"
  local body_file="$2"
  local jq_assert="$3"
  local passed=false
  if jq -e "${jq_assert}" "${body_file}" >/dev/null 2>&1; then
    passed=true
  fi
  jq -cn \
    --arg name "${name}" \
    --argjson passed "${passed}" \
    --slurpfile body "${body_file}" \
    '{name: $name, passed: $passed, body: ($body[0] // {})}' >> "${case_rows}"
}

health_body="${tmp_dir}/health.json"
local_body="${tmp_dir}/local.json"
local_v2_body="${tmp_dir}/local-v2.json"
upstream_body="${tmp_dir}/upstream.json"
upstream_v2_body="${tmp_dir}/upstream-v2.json"

curl_json GET /health "" "${health_body}"
record_case "health_upstream_configured" "${health_body}" '.ok == true and .upstream_configured == true'

curl_json POST /v1/chat/completions \
  '{"model":"nando-test","messages":[{"role":"user","content":"nando compression"}]}' \
  "${local_body}"
record_case "local_compression_not_upstream" "${local_body}" '.nando.local_accept == true and .model == "nando-local"'

curl_json POST /v2/chat/completions \
  '{"model":"nando-test","messages":[{"role":"user","content":"nando compression"}]}' \
  "${local_v2_body}"
record_case "v2_local_compression_not_upstream" "${local_v2_body}" '.nando.local_accept == true and .nando.api_version == "v2" and .nando.transition_runtime == true and .model == "nando-local"'

curl_json POST /v1/chat/completions \
  '{"model":"nando-test","messages":[{"role":"user","content":"ordinary broad prompt"}]}' \
  "${upstream_body}"
record_case "broad_prompt_reaches_upstream" "${upstream_body}" '.model == "upstream-smoke" and .choices[0].message.content == "UPSTREAM_OK"'

curl_json POST /v2/chat/completions \
  '{"model":"nando-test","messages":[{"role":"user","content":"ordinary broad prompt"}]}' \
  "${upstream_v2_body}"
record_case "v2_broad_prompt_reaches_upstream_v1" "${upstream_v2_body}" '.model == "upstream-smoke" and .choices[0].message.content == "UPSTREAM_OK"'

upstream_hit_count=0
if [[ -s "${upstream_hits}" ]]; then
  upstream_hit_count="$(wc -l < "${upstream_hits}" | tr -d ' ')"
fi
boundary_event_count=0
boundary_total_tokens=0
if [[ -s "${tmp_dir}/bridge-boundary.jsonl" ]]; then
  boundary_event_count="$(wc -l < "${tmp_dir}/bridge-boundary.jsonl" | tr -d ' ')"
  boundary_total_tokens="$(jq -s '[.[] | .provider_total_tokens // 0] | add // 0' "${tmp_dir}/bridge-boundary.jsonl")"
fi

jq -s \
  --arg env_file "${ENV_FILE}" \
  --arg bridge_url "http://127.0.0.1:${bridge_port}" \
  --arg upstream_url "http://127.0.0.1:${upstream_port}" \
  --argjson upstream_hit_count "${upstream_hit_count}" \
  --argjson boundary_event_count "${boundary_event_count}" \
  --argjson boundary_total_tokens "${boundary_total_tokens}" \
  '{
    report_kind: "nando_provider_bridge_upstream_smoke_v1",
    env_file: $env_file,
    bridge_url: $bridge_url,
    upstream_url: $upstream_url,
    cases: .,
    case_count: length,
    passed_count: ([.[] | select(.passed)] | length),
    failed_count: ([.[] | select(.passed | not)] | length),
    upstream_configured: true,
    api_key_value_printed: false,
    upstream_hit_count: $upstream_hit_count,
    provider_boundary_event_count: $boundary_event_count,
    provider_boundary_total_tokens: $boundary_total_tokens,
    verdict: (if (all(.[]; .passed) and $upstream_hit_count == 2 and $boundary_event_count == 2 and $boundary_total_tokens == 20) then
      "NANDO_PROVIDER_BRIDGE_UPSTREAM_SMOKE_PASS"
    else
      "NANDO_PROVIDER_BRIDGE_UPSTREAM_SMOKE_FAIL"
    end),
    boundary: "temporary upstream smoke only: proves v1/v2 broad traffic can fail-open to upstream while exact verifier-bound routes stay local"
  }' "${case_rows}" > "${tmp_dir}/report.json"

mv "${tmp_dir}/report.json" "${OUT_JSON}"

jq -e '.failed_count == 0 and .upstream_hit_count == 2 and .provider_boundary_event_count == 2 and .provider_boundary_total_tokens == 20' "${OUT_JSON}" >/dev/null
echo "${OUT_JSON}"
