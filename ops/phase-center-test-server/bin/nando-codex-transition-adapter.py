#!/usr/bin/env python3
"""Convert a Codex PostToolUse event into a bounded grounded transition."""

from __future__ import annotations

import hashlib
import json
import sys
from datetime import datetime, timezone
from typing import Any


SCHEMA = "nando.transition-observation.v1"
EVIDENCE_SOURCE = "tool_result"
EVIDENCE_VERIFIER = "codex_post_tool_use_payload_v1"
EVIDENCE_RECEIPT_SCHEMA = "nando.grounded-transition-receipt.v2"


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def evidence_receipt(
    before: Any,
    action: Any,
    after: Any,
    provenance: dict[str, Any] | None = None,
    observed_at: str = "",
    verifier: str = EVIDENCE_VERIFIER,
) -> str:
    material = {
        "receipt_schema": EVIDENCE_RECEIPT_SCHEMA,
        "before": before,
        "action": action,
        "after": after,
        "evidence_source": EVIDENCE_SOURCE,
        "evidence_verifier": verifier,
        "observed_at": observed_at,
        "provenance": provenance or {},
    }
    return hashlib.sha256(canonical_bytes(material)).hexdigest()


def tool_family(tool_name: str) -> str:
    lowered = tool_name.lower()
    if lowered in {"bash", "shell", "exec_command"} or "shell" in lowered:
        return "shell"
    if any(token in lowered for token in ("apply_patch", "edit", "write")):
        return "file_edit"
    if any(token in lowered for token in ("read", "view", "cat", "open")):
        return "file_read"
    return "tool_call"


def tool_outcome(response: Any) -> str:
    if isinstance(response, dict):
        exit_code = response.get("exit_code", response.get("exitCode"))
        if isinstance(exit_code, int):
            return "succeeded" if exit_code == 0 else "failed"
        if response.get("is_error") is True or response.get("error") is not None:
            return "failed"
        status = response.get("status")
        if isinstance(status, str) and status.lower() in {
            "error",
            "failed",
            "failure",
            "rejected",
        }:
            return "failed"
    return "completed"


def tool_operation_kind(tool_name: str, tool_input: Any) -> str:
    family = tool_family(tool_name)
    if family == "shell" and isinstance(tool_input, dict):
        command = str(tool_input.get("command") or tool_input.get("cmd") or "").lower()
        if any(token in command for token in ("cargo test", "pytest", "unittest", "npm test")):
            return "test"
        if any(token in command for token in ("cargo check", "cargo build", "clippy", "compile")):
            return "build"
        if any(token in command for token in ("install ", "systemctl ", "chmod ", "mkdir ")):
            return "mutate"
        if any(token in command for token in ("rg ", "sed ", "jq ", "curl ", "cat ")):
            return "inspect"
        return "shell_other"
    lowered = tool_name.lower()
    if "apply_patch" in lowered or "edit" in lowered:
        return "edit"
    if "write" in lowered:
        return "write"
    if any(token in lowered for token in ("read", "view", "cat", "open")):
        return "read"
    return "call"


def lifecycle_transition(
    family: str, operation: str, run_id: str, outcome: str
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    if family == "shell":
        before = {"tool_runs": [{"run_id": run_id, "runner": family, "status": "pending"}]}
        action = {"event": {"kind": operation, "target": run_id, "value": outcome}}
        after = {"tool_runs": [{"run_id": run_id, "runner": family, "status": outcome}]}
    elif family == "file_edit":
        before = {
            "file_operations": [
                {"operation_id": run_id, "editor": family, "result": "pending"}
            ]
        }
        action = {
            "change": {"operation": operation, "reference": run_id, "result": outcome}
        }
        after = {
            "file_operations": [
                {"operation_id": run_id, "editor": family, "result": outcome}
            ]
        }
    elif family == "file_read":
        before = {
            "read_requests": [
                {"request_id": run_id, "reader": family, "state": "pending"}
            ]
        }
        action = {
            "observation": {"kind": operation, "request": run_id, "state": outcome}
        }
        after = {
            "read_requests": [
                {"request_id": run_id, "reader": family, "state": outcome}
            ]
        }
    else:
        before = {"tool_calls": [{"call_id": run_id, "provider": family, "outcome": "pending"}]}
        action = {"call": {"operation": operation, "id": run_id, "outcome": outcome}}
        after = {"tool_calls": [{"call_id": run_id, "provider": family, "outcome": outcome}]}
    return before, action, after


def build_observation(event: Any) -> dict[str, Any] | None:
    if not isinstance(event, dict) or event.get("hook_event_name") != "PostToolUse":
        return None
    session_id = event.get("session_id")
    tool_name = event.get("tool_name")
    if not isinstance(session_id, str) or not session_id:
        return None
    if not isinstance(tool_name, str) or not tool_name:
        return None

    family = tool_family(tool_name)
    operation = tool_operation_kind(tool_name, event.get("tool_input"))
    outcome = tool_outcome(event.get("tool_response"))
    identity_material = {
        "session_id": session_id,
        "turn_id": event.get("turn_id"),
        "tool_use_id": event.get("tool_use_id"),
        "tool_name": tool_name,
        "tool_input": event.get("tool_input"),
        "tool_response": event.get("tool_response"),
    }
    digest = hashlib.sha256(canonical_bytes(identity_material)).hexdigest()
    run_id = digest[:24]
    before, action, after = lifecycle_transition(family, operation, run_id, outcome)
    session_digest = hashlib.sha256(session_id.encode("utf-8")).hexdigest()
    observed_at = event.get("timestamp")
    if not isinstance(observed_at, str) or not observed_at:
        observed_at = datetime.now(timezone.utc).isoformat()
    provenance = {
        "source_session_id_sha256": session_digest,
        "source_event_id_sha256": digest,
    }
    receipt = evidence_receipt(before, action, after, provenance, observed_at)
    return {
        "schema": SCHEMA,
        "trace_id": f"codex-tool-{digest}",
        "observed_at": observed_at,
        "before": before,
        "action": action,
        "after": after,
        "evidence": {
            "source": EVIDENCE_SOURCE,
            "verifier": EVIDENCE_VERIFIER,
            "receipt_schema": EVIDENCE_RECEIPT_SCHEMA,
            "receipt_sha256": receipt,
        },
        "provenance": provenance,
        "usage": {"input_tokens": 0, "output_tokens": 0, "total_tokens": 0},
    }


def main() -> int:
    try:
        event = json.load(sys.stdin)
        observation = build_observation(event)
        if observation is not None:
            json.dump(observation, sys.stdout, ensure_ascii=False, separators=(",", ":"))
            sys.stdout.write("\n")
    except (OSError, TypeError, ValueError):
        return 0
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
