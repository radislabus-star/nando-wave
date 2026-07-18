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
import threading
import time
import urllib.error
import urllib.request
import uuid
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
TYPED_TRANSITION_CMD = os.environ.get(
    "NANDO_TRANSITION_EXECUTOR",
    "/opt/nando-wave/bin/nando-transition-live-exec",
)
TRANSITION_TRACE_JSONL = Path(
    os.environ.get(
        "NANDO_TRANSITION_TRACE_JSONL",
        "/var/lib/nando-wave/transition/live-transitions.jsonl",
    )
)
TRANSITION_EVENTS_JSONL = Path(
    os.environ.get(
        "NANDO_TRANSITION_EXECUTION_EVENTS_JSONL",
        "/var/lib/nando-wave/transition/execution-events.jsonl",
    )
)
TRANSITION_METRICS_JSON = Path(
    os.environ.get(
        "NANDO_TRANSITION_METRICS",
        "/var/lib/nando-wave/transition/metrics.json",
    )
)
TRANSITION_ADMISSION_JSON = Path(
    os.environ.get(
        "NANDO_TRANSITION_ADMISSION_JSON",
        "/var/lib/nando-wave/transition/admission.json",
    )
)
ECONOMICS_SNAPSHOT_JSON = Path(
    os.environ.get(
        "NANDO_ECONOMICS_SNAPSHOT_JSON",
        "/var/lib/nando-wave/transition/economics.json",
    )
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
ECONOMICS_LEDGER_JSONL = Path(
    os.environ.get(
        "NANDO_ECONOMICS_LEDGER_JSONL",
        "/var/lib/nando-wave/transition/economics-terminal.jsonl",
    )
)
PROVIDER_NAME = os.environ.get("NANDO_PROVIDER_NAME", "openai_compatible_upstream")
TIMEOUT_MS = int(os.environ.get("NANDO_PROVIDER_BRIDGE_LOCAL_TIMEOUT_MS", "200"))
MAX_BODY_BYTES = int(os.environ.get("NANDO_PROVIDER_BRIDGE_MAX_BODY_BYTES", "1048576"))
DEFAULT_UPSTREAM_API_PREFIX = os.environ.get("NANDO_PROVIDER_UPSTREAM_API_PREFIX", "/v1")
UPSTREAM_TIMEOUT_S = int(os.environ.get("NANDO_PROVIDER_UPSTREAM_TIMEOUT_S", "300"))
UPSTREAM_CAPTURE_MAX_BYTES = int(
    os.environ.get("NANDO_PROVIDER_UPSTREAM_CAPTURE_MAX_BYTES", "8388608")
)
TRANSPORT_FAILURE_THRESHOLD = int(
    os.environ.get("NANDO_PROVIDER_TRANSPORT_FAILURE_THRESHOLD", "3")
)
TRANSPORT_FAILURE_WINDOW_S = int(
    os.environ.get("NANDO_PROVIDER_TRANSPORT_FAILURE_WINDOW_SECONDS", "120")
)

UPSTREAM_HEADER_ALLOWLIST = {
    "accept",
    "authorization",
    "chatgpt-account-id",
    "content-type",
    "originator",
    "user-agent",
}
UPSTREAM_HEADER_PREFIXES = ("openai-", "x-codex-", "x-openai-")

TRANSPORT_LOCK = threading.Lock()
TRANSPORT_FAILURES: list[float] = []
TRANSPORT_LAST_SUCCESS_UNIX = 0.0
TRANSPORT_LAST_ERROR = ""


class ClientDisconnected(Exception):
    pass


def now_iso() -> str:
    return datetime.now(timezone.utc).astimezone().isoformat()


def record_transport_result(success: bool, error: str = "") -> None:
    global TRANSPORT_LAST_SUCCESS_UNIX, TRANSPORT_LAST_ERROR
    current = time.time()
    with TRANSPORT_LOCK:
        TRANSPORT_FAILURES[:] = [
            timestamp
            for timestamp in TRANSPORT_FAILURES
            if current - timestamp <= TRANSPORT_FAILURE_WINDOW_S
        ]
        if success:
            TRANSPORT_LAST_SUCCESS_UNIX = current
            TRANSPORT_LAST_ERROR = ""
            TRANSPORT_FAILURES.clear()
        else:
            TRANSPORT_FAILURES.append(current)
            TRANSPORT_LAST_ERROR = error[:160]


def transport_status() -> dict[str, Any]:
    current = time.time()
    with TRANSPORT_LOCK:
        recent_failures = sum(
            current - timestamp <= TRANSPORT_FAILURE_WINDOW_S
            for timestamp in TRANSPORT_FAILURES
        )
        circuit_open = recent_failures >= max(1, TRANSPORT_FAILURE_THRESHOLD)
        return {
            "transport_ready": not circuit_open,
            "transport_circuit_open": circuit_open,
            "transport_recent_failures": recent_failures,
            "transport_failure_threshold": max(1, TRANSPORT_FAILURE_THRESHOLD),
            "transport_failure_window_seconds": max(1, TRANSPORT_FAILURE_WINDOW_S),
            "transport_last_success_unix": int(TRANSPORT_LAST_SUCCESS_UNIX),
            "transport_last_error": TRANSPORT_LAST_ERROR,
        }


def relay_upstream_stream(
    response: Any,
    sink: Any,
    capture_limit: int = UPSTREAM_CAPTURE_MAX_BYTES,
) -> tuple[bytes, int, int]:
    captured = bytearray()
    total_bytes = 0
    first_byte_ns = 0
    read_chunk = getattr(response, "read1", None) or response.read
    while True:
        chunk = read_chunk(65_536)
        if not chunk:
            break
        if first_byte_ns == 0:
            first_byte_ns = time.monotonic_ns()
        total_bytes += len(chunk)
        remaining = max(0, capture_limit - len(captured))
        if remaining:
            captured.extend(chunk[:remaining])
        sink(chunk)
    return bytes(captured), total_bytes, first_byte_ns


def economics_window_id() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H")


def token_estimate(text: str) -> int:
    return max(1, (len(text.encode("utf-8")) + 3) // 4)


def stable_receipt(value: Any) -> str:
    material = json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    return hashlib.sha256(material).hexdigest()


def client_intent_identity(
    headers: Any, payload: Any, request_hash: str = ""
) -> tuple[str, str, bool]:
    metadata = payload.get("metadata") if isinstance(payload, dict) else None
    candidates = (
        ("metadata", metadata.get("nando_client_intent_id") if isinstance(metadata, dict) else None),
        ("idempotency_key", headers.get("idempotency-key")),
        ("nando_header", headers.get("x-nando-client-intent-id")),
    )
    for source, value in candidates:
        if isinstance(value, str) and value.strip():
            digest = hashlib.sha256(value.strip().encode("utf-8")).hexdigest()
            return f"intent-{digest}", source, True
    codex_turn = headers.get("x-codex-turn-metadata")
    if isinstance(codex_turn, str) and codex_turn.strip() and request_hash:
        material = f"{codex_turn.strip()}\0{request_hash}".encode("utf-8")
        return f"intent-{hashlib.sha256(material).hexdigest()}", "codex_turn_body", True
    generated = uuid.uuid4().hex
    return f"intent-{generated}", "bridge_generated", False


def provider_attempt_identity() -> str:
    return f"attempt-{uuid.uuid4().hex}"


def valid_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def economics_terminal_row(
    *,
    client_intent_id: str,
    intent_id_source: str,
    intent_dedupe_eligible: bool,
    provider_attempt_id: str | None,
    request_hash: str,
    endpoint: str,
    route: str,
    terminal_state: str,
    input_tokens: int,
    upstream_socket_opened: bool,
    avoided_call: bool,
    verification_status: str,
    verification_receipt_id: str | None = None,
    projector_receipt_id: str | None = None,
    status_code: int | None = None,
    **extra: Any,
) -> dict[str, Any]:
    verified_avoid = (
        avoided_call
        and not upstream_socket_opened
        and verification_status == "verified"
        and valid_sha256(verification_receipt_id)
        and valid_sha256(projector_receipt_id)
        and terminal_state == "delivered"
    )
    return {
        "schema": "nando.economics-terminal.v1",
        "timestamp": now_iso(),
        "window_id": economics_window_id(),
        "client_intent_id": client_intent_id,
        "intent_id_source": intent_id_source,
        "intent_dedupe_eligible": intent_dedupe_eligible,
        "provider_attempt_id": provider_attempt_id,
        "request_sha256": request_hash,
        "endpoint": endpoint,
        "route": route,
        "terminal_state": terminal_state,
        "status_code": status_code,
        "input_tokens": max(0, input_tokens),
        "input_token_measurement": "estimated_utf8_quarter",
        "upstream_socket_opened": upstream_socket_opened,
        "avoided_call": verified_avoid,
        "verification_status": verification_status,
        "verification_receipt_id": verification_receipt_id,
        "projector_receipt_id": projector_receipt_id,
        **extra,
    }


def write_economics_terminal(**values: Any) -> None:
    write_jsonl(ECONOMICS_LEDGER_JSONL, economics_terminal_row(**values))


def write_jsonl(path: Path, row: dict[str, Any]) -> None:
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")
    except OSError:
        pass


def grounded_evidence_receipt(
    before: Any,
    action: Any,
    after: Any,
    evidence_source: str,
    evidence_verifier: str,
    receipt_schema: str = "nando.grounded-transition-receipt.v1",
    observed_at: str = "",
    provenance: dict[str, Any] | None = None,
) -> str:
    material: dict[str, Any] = {
        "before": before,
        "action": action,
        "after": after,
        "evidence_source": evidence_source,
        "evidence_verifier": evidence_verifier,
    }
    if receipt_schema == "nando.grounded-transition-receipt.v2":
        material.update(
            {
                "receipt_schema": receipt_schema,
                "observed_at": observed_at,
                "provenance": provenance or {},
            }
        )
    material_bytes = json.dumps(
        material,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    return hashlib.sha256(material_bytes).hexdigest()


TRANSITION_TRACE_LOCK = threading.Lock()
TRANSITION_TRACE_IDS: set[str] | None = None


def append_transition_once(row: dict[str, Any]) -> str:
    global TRANSITION_TRACE_IDS
    trace_id = str(row.get("trace_id") or "")
    if not trace_id:
        return "invalid"
    with TRANSITION_TRACE_LOCK:
        if TRANSITION_TRACE_IDS is None:
            ids: set[str] = set()
            try:
                with TRANSITION_TRACE_JSONL.open("r", encoding="utf-8") as handle:
                    for line in handle:
                        try:
                            existing = json.loads(line)
                        except json.JSONDecodeError:
                            continue
                        existing_id = existing.get("trace_id")
                        if isinstance(existing_id, str) and existing_id:
                            ids.add(existing_id)
            except OSError:
                pass
            TRANSITION_TRACE_IDS = ids
        if trace_id in TRANSITION_TRACE_IDS:
            return "duplicate"
        try:
            TRANSITION_TRACE_JSONL.parent.mkdir(parents=True, exist_ok=True)
            with TRANSITION_TRACE_JSONL.open("a", encoding="utf-8") as handle:
                handle.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")
        except OSError:
            return "error"
        TRANSITION_TRACE_IDS.add(trace_id)
        return "appended"


def valid_observed_at(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return False
    return parsed.tzinfo is not None


def read_json_dict(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


def env_bool(name: str, default: str = "0") -> bool:
    return os.environ.get(name, default) == "1"


def text_from_message_content(content: Any) -> str:
    if isinstance(content, str):
        return content
    if not isinstance(content, list):
        return ""
    parts: list[str] = []
    for item in content:
        if not isinstance(item, dict):
            continue
        text = item.get("text")
        if isinstance(text, str):
            parts.append(text)
    return "\n".join(parts)


def latest_user_message_text(messages: Any) -> str:
    if not isinstance(messages, list):
        return ""
    fallback: list[str] = []
    user_messages: list[str] = []
    for message in messages:
        if not isinstance(message, dict):
            continue
        text = text_from_message_content(message.get("content"))
        if not text:
            continue
        fallback.append(text)
        if message.get("role") == "user":
            user_messages.append(text)
    if user_messages:
        return user_messages[-1]
    return fallback[-1] if fallback else ""


def extract_request_text(payload: Any) -> str:
    if not isinstance(payload, dict):
        return ""
    request_input = payload.get("input")
    if isinstance(request_input, str):
        return request_input
    if isinstance(request_input, list):
        text = latest_user_message_text(request_input)
        if text:
            return text
    if isinstance(payload.get("prompt"), str):
        return payload["prompt"]
    return latest_user_message_text(payload.get("messages"))


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


def transition_envelope_from_payload(payload: Any) -> dict[str, Any] | None:
    if not isinstance(payload, dict):
        return None
    metadata = payload.get("metadata")
    if not isinstance(metadata, dict):
        return None
    envelope = metadata.get("nando_transition")
    if not isinstance(envelope, dict):
        return None
    if "before" not in envelope or "action" not in envelope:
        return None
    return {"before": envelope["before"], "action": envelope["action"]}


def upstream_output_text(payload: dict[str, Any]) -> str:
    output_text = payload.get("output_text")
    if isinstance(output_text, str):
        return output_text
    choices = payload.get("choices")
    if isinstance(choices, list) and choices:
        choice = choices[0]
        if isinstance(choice, dict):
            message = choice.get("message")
            if isinstance(message, dict) and isinstance(message.get("content"), str):
                return message["content"]
    output = payload.get("output")
    if isinstance(output, list):
        for item in output:
            if not isinstance(item, dict):
                continue
            content = item.get("content")
            if not isinstance(content, list):
                continue
            for part in content:
                if isinstance(part, dict) and isinstance(part.get("text"), str):
                    return part["text"]
    return ""


def observed_after_from_upstream(upstream_body: bytes) -> tuple[Any | None, dict[str, int]]:
    payload = parse_json_body(upstream_body)
    usage = usage_from_payload(payload)
    text = upstream_output_text(payload).strip()
    if not text:
        return None, usage
    try:
        observed = json.loads(text)
    except json.JSONDecodeError:
        return None, usage
    if isinstance(observed, dict) and "after" in observed:
        return observed["after"], usage
    return observed, usage


def write_transition_event(event: str, request_hash: str, tokens: int, **extra: Any) -> None:
    write_jsonl(
        TRANSITION_EVENTS_JSONL,
        {
            "schema": "nando.transition-execution-event.v1",
            "timestamp": now_iso(),
            "event": event,
            "request_sha256": request_hash,
            "tokens": max(0, tokens),
            **extra,
        },
    )


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


def upstream_headers_for_request(headers: Any) -> dict[str, str]:
    forwarded: dict[str, str] = {}
    try:
        items = headers.items()
    except AttributeError:
        items = []
    for name, value in items:
        lower_name = name.lower()
        if lower_name in UPSTREAM_HEADER_ALLOWLIST or lower_name.startswith(UPSTREAM_HEADER_PREFIXES):
            forwarded[name] = value
    if "authorization" not in {name.lower() for name in forwarded} and UPSTREAM_API_KEY:
        forwarded["authorization"] = f"Bearer {UPSTREAM_API_KEY}"
    return forwarded


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
    admission = read_json_dict(TRANSITION_ADMISSION_JSON)
    admission_age = max(0, int(time.time()) - int(admission.get("generated_at_unix") or 0))
    max_admission_age = max(
        1, int(os.environ.get("NANDO_TRANSITION_ADMISSION_MAX_AGE_SECONDS", "30"))
    )
    return (
        env_bool("NANDO_OFFLOAD", "1")
        and env_bool("NANDO_LOCAL_ACCEPT_ENABLED")
        and env_bool("NANDO_CLIENT_ALLOW_LOCAL_ACCEPT")
        and not env_bool("NANDO_CLIENT_KILL_SWITCH")
        and admission.get("eligible_for_local_accept") is True
        and admission.get("verdict") == "PASS"
        and admission_age <= max_admission_age
    )


def try_typed_transition_executor(
    envelope: dict[str, Any],
) -> tuple[bool, dict[str, Any], str]:
    if not local_policy_allows():
        return False, {}, "typed_local_policy_disabled"
    request = {
        "schema": "nando.live-transition-request.v1",
        "before": envelope["before"],
        "action": envelope["action"],
    }
    started = time.monotonic_ns()
    try:
        completed = subprocess.run(
            [TYPED_TRANSITION_CMD],
            input=json.dumps(request, ensure_ascii=False, separators=(",", ":")),
            text=True,
            capture_output=True,
            timeout=max(0.001, TIMEOUT_MS / 1000),
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return False, {}, "typed_executor_unavailable"
    elapsed_ns = time.monotonic_ns() - started
    if completed.returncode != 0:
        return False, {"elapsed_ns": elapsed_ns}, "typed_executor_error"
    try:
        local = json.loads(completed.stdout)
    except json.JSONDecodeError:
        return False, {"elapsed_ns": elapsed_ns}, "typed_executor_invalid_json"
    verifier_ok = bool(local.get("verifier_ok"))
    false_accepts = int(local.get("false_accepts") or 0)
    response = str(local.get("response") or "")
    verification_receipt_id = local.get("verification_receipt_id")
    verified_after_digest = local.get("verified_after_digest")
    verifier_schema = local.get("verifier_schema")
    if env_bool("NANDO_CLIENT_REQUIRE_VERIFIER", "1") and not verifier_ok:
        return False, {**local, "elapsed_ns": elapsed_ns}, "typed_verifier_required_not_ok"
    if env_bool("NANDO_CLIENT_REQUIRE_FALSE_ACCEPTS_ZERO", "1") and false_accepts != 0:
        return False, {**local, "elapsed_ns": elapsed_ns}, "typed_false_accepts_nonzero"
    if not (
        valid_sha256(verification_receipt_id)
        and valid_sha256(verified_after_digest)
        and verifier_schema == "typed_actor_independent_verifier.v1"
    ):
        return False, {**local, "elapsed_ns": elapsed_ns}, "typed_verification_receipt_missing"
    try:
        actor_payload = json.loads(response)
    except json.JSONDecodeError:
        return False, {**local, "elapsed_ns": elapsed_ns}, "typed_response_invalid_json"
    if stable_receipt(actor_payload.get("after")) != verified_after_digest:
        return False, {**local, "elapsed_ns": elapsed_ns}, "typed_verified_after_digest_mismatch"
    if not (bool(local.get("local_accept")) and verifier_ok and response):
        return False, {**local, "elapsed_ns": elapsed_ns}, "typed_local_declined"
    return True, {**local, "elapsed_ns": elapsed_ns}, "typed_verifier_bound_local_accept"


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
                "id": "msg_nando_local",
                "type": "message",
                "status": "completed",
                "role": "assistant",
                "content": [
                    {
                        "type": "output_text",
                        "annotations": [],
                        "logprobs": [],
                        "text": response_text,
                    }
                ],
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


def responses_sse_body(response: dict[str, Any]) -> bytes:
    completed_item = response["output"][0]
    completed_part = completed_item["content"][0]
    in_progress_response = {**response, "status": "in_progress", "output": []}
    in_progress_item = {**completed_item, "status": "in_progress", "content": []}
    events = [
        ("response.created", {"response": in_progress_response}),
        (
            "response.output_item.added",
            {"output_index": 0, "item": in_progress_item},
        ),
        (
            "response.content_part.added",
            {
                "item_id": completed_item["id"],
                "output_index": 0,
                "content_index": 0,
                "part": {"type": "output_text", "annotations": [], "text": ""},
            },
        ),
        (
            "response.output_text.delta",
            {
                "item_id": completed_item["id"],
                "output_index": 0,
                "content_index": 0,
                "delta": completed_part["text"],
                "logprobs": [],
            },
        ),
        (
            "response.output_text.done",
            {
                "item_id": completed_item["id"],
                "output_index": 0,
                "content_index": 0,
                "text": completed_part["text"],
                "logprobs": [],
            },
        ),
        (
            "response.content_part.done",
            {
                "item_id": completed_item["id"],
                "output_index": 0,
                "content_index": 0,
                "part": completed_part,
            },
        ),
        (
            "response.output_item.done",
            {"output_index": 0, "item": completed_item},
        ),
        ("response.completed", {"response": response}),
    ]
    chunks: list[str] = []
    for sequence_number, (event_type, event_payload) in enumerate(events):
        payload = {
            "type": event_type,
            "sequence_number": sequence_number,
            **event_payload,
        }
        chunks.append(
            f"event: {event_type}\ndata: "
            f"{json.dumps(payload, ensure_ascii=False, separators=(',', ':'))}\n\n"
        )
    chunks.append("data: [DONE]\n\n")
    return "".join(chunks).encode("utf-8")


class BridgeHandler(BaseHTTPRequestHandler):
    server_version = "nando-provider-bridge/0.1"
    protocol_version = "HTTP/1.1"

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

    def send_responses_stream(self, payload: dict[str, Any]) -> None:
        body = responses_sse_body(payload)
        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
        self.send_header("cache-control", "no-cache")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:
        if self.path in {"/health", "/v1/health", "/v2/health"}:
            transition_metrics = read_json_dict(TRANSITION_METRICS_JSON)
            transition_admission = read_json_dict(TRANSITION_ADMISSION_JSON)
            economics = read_json_dict(ECONOMICS_SNAPSHOT_JSON)
            transport = transport_status()
            self.send_json(
                200,
                {
                    "ok": True,
                    "status": "ok",
                    "service": "nando-provider-bridge",
                    "local_accept_enabled": env_bool("NANDO_LOCAL_ACCEPT_ENABLED"),
                    "client_allow_local_accept": env_bool("NANDO_CLIENT_ALLOW_LOCAL_ACCEPT"),
                    "effective_local_accept_enabled": local_policy_allows(),
                    "safety_policy": os.environ.get("NANDO_CLIENT_SAFETY_POLICY", ""),
                    "upstream_configured": bool(UPSTREAM_BASE_URL),
                    "upstream_base_url_configured": bool(UPSTREAM_BASE_URL),
                    **transport,
                    "upstream_server_api_key_configured": bool(
                        UPSTREAM_BASE_URL and UPSTREAM_API_KEY
                    ),
                    "client_auth_forwarding_supported": True,
                    "api_key_present": bool(UPSTREAM_API_KEY),
                    "api_key_value_printed": False,
                    "default_client_api_version": "v2",
                    "supported_api_versions": ["v1", "v2"],
                    "v2_architecture": "compact_latent_transition_runtime",
                    "autonomous_transition_profiles_enabled": True,
                    "transition_profile_state": transition_metrics.get("verdict", "starting"),
                    "transition_active_profiles": int(
                        transition_metrics.get("active_profile_count") or 0
                    ),
                    "transition_non_raw_active_profiles": int(
                        transition_metrics.get("non_raw_active_profile_count") or 0
                    ),
                    "transition_false_accepts": int(
                        transition_metrics.get("false_accepts") or 0
                    ),
                    "transition_raw_phase_enabled": bool(
                        transition_metrics.get("raw_phase_enabled")
                    ),
                    "transition_raw_phase_families": int(
                        transition_metrics.get("raw_phase_family_count") or 0
                    ),
                    "transition_raw_phase_cleanup_families": int(
                        transition_metrics.get("raw_phase_cleanup_families") or 0
                    ),
                    "transition_raw_phase_total_observed_surfaces": int(
                        transition_metrics.get("raw_phase_total_observed_surfaces") or 0
                    ),
                    "transition_raw_phase_total_covered_surfaces": int(
                        transition_metrics.get("raw_phase_total_covered_surfaces") or 0
                    ),
                    "transition_raw_phase_total_unsupported_surfaces": int(
                        transition_metrics.get("raw_phase_total_unsupported_surfaces") or 0
                    ),
                    "transition_raw_phase_frontier_observed_tokens": int(
                        transition_metrics.get("raw_phase_frontier_observed_tokens") or 0
                    ),
                    "transition_raw_phase_frontier_covered_tokens": int(
                        transition_metrics.get("raw_phase_frontier_covered_tokens") or 0
                    ),
                    "transition_raw_phase_frontier_token_coverage_milli": int(
                        transition_metrics.get("raw_phase_frontier_token_coverage_milli") or 0
                    ),
                    "transition_raw_phase_max_surface_sessions": int(
                        transition_metrics.get("raw_phase_max_surface_sessions") or 0
                    ),
                    "transition_raw_phase_transfer_pass_families": int(
                        transition_metrics.get("raw_phase_transfer_pass_families") or 0
                    ),
                    "transition_raw_phase_transfer_tested_surfaces": int(
                        transition_metrics.get("raw_phase_transfer_tested_surfaces") or 0
                    ),
                    "transition_raw_phase_transfer_passed_surfaces": int(
                        transition_metrics.get("raw_phase_transfer_passed_surfaces") or 0
                    ),
                    "transition_raw_phase_transfer_query_rows": int(
                        transition_metrics.get("raw_phase_transfer_query_rows") or 0
                    ),
                    "transition_raw_phase_transfer_correct_executions": int(
                        transition_metrics.get("raw_phase_transfer_correct_executions") or 0
                    ),
                    "transition_raw_phase_transfer_abstains": int(
                        transition_metrics.get("raw_phase_transfer_abstains") or 0
                    ),
                    "transition_raw_phase_transfer_wrong_accepts": int(
                        transition_metrics.get("raw_phase_transfer_wrong_accepts") or 0
                    ),
                    "transition_raw_phase_session_transfer_pass_families": int(
                        transition_metrics.get("raw_phase_session_transfer_pass_families") or 0
                    ),
                    "transition_raw_phase_session_transfer_query_rows": int(
                        transition_metrics.get("raw_phase_session_transfer_query_rows") or 0
                    ),
                    "transition_raw_phase_session_transfer_wrong_accepts": int(
                        transition_metrics.get("raw_phase_session_transfer_wrong_accepts") or 0
                    ),
                    "transition_raw_phase_time_transfer_pass_families": int(
                        transition_metrics.get("raw_phase_time_transfer_pass_families") or 0
                    ),
                    "transition_raw_phase_time_transfer_query_rows": int(
                        transition_metrics.get("raw_phase_time_transfer_query_rows") or 0
                    ),
                    "transition_raw_phase_time_transfer_wrong_accepts": int(
                        transition_metrics.get("raw_phase_time_transfer_wrong_accepts") or 0
                    ),
                    "economics_schema": economics.get("schema", "missing"),
                    "economics_unique_client_intents": int(
                        economics.get("unique_client_intents") or 0
                    ),
                    "economics_dedupe_eligible_client_intents": int(
                        economics.get("dedupe_eligible_client_intents") or 0
                    ),
                    "economics_verified_local_accepts": int(
                        economics.get("verified_local_accepts") or 0
                    ),
                    "economics_avoided_calls": int(economics.get("avoided_calls") or 0),
                    "economics_avoided_input_tokens": int(
                        economics.get("avoided_input_tokens") or 0
                    ),
                    "economics_input_token_saving_share_milli": int(
                        economics.get("input_token_saving_share_milli") or 0
                    ),
                    "economics_verification_coverage_milli": int(
                        economics.get("verification_coverage_milli") or 0
                    ),
                    "economics_unresolved_local_outcomes": int(
                        economics.get("unresolved_local_outcomes") or 0
                    ),
                    "economics_missing_evidence_receipts": int(
                        economics.get("missing_evidence_receipts") or 0
                    ),
                    "economics_hard_gate_pass": bool(economics.get("hard_gate_pass")),
                    "economics_product_m1_pass": bool(economics.get("product_m1_pass")),
                    "transition_admission_verdict": transition_admission.get(
                        "verdict", "MISSING"
                    ),
                    "transition_local_accept_eligible": bool(
                        transition_admission.get("eligible_for_local_accept")
                    ),
                    "transition_admission_generated_at_unix": int(
                        transition_admission.get("generated_at_unix") or 0
                    ),
                    "status_dashboard_enabled": dashboard_enabled(),
                },
            )
            return
        if self.path.startswith(("/v1/models", "/v2/models")):
            self.proxy_upstream_get()
            return
        if dashboard_key_for_path(self.path):
            if dashboard_path_authorized(self.path):
                self.send_html(200, status_dashboard_html(self.path))
            else:
                self.send_json(404, {"error": {"message": "not found", "type": "not_found"}})
            return
        self.send_json(404, {"error": {"message": "not found", "type": "not_found"}})

    def proxy_upstream_get(self) -> None:
        if not UPSTREAM_BASE_URL:
            self.send_json(
                502,
                {"error": {"message": "upstream is not configured", "type": "upstream_missing"}},
            )
            return
        upstream_path = upstream_path_for_bridge_path(self.path)
        request = urllib.request.Request(
            f"{UPSTREAM_BASE_URL}{upstream_path}",
            headers=upstream_headers_for_request(self.headers),
            method="GET",
        )
        try:
            with urllib.request.urlopen(request, timeout=UPSTREAM_TIMEOUT_S) as response:
                body = response.read()
                self.send_response(response.status)
                self.send_header("content-type", response.headers.get("content-type", "application/json"))
                self.send_header("content-length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
        except urllib.error.HTTPError as error:
            body = error.read()
            self.send_response(error.code)
            self.send_header("content-type", error.headers.get("content-type", "application/json"))
            self.send_header("content-length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        except OSError as error:
            self.send_json(
                502,
                {"error": {"message": f"upstream proxy failed: {error}", "type": "upstream_error"}},
            )

    def handle_transition_execute(self, payload: Any) -> None:
        if not isinstance(payload, dict):
            self.send_json(400, {"error": {"message": "object required", "type": "bad_request"}})
            return
        if payload.get("schema") != "nando.transition-execute.v1":
            self.send_json(400, {"error": {"message": "unsupported schema", "type": "bad_request"}})
            return
        if "before" not in payload or "action" not in payload:
            self.send_json(400, {"error": {"message": "before/action required", "type": "bad_request"}})
            return
        local_ok, local, reason = try_typed_transition_executor(
            {"before": payload["before"], "action": payload["action"]}
        )
        if not local_ok:
            self.send_json(
                409,
                {
                    "schema": "nando.transition-execute-response.v1",
                    "local_accept": False,
                    "fallback_required": True,
                    "reason": reason,
                },
            )
            return
        response = local.get("response")
        try:
            transition = json.loads(response) if isinstance(response, str) else response
        except json.JSONDecodeError:
            self.send_json(502, {"error": {"message": "invalid executor response", "type": "executor_error"}})
            return
        projected = {
            "schema": "nando.transition-execute-response.v1",
            "local_accept": True,
            "fallback_required": False,
            "verifier_ok": True,
            "route": local.get("route"),
            "transition": transition,
            "elapsed_ns": int(local.get("elapsed_ns") or 0),
            "verification_receipt_id": local.get("verification_receipt_id"),
            "verified_after_digest": local.get("verified_after_digest"),
            "verifier_schema": local.get("verifier_schema"),
        }
        projected["projector_receipt_id"] = stable_receipt(projected)
        self.send_json(200, projected)

    def handle_transition_observe(self, payload: Any, request_hash: str) -> None:
        if not isinstance(payload, dict):
            self.send_json(400, {"error": {"message": "object required", "type": "bad_request"}})
            return
        if payload.get("schema") != "nando.transition-observation.v1":
            self.send_json(400, {"error": {"message": "unsupported schema", "type": "bad_request"}})
            return
        if any(key not in payload for key in ("before", "action", "after", "evidence")):
            self.send_json(400, {"error": {"message": "before/action/after/evidence required", "type": "bad_request"}})
            return
        evidence = payload.get("evidence")
        if not isinstance(evidence, dict):
            self.send_json(400, {"error": {"message": "evidence object required", "type": "bad_request"}})
            return
        evidence_source = str(evidence.get("source") or "")
        evidence_verifier = str(evidence.get("verifier") or "")
        receipt_schema = str(
            evidence.get("receipt_schema") or "nando.grounded-transition-receipt.v1"
        )
        supplied_receipt = str(evidence.get("receipt_sha256") or "").lower()
        if evidence_source not in {
            "application_state",
            "tool_result",
            "environment_snapshot",
        } or not evidence_verifier:
            self.send_json(400, {"error": {"message": "unsupported evidence", "type": "bad_request"}})
            return
        provenance = payload.get("provenance")
        provenance = provenance if isinstance(provenance, dict) else {}
        source_session_id_sha256 = str(
            provenance.get("source_session_id_sha256") or ""
        ).lower()
        source_event_id_sha256 = str(
            provenance.get("source_event_id_sha256") or ""
        ).lower()
        call_input_sha256 = str(provenance.get("call_input_sha256") or "").lower()
        call_output_sha256 = str(provenance.get("call_output_sha256") or "").lower()
        provenance_digests = (source_session_id_sha256, source_event_id_sha256)
        if receipt_schema == "nando.grounded-transition-receipt.v2":
            provenance_digests += (call_input_sha256, call_output_sha256)
        for value in provenance_digests:
            if value and (
                len(value) != 64 or not all(character in "0123456789abcdef" for character in value)
            ):
                self.send_json(
                    400,
                    {"error": {"message": "invalid provenance digest", "type": "bad_request"}},
                )
                return
        observed_at = payload.get("observed_at")
        if receipt_schema == "nando.grounded-transition-receipt.v2" and (
            not all(valid_sha256(value) for value in provenance_digests[:2])
            or not valid_observed_at(observed_at)
        ):
            self.send_json(
                400,
                {"error": {"message": "v2 provenance required", "type": "bad_request"}},
            )
            return
        expected_receipt = grounded_evidence_receipt(
            payload["before"],
            payload["action"],
            payload["after"],
            evidence_source,
            evidence_verifier,
            receipt_schema,
            str(observed_at or ""),
            provenance,
        )
        if supplied_receipt != expected_receipt:
            self.send_json(400, {"error": {"message": "evidence receipt mismatch", "type": "bad_request"}})
            return
        trace_id = str(payload.get("trace_id") or request_hash)
        usage = payload.get("usage") if isinstance(payload.get("usage"), dict) else {}
        input_tokens = max(0, int(usage.get("input_tokens") or 0))
        output_tokens = max(0, int(usage.get("output_tokens") or 0))
        total_tokens = max(0, int(usage.get("total_tokens") or input_tokens + output_tokens))
        trace_row = {
                "schema": "nando.live-observed-transition.v2",
                "trace_id": trace_id,
                "timestamp": str(observed_at) if valid_observed_at(observed_at) else now_iso(),
                "before": payload["before"],
                "action": payload["action"],
                "after": payload["after"],
                "input_tokens": input_tokens,
                "output_tokens": output_tokens,
                "total_tokens": total_tokens,
                "request_sha256": request_hash,
                "evidence_source": evidence_source,
                "evidence_verifier": evidence_verifier,
                "evidence_receipt_sha256": expected_receipt,
                "source_session_id_sha256": source_session_id_sha256,
                "source_event_id_sha256": source_event_id_sha256,
        }
        append_status = append_transition_once(trace_row)
        if append_status == "error":
            self.send_json(503, {"error": {"message": "trace store unavailable", "type": "storage_error"}})
            return
        if append_status == "invalid":
            self.send_json(400, {"error": {"message": "trace_id required", "type": "bad_request"}})
            return
        if append_status == "appended":
            write_transition_event(
                "grounded_transition_observation",
                request_hash,
                total_tokens,
                trace_id=trace_id,
                evidence_source=evidence_source,
                evidence_verifier=evidence_verifier,
            )
        self.send_json(
            202,
            {
                "schema": "nando.transition-observation-response.v1",
                "accepted": True,
                "grounded": True,
                "duplicate": append_status == "duplicate",
                "trace_id": trace_id,
                "receipt_sha256": expected_receipt,
            },
        )

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
        try:
            payload = json.loads(body)
        except json.JSONDecodeError:
            self.send_json(400, {"error": {"message": "invalid JSON", "type": "bad_request"}})
            return

        client_intent_id, intent_id_source, intent_dedupe_eligible = client_intent_identity(
            self.headers, payload, request_hash
        )
        write_jsonl(
            EVENTS_JSONL,
            {
                "schema_version": "nando_provider_bridge_event_v1",
                "timestamp": now_iso(),
                "stage": "ingress",
                "path": self.path,
                "api_version": api_version,
                "endpoint": endpoint,
                "client_intent_id": client_intent_id,
                "intent_id_source": intent_id_source,
                "intent_dedupe_eligible": intent_dedupe_eligible,
                "request_sha256": request_hash,
                "request_bytes": len(body),
                "local_accept_enabled": env_bool("NANDO_LOCAL_ACCEPT_ENABLED"),
                "client_allow_local_accept": env_bool("NANDO_CLIENT_ALLOW_LOCAL_ACCEPT"),
                "safety_policy": os.environ.get("NANDO_CLIENT_SAFETY_POLICY", ""),
            },
        )

        if self.path.rstrip("/") == "/v2/transitions/execute":
            self.handle_transition_execute(payload)
            return
        if self.path.rstrip("/") == "/v2/transitions/observe":
            self.handle_transition_observe(payload, request_hash)
            return

        request_text = extract_request_text(payload)
        traffic_source = traffic_source_from_payload(payload)
        transition_envelope = transition_envelope_from_payload(payload)
        transition_tokens = token_estimate(request_text)
        write_transition_event(
            "bridge_request",
            request_hash,
            transition_tokens,
            traffic_source=traffic_source,
            client_intent_id=client_intent_id,
            intent_dedupe_eligible=intent_dedupe_eligible,
        )
        if transition_envelope is not None:
            write_transition_event(
                "transition_request",
                request_hash,
                transition_tokens,
                traffic_source=traffic_source,
                client_intent_id=client_intent_id,
                intent_dedupe_eligible=intent_dedupe_eligible,
            )
            shadow_sample = int(request_hash[:8], 16) % 64 == 0
            if shadow_sample:
                local_ok, local, reason = False, {}, "typed_continuous_shadow_sample"
            else:
                local_ok, local, reason = try_typed_transition_executor(transition_envelope)
        else:
            local_ok, local, reason = False, {}, "no_grounded_transition_envelope"
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
                    "client_intent_id": client_intent_id,
                    "intent_dedupe_eligible": intent_dedupe_eligible,
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
            verification_receipt_id = None
            projector_receipt_id = None
            verification_status = "not_applicable"
            if transition_envelope is not None:
                verification_status = "verified"
                verification_receipt_id = str(local.get("verification_receipt_id"))
                projector_receipt_id = stable_receipt(
                    {
                        "client_intent_id": client_intent_id,
                        "endpoint": endpoint,
                        "response_payload": response_payload,
                    }
                )
            if endpoint == "responses" and payload.get("stream") is True:
                self.send_responses_stream(response_payload)
            else:
                self.send_json(200, response_payload)
            write_economics_terminal(
                client_intent_id=client_intent_id,
                intent_id_source=intent_id_source,
                intent_dedupe_eligible=intent_dedupe_eligible,
                provider_attempt_id=None,
                request_hash=request_hash,
                endpoint=endpoint,
                route="local_actor" if transition_envelope is not None else "local_canary",
                terminal_state="delivered",
                input_tokens=transition_tokens,
                upstream_socket_opened=False,
                avoided_call=transition_envelope is not None,
                verification_status=verification_status,
                verification_receipt_id=verification_receipt_id,
                projector_receipt_id=projector_receipt_id,
                status_code=200,
                local_route=route,
                verified_after_digest=local.get("verified_after_digest"),
                verifier_schema=local.get("verifier_schema"),
            )
            if transition_envelope is not None:
                write_transition_event(
                    "local_accept",
                    request_hash,
                    transition_tokens,
                    route=route,
                    elapsed_ns=elapsed_ns,
                    client_intent_id=client_intent_id,
                    verification_receipt_id=verification_receipt_id,
                    projector_receipt_id=projector_receipt_id,
                    verified_after_digest=local.get("verified_after_digest"),
                    verifier_schema=local.get("verifier_schema"),
                )
            return

        self.proxy_upstream(
            body,
            request_hash,
            reason,
            started_ns,
            api_version,
            endpoint,
            traffic_source,
            transition_envelope,
            client_intent_id,
            intent_id_source,
            intent_dedupe_eligible,
        )

    def proxy_upstream(
        self,
        body: bytes,
        request_hash: str,
        reason: str,
        started_ns: int,
        api_version: str,
        endpoint: str,
        traffic_source: str,
        transition_envelope: dict[str, Any] | None,
        client_intent_id: str,
        intent_id_source: str,
        intent_dedupe_eligible: bool,
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
                    "client_intent_id": client_intent_id,
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
            write_economics_terminal(
                client_intent_id=client_intent_id,
                intent_id_source=intent_id_source,
                intent_dedupe_eligible=intent_dedupe_eligible,
                provider_attempt_id=None,
                request_hash=request_hash,
                endpoint=endpoint,
                route="fallback_blocked",
                terminal_state="delivered_error",
                input_tokens=token_estimate(extract_request_text(parse_json_body(body))),
                upstream_socket_opened=False,
                avoided_call=False,
                verification_status="not_verified",
                status_code=502,
                fallback_reason=reason,
            )
            return

        upstream_path = upstream_path_for_bridge_path(self.path)
        url = f"{UPSTREAM_BASE_URL}{upstream_path}"
        headers = upstream_headers_for_request(self.headers)
        request = urllib.request.Request(url, data=body, headers=headers, method="POST")
        provider_attempt_id = provider_attempt_identity()
        input_tokens = token_estimate(extract_request_text(parse_json_body(body)))
        request_payload = parse_json_body(body)
        stream_requested = bool(
            isinstance(request_payload, dict) and request_payload.get("stream") is True
        )
        terminal_state = "upstream_error"
        terminal_status_code: int | None = None
        upstream_headers_ns = 0
        first_upstream_byte_ns = 0
        relayed_bytes = 0
        response_started = False
        write_jsonl(
            EVENTS_JSONL,
            {
                "schema_version": "nando_provider_bridge_event_v1",
                "timestamp": now_iso(),
                "stage": "upstream_attempt",
                "client_intent_id": client_intent_id,
                "provider_attempt_id": provider_attempt_id,
                "request_sha256": request_hash,
                "upstream_path": upstream_path,
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=UPSTREAM_TIMEOUT_S) as response:
                terminal_status_code = response.status
                upstream_headers_ns = time.monotonic_ns()
                content_type = response.headers.get("content-type", "application/json")
                streaming_response = stream_requested or content_type.startswith("text/event-stream")
                if streaming_response:
                    self.send_response(response.status)
                    self.send_header("content-type", content_type)
                    self.send_header("cache-control", response.headers.get("cache-control", "no-cache"))
                    self.send_header("connection", "close")
                    self.end_headers()
                    self.close_connection = True
                    response_started = True

                    def write_chunk(chunk: bytes) -> None:
                        try:
                            self.wfile.write(chunk)
                            self.wfile.flush()
                        except (BrokenPipeError, ConnectionResetError) as error:
                            raise ClientDisconnected from error

                    upstream_body, relayed_bytes, first_upstream_byte_ns = relay_upstream_stream(
                        response,
                        write_chunk,
                    )
                else:
                    upstream_body = response.read()
                    relayed_bytes = len(upstream_body)
                    first_upstream_byte_ns = time.monotonic_ns()
                    self.send_response(response.status)
                    self.send_header("content-type", content_type)
                    self.send_header("content-length", str(len(upstream_body)))
                    self.end_headers()
                    response_started = True
                    self.wfile.write(upstream_body)
                write_provider_boundary_event(
                    request_hash=request_hash,
                    path=upstream_path,
                    status_code=response.status,
                    headers=response.headers,
                    upstream_body=upstream_body,
                )
                if transition_envelope is not None and env_bool(
                    "NANDO_ALLOW_UPSTREAM_SELF_REPORTED_TRANSITIONS", "0"
                ):
                    observed_after, usage = observed_after_from_upstream(upstream_body)
                    if observed_after is not None:
                        write_jsonl(
                            TRANSITION_TRACE_JSONL,
                            {
                                "schema": "nando.live-observed-transition.v1",
                                "trace_id": request_hash,
                                "timestamp": now_iso(),
                                "before": transition_envelope["before"],
                                "action": transition_envelope["action"],
                                "after": observed_after,
                                "input_tokens": usage["input_tokens"],
                                "output_tokens": usage["output_tokens"],
                                "total_tokens": usage["total_tokens"],
                                "request_sha256": request_hash,
                            },
                        )
                        write_transition_event(
                            "future_shadow_observation",
                            request_hash,
                            usage["total_tokens"],
                            status_code=response.status,
                        )
                elif transition_envelope is not None:
                    write_transition_event(
                        "upstream_transition_not_grounded",
                        request_hash,
                        0,
                        status_code=response.status,
                        required_endpoint="/v2/transitions/observe",
                    )
                terminal_state = "delivered"
                record_transport_result(True)
        except ClientDisconnected:
            terminal_state = "client_disconnected"
        except urllib.error.HTTPError as error:
            upstream_body = error.read()
            terminal_status_code = error.code
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
            terminal_state = "delivered_provider_error"
            record_transport_result(True)
        except OSError as error:
            record_transport_result(False, f"{type(error).__name__}:{error}")
            if not response_started:
                self.send_json(
                    502,
                    {"error": {"message": f"upstream proxy failed: {error}", "type": "upstream_error"}},
                )
            else:
                self.close_connection = True
            terminal_status_code = 502
            terminal_state = "stream_interrupted" if response_started else "delivered_bridge_error"
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
                    "client_intent_id": client_intent_id,
                    "provider_attempt_id": provider_attempt_id,
                    "elapsed_ns": elapsed_ns,
                    "upstream_configured": bool(UPSTREAM_BASE_URL),
                    "upstream_path": upstream_path,
                    "upstream_headers_ns": upstream_headers_ns,
                    "first_upstream_byte_ns": first_upstream_byte_ns,
                    "relayed_bytes": relayed_bytes,
                    "stream_requested": stream_requested,
                    "transport": transport_status(),
                },
            )
            write_economics_terminal(
                client_intent_id=client_intent_id,
                intent_id_source=intent_id_source,
                intent_dedupe_eligible=intent_dedupe_eligible,
                provider_attempt_id=provider_attempt_id,
                request_hash=request_hash,
                endpoint=endpoint,
                route="upstream",
                terminal_state=terminal_state,
                input_tokens=input_tokens,
                upstream_socket_opened=True,
                avoided_call=False,
                verification_status="not_applicable",
                status_code=terminal_status_code,
                fallback_reason=reason,
                elapsed_ns=elapsed_ns,
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
