#!/usr/bin/env python3
"""Fail-open OpenAI-compatible provider bridge for Nando canary routes.

This is an ops adapter, not model/runtime authority. It accepts HTTP requests,
tries the configured verifier-bound local executor only when server policy
allows it, and otherwise proxies the original request to the upstream provider.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any


def load_env(path: str) -> None:
    env_path = Path(path)
    if not env_path.exists():
        return
    for raw_line in env_path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        os.environ.setdefault(key, value)


ENV_FILE = os.environ.get("NANDO_PHASE_CENTER_ENV", "/etc/nando-wave/phase-center.env")
load_env(ENV_FILE)

from nando_status_dashboard import (  # noqa: E402
    dashboard_enabled,
    dashboard_key_for_path,
    dashboard_path_authorized,
    status_dashboard_html,
)

BRIDGE_BIND = os.environ.get("NANDO_PROVIDER_BRIDGE_BIND", "127.0.0.1:8787")
UPSTREAM_BASE_URL = os.environ.get("NANDO_PROVIDER_UPSTREAM_BASE_URL", "").rstrip("/")
UPSTREAM_API_KEY = os.environ.get("NANDO_PROVIDER_UPSTREAM_API_KEY", "")
LOCAL_CMD = os.environ.get(
    "NANDO_GATEWAY_LOCAL_CMD",
    "/opt/nando-wave/ops/phase-center-test-server/bin/nando-llm-local-executor.sh",
)
EVENTS_JSONL = Path(
    os.environ.get(
        "NANDO_PROVIDER_BRIDGE_EVENTS_JSONL",
        "/var/lib/nando-wave/streaming/nando-provider-bridge.events.jsonl",
    )
)
DECISIONS_JSONL = Path(
    os.environ.get(
        "NANDO_PROVIDER_BRIDGE_DECISIONS_JSONL",
        "/var/lib/nando-wave/streaming/nando-provider-bridge.decisions.jsonl",
    )
)
PROVIDER_BOUNDARY_JSONL = Path(
    os.environ.get(
        "NANDO_PROVIDER_BRIDGE_BOUNDARY_EVENTS_JSONL",
        "/var/lib/nando-wave/streaming/nando-provider-bridge.provider-boundary-events.jsonl",
    )
)
PROVIDER_NAME = os.environ.get("NANDO_PROVIDER_NAME", "openai_compatible_upstream")
TIMEOUT_MS = int(os.environ.get("NANDO_PROVIDER_BRIDGE_LOCAL_TIMEOUT_MS", "200"))
MAX_BODY_BYTES = int(os.environ.get("NANDO_PROVIDER_BRIDGE_MAX_BODY_BYTES", "1048576"))
DEFAULT_UPSTREAM_API_PREFIX = os.environ.get("NANDO_PROVIDER_UPSTREAM_API_PREFIX", "/v1")


def now_iso() -> str:
    return datetime.now(timezone.utc).astimezone().isoformat()


def token_estimate(text: str) -> int:
    return max(1, (len(text.encode("utf-8")) + 3) // 4)


def write_jsonl(path: Path, row: dict[str, Any]) -> None:
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")
    except OSError:
        pass


def env_bool(name: str, default: str = "0") -> bool:
    return os.environ.get(name, default) == "1"


def extract_request_text(payload: Any) -> str:
    if isinstance(payload, dict):
        if isinstance(payload.get("input"), str):
            return payload["input"]
        if isinstance(payload.get("prompt"), str):
            return payload["prompt"]
        messages = payload.get("messages")
        if isinstance(messages, list):
            parts: list[str] = []
            for message in messages:
                if not isinstance(message, dict):
                    continue
                content = message.get("content")
                if isinstance(content, str):
                    parts.append(content)
                elif isinstance(content, list):
                    for item in content:
                        if isinstance(item, dict):
                            text = item.get("text")
                            if isinstance(text, str):
                                parts.append(text)
            return "\n".join(parts)
    return ""


def traffic_source_from_payload(payload: Any) -> str:
    if not isinstance(payload, dict):
        return "unspecified"
    metadata = payload.get("metadata")
    if isinstance(metadata, dict):
        value = metadata.get("nando_traffic_source") or metadata.get("traffic_source")
        if isinstance(value, str) and value:
            return value[:80]
    value = payload.get("nando_traffic_source") or payload.get("traffic_source")
    if isinstance(value, str) and value:
        return value[:80]
    return "unspecified"


def int_path(payload: Any, *paths: tuple[str, ...]) -> int:
    for path in paths:
        current = payload
        for key in path:
            if not isinstance(current, dict) or key not in current:
                current = None
                break
            current = current[key]
        if isinstance(current, int):
            return current
        if isinstance(current, float) and current.is_integer():
            return int(current)
    return 0


def str_path(payload: Any, *paths: tuple[str, ...]) -> str:
    for path in paths:
        current = payload
        for key in path:
            if not isinstance(current, dict) or key not in current:
                current = None
                break
            current = current[key]
        if isinstance(current, str) and current:
            return current
    return ""


def parse_json_body(body: bytes) -> dict[str, Any]:
    try:
        value = json.loads(body)
    except json.JSONDecodeError:
        return {}
    return value if isinstance(value, dict) else {}


def usage_from_payload(payload: dict[str, Any]) -> dict[str, int]:
    usage = payload.get("usage")
    if not isinstance(usage, dict):
        usage = {}
    input_tokens = int_path(
        usage,
        ("input_tokens",),
        ("prompt_tokens",),
        ("prompt_tokens_details", "uncached_tokens"),
    )
    cached_input_tokens = int_path(
        usage,
        ("cached_input_tokens",),
        ("prompt_tokens_details", "cached_tokens"),
    )
    output_tokens = int_path(usage, ("output_tokens",), ("completion_tokens",))
    total_tokens = int_path(usage, ("total_tokens",))
    if total_tokens == 0:
        total_tokens = input_tokens + cached_input_tokens + output_tokens
    return {
        "input_tokens": input_tokens,
        "cached_input_tokens": cached_input_tokens,
        "output_tokens": output_tokens,
        "total_tokens": total_tokens,
    }


def header_value(headers: Any, name: str) -> str:
    try:
        value = headers.get(name)
    except AttributeError:
        return ""
    return value or ""


def api_version_for_path(path: str) -> str:
    if path == "/v2" or path.startswith("/v2/"):
        return "v2"
    return "v1"


def endpoint_for_path(path: str) -> str:
    if path.endswith("/chat/completions"):
        return "chat.completions"
    if path.endswith("/responses"):
        return "responses"
    return "unknown"


def upstream_path_for_bridge_path(path: str) -> str:
    if path.startswith("/v2/"):
        return f"{DEFAULT_UPSTREAM_API_PREFIX.rstrip('/')}{path[3:]}"
    return path


def nanda_cpu_boundary(api_version: str) -> str:
    if api_version == "v2":
        return (
            "NANDA CPU v2 compact latent transition runtime: "
            "surface_event -> hidden_state z_t -> transition_center C(a) -> verifier"
        )
    return "verifier-bound local route response"


def write_provider_boundary_event(
    *,
    request_hash: str,
    path: str,
    status_code: int,
    headers: Any,
    upstream_body: bytes,
) -> None:
    payload = parse_json_body(upstream_body)
    usage = usage_from_payload(payload)
    provider_request_id = (
        header_value(headers, "x-request-id")
        or header_value(headers, "request-id")
        or header_value(headers, "x-openai-request-id")
    )
    provider_response_id = str_path(payload, ("id",), ("response", "id"))
    provider_trace_id = header_value(headers, "x-trace-id") or header_value(headers, "trace-id")
    provider_keys = []
    if provider_request_id:
        provider_keys.append(f"provider_request_id:{provider_request_id}")
    if provider_response_id:
        provider_keys.append(f"provider_response_id:{provider_response_id}")
    if provider_trace_id:
        provider_keys.append(f"provider_trace_id:{provider_trace_id}")
    if provider_request_id:
        provider_keys.append(f"openai_request_id:{provider_request_id}")
    provider_keys = sorted(set(provider_keys))
    if not provider_keys:
        provider_keys = [f"custom_id:bridge_request_sha256_{request_hash}"]

    row = {
        "schema_version": "nando_provider_bridge_provider_boundary_event_v1",
        "timestamp": now_iso(),
        "provider": PROVIDER_NAME,
        "billing_source": "nando_provider_bridge_observed_upstream_response",
        "request_fingerprint": request_hash,
        "match_keys": [f"request_fingerprint:{request_hash}"],
        "external_provider_correlation_keys": provider_keys,
        "provider_request_id": provider_request_id or None,
        "provider_response_id": provider_response_id or None,
        "provider_trace_id": provider_trace_id or None,
        "openai_request_id": provider_request_id or None,
        "model_id": str_path(payload, ("model",)),
        "path": path,
        "status_code": status_code,
        "provider_total_tokens": usage["total_tokens"],
        "input_tokens": usage["input_tokens"],
        "cached_input_tokens": usage["cached_input_tokens"],
        "output_tokens": usage["output_tokens"],
        "provider_cost_microusd": 0,
        "token_cost": {
            "total_tokens": usage["total_tokens"],
            "input_tokens": usage["input_tokens"],
            "cached_input_tokens": usage["cached_input_tokens"],
            "output_tokens": usage["output_tokens"],
            "total_cost_microusd": 0,
            "token_evidence_missing": usage["total_tokens"] == 0,
            "cost_evidence_missing": True,
        },
        "local_accept_enabled": False,
        "market_money_claim_allowed": False,
        "boundary": "observed upstream provider boundary from HTTP bridge; metadata only, no scoring, no local_accept, no money claim without external cost evidence",
    }
    write_jsonl(PROVIDER_BOUNDARY_JSONL, row)


def local_policy_allows() -> bool:
    return (
        env_bool("NANDO_OFFLOAD", "1")
        and env_bool("NANDO_LOCAL_ACCEPT_ENABLED")
        and env_bool("NANDO_CLIENT_ALLOW_LOCAL_ACCEPT")
        and not env_bool("NANDO_CLIENT_KILL_SWITCH")
    )


def try_local_executor(request_text: str) -> tuple[bool, dict[str, Any], str]:
    if not local_policy_allows():
        return False, {}, "local_policy_disabled"
    started = time.monotonic_ns()
    try:
        completed = subprocess.run(
            ["bash", "-c", LOCAL_CMD],
            input=request_text,
            text=True,
            capture_output=True,
            timeout=max(0.001, TIMEOUT_MS / 1000),
            check=False,
        )
    except subprocess.TimeoutExpired:
        return False, {}, "local_cmd_timeout"
    elapsed_ns = time.monotonic_ns() - started
    if completed.returncode != 0:
        return False, {"elapsed_ns": elapsed_ns}, "local_cmd_error"
    try:
        local = json.loads(completed.stdout)
    except json.JSONDecodeError:
        return False, {"elapsed_ns": elapsed_ns}, "local_cmd_invalid_json"

    verifier_ok = bool(local.get("verifier_ok") or local.get("verified_safe_accept"))
    false_accepts = int(local.get("false_accepts") or 0)
    response = str(local.get("response") or local.get("output") or local.get("output_text") or "")
    local_accept = bool(local.get("local_accept"))
    if env_bool("NANDO_CLIENT_REQUIRE_VERIFIER", "1") and not verifier_ok:
        return False, {**local, "elapsed_ns": elapsed_ns}, "verifier_required_not_ok"
    if env_bool("NANDO_CLIENT_REQUIRE_FALSE_ACCEPTS_ZERO", "1") and false_accepts != 0:
        return False, {**local, "elapsed_ns": elapsed_ns}, "false_accepts_nonzero"
    if not (local_accept and verifier_ok and response):
        return False, {**local, "elapsed_ns": elapsed_ns}, "local_declined_or_empty"
    return True, {**local, "elapsed_ns": elapsed_ns}, "verifier_bound_local_accept"


def chat_completion_response(
    response_text: str,
    request_text: str,
    route: str,
    api_version: str,
) -> dict[str, Any]:
    now = int(time.time())
    completion_tokens = token_estimate(response_text)
    prompt_tokens = token_estimate(request_text)
    return {
        "id": "chatcmpl-nando-local",
        "object": "chat.completion",
        "created": now,
        "model": "nando-local",
        "choices": [
            {
                "index": 0,
                "message": {"role": "assistant", "content": response_text},
                "finish_reason": "stop",
            }
        ],
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": prompt_tokens + completion_tokens,
        },
        "nando": {
            "api_version": api_version,
            "local_accept": True,
            "route": route,
            "false_accepts": 0,
            "architecture": "compact_latent_transition_runtime" if api_version == "v2" else "v1_canary_route",
            "transition_runtime": api_version == "v2",
            "boundary": nanda_cpu_boundary(api_version),
        },
    }


def responses_response(
    response_text: str,
    request_text: str,
    route: str,
    api_version: str,
) -> dict[str, Any]:
    now = int(time.time())
    output_tokens = token_estimate(response_text)
    input_tokens = token_estimate(request_text)
    return {
        "id": "resp_nando_local",
        "object": "response",
        "created_at": now,
        "status": "completed",
        "model": "nando-local",
        "output": [
            {
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": response_text}],
            }
        ],
        "output_text": response_text,
        "usage": {
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "total_tokens": input_tokens + output_tokens,
        },
        "nando": {
            "api_version": api_version,
            "local_accept": True,
            "route": route,
            "false_accepts": 0,
            "architecture": "compact_latent_transition_runtime" if api_version == "v2" else "v1_canary_route",
            "transition_runtime": api_version == "v2",
            "boundary": nanda_cpu_boundary(api_version),
        },
    }


class BridgeHandler(BaseHTTPRequestHandler):
    server_version = "nando-provider-bridge/0.1"

    def log_message(self, _format: str, *_args: Any) -> None:
        return

    def send_json(self, status: int, payload: dict[str, Any]) -> None:
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("content-type", "application/json; charset=utf-8")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def send_html(self, status: int, payload: str) -> None:
        body = payload.encode("utf-8")
        self.send_response(status)
        self.send_header("content-type", "text/html; charset=utf-8")
        self.send_header("cache-control", "no-store")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:
        if self.path in {"/health", "/v1/health", "/v2/health"}:
            self.send_json(
                200,
                {
                    "ok": True,
                    "status": "ok",
                    "service": "nando-provider-bridge",
                    "local_accept_enabled": env_bool("NANDO_LOCAL_ACCEPT_ENABLED"),
                    "client_allow_local_accept": env_bool("NANDO_CLIENT_ALLOW_LOCAL_ACCEPT"),
                    "safety_policy": os.environ.get("NANDO_CLIENT_SAFETY_POLICY", ""),
                    "upstream_configured": bool(UPSTREAM_BASE_URL),
                    "default_client_api_version": "v2",
                    "supported_api_versions": ["v1", "v2"],
                    "v2_architecture": "compact_latent_transition_runtime",
                    "status_dashboard_enabled": dashboard_enabled(),
                },
            )
            return
        if dashboard_key_for_path(self.path):
            if dashboard_path_authorized(self.path):
                self.send_html(200, status_dashboard_html(self.path))
            else:
                self.send_json(404, {"error": {"message": "not found", "type": "not_found"}})
            return
        self.send_json(404, {"error": {"message": "not found", "type": "not_found"}})

    def do_POST(self) -> None:
        content_length = int(self.headers.get("content-length") or 0)
        if content_length <= 0 or content_length > MAX_BODY_BYTES:
            self.send_json(413, {"error": {"message": "invalid body size", "type": "bad_request"}})
            return
        body = self.rfile.read(content_length)
        request_hash = hashlib.sha256(body).hexdigest()
        started_ns = time.monotonic_ns()
        api_version = api_version_for_path(self.path)
        endpoint = endpoint_for_path(self.path)
        write_jsonl(
            EVENTS_JSONL,
            {
                "schema_version": "nando_provider_bridge_event_v1",
                "timestamp": now_iso(),
                "stage": "ingress",
                "path": self.path,
                "api_version": api_version,
                "endpoint": endpoint,
                "request_sha256": request_hash,
                "request_bytes": len(body),
                "local_accept_enabled": env_bool("NANDO_LOCAL_ACCEPT_ENABLED"),
                "client_allow_local_accept": env_bool("NANDO_CLIENT_ALLOW_LOCAL_ACCEPT"),
                "safety_policy": os.environ.get("NANDO_CLIENT_SAFETY_POLICY", ""),
            },
        )
        try:
            payload = json.loads(body)
        except json.JSONDecodeError:
            self.send_json(400, {"error": {"message": "invalid JSON", "type": "bad_request"}})
            return

        request_text = extract_request_text(payload)
        traffic_source = traffic_source_from_payload(payload)
        local_ok, local, reason = try_local_executor(request_text)
        if local_ok:
            response_text = str(local.get("response") or local.get("output") or local.get("output_text"))
            route = str(local.get("route") or "unknown")
            if endpoint == "responses":
                response_payload = responses_response(response_text, request_text, route, api_version)
            else:
                response_payload = chat_completion_response(response_text, request_text, route, api_version)
            elapsed_ns = time.monotonic_ns() - started_ns
            write_jsonl(
                DECISIONS_JSONL,
                {
                    "schema_version": "nando_provider_bridge_decision_v1",
                    "timestamp": now_iso(),
                    "request_sha256": request_hash,
                    "api_version": api_version,
                    "endpoint": endpoint,
                    "traffic_source": traffic_source,
                    "decision": "local_accept",
                    "reason": reason,
                    "local_route": route,
                    "elapsed_ns": elapsed_ns,
                    "tokens_saved_estimated": token_estimate(request_text),
                    "false_accepts": 0,
                    "architecture": "compact_latent_transition_runtime" if api_version == "v2" else "v1_canary_route",
                },
            )
            self.send_json(200, response_payload)
            return

        self.proxy_upstream(body, request_hash, reason, started_ns, api_version, endpoint, traffic_source)

    def proxy_upstream(
        self,
        body: bytes,
        request_hash: str,
        reason: str,
        started_ns: int,
        api_version: str,
        endpoint: str,
        traffic_source: str,
    ) -> None:
        if not UPSTREAM_BASE_URL:
            elapsed_ns = time.monotonic_ns() - started_ns
            write_jsonl(
                EVENTS_JSONL,
                {
                    "schema_version": "nando_provider_bridge_event_v1",
                    "timestamp": now_iso(),
                    "stage": "egress",
                    "api_version": api_version,
                    "endpoint": endpoint,
                    "traffic_source": traffic_source,
                    "decision": "fallback_blocked",
                    "reason": "upstream_missing",
                    "local_decline_reason": reason,
                    "request_sha256": request_hash,
                    "elapsed_ns": elapsed_ns,
                },
            )
            self.send_json(
                502,
                {
                    "error": {
                        "message": "Nando local route declined and upstream is not configured",
                        "type": "upstream_missing",
                    },
                    "nando": {
                        "api_version": api_version,
                        "local_accept": False,
                        "fallback_reason": reason,
                        "architecture": "compact_latent_transition_runtime" if api_version == "v2" else "v1_canary_route",
                    },
                },
            )
            return

        upstream_path = upstream_path_for_bridge_path(self.path)
        url = f"{UPSTREAM_BASE_URL}{upstream_path}"
        headers = {"content-type": self.headers.get("content-type", "application/json")}
        auth = self.headers.get("authorization")
        if auth:
            headers["authorization"] = auth
        elif UPSTREAM_API_KEY:
            headers["authorization"] = f"Bearer {UPSTREAM_API_KEY}"
        request = urllib.request.Request(url, data=body, headers=headers, method="POST")
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                upstream_body = response.read()
                write_provider_boundary_event(
                    request_hash=request_hash,
                    path=upstream_path,
                    status_code=response.status,
                    headers=response.headers,
                    upstream_body=upstream_body,
                )
                self.send_response(response.status)
                self.send_header("content-type", response.headers.get("content-type", "application/json"))
                self.send_header("content-length", str(len(upstream_body)))
                self.end_headers()
                self.wfile.write(upstream_body)
        except urllib.error.HTTPError as error:
            upstream_body = error.read()
            write_provider_boundary_event(
                request_hash=request_hash,
                path=upstream_path,
                status_code=error.code,
                headers=error.headers,
                upstream_body=upstream_body,
            )
            self.send_response(error.code)
            self.send_header("content-type", error.headers.get("content-type", "application/json"))
            self.send_header("content-length", str(len(upstream_body)))
            self.end_headers()
            self.wfile.write(upstream_body)
        except OSError as error:
            self.send_json(
                502,
                {"error": {"message": f"upstream proxy failed: {error}", "type": "upstream_error"}},
            )
        finally:
            elapsed_ns = time.monotonic_ns() - started_ns
            write_jsonl(
                EVENTS_JSONL,
                {
                    "schema_version": "nando_provider_bridge_event_v1",
                    "timestamp": now_iso(),
                    "stage": "egress",
                    "api_version": api_version,
                    "endpoint": endpoint,
                    "traffic_source": traffic_source,
                    "decision": "upstream_fallback",
                    "reason": reason,
                    "request_sha256": request_hash,
                    "elapsed_ns": elapsed_ns,
                    "upstream_configured": bool(UPSTREAM_BASE_URL),
                    "upstream_path": upstream_path,
                },
            )


def parse_bind(bind: str) -> tuple[str, int]:
    if ":" not in bind:
        raise SystemExit(f"invalid NANDO_PROVIDER_BRIDGE_BIND: {bind}")
    host, port = bind.rsplit(":", 1)
    return host, int(port)


def main() -> int:
    host, port = parse_bind(BRIDGE_BIND)
    server = ThreadingHTTPServer((host, port), BridgeHandler)
    print(f"nando-provider-bridge listening on {host}:{port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
