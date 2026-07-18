#!/usr/bin/env python3
"""Incrementally convert grounded Codex call/output pairs into transitions."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
import tempfile
import urllib.error
import urllib.request
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any


STATE_SCHEMA = "nando.codex-session-capture-state.v1"
OBSERVATION_SCHEMA = "nando.transition-observation.v1"
PROVIDER_OUTCOME_SCHEMA = "nando.provider-outcome.v1"
RESPONSE_RELATION_SCHEMA = "nando.response-relation-observation.v1"
RESPONSE_SHADOW_SCHEMA = "nando.response-shadow-observation.v1"
RELATION_FRAME_SCHEMA = "nando.response-relation-frame.v1"
EVIDENCE_SOURCE = "tool_result"
EVIDENCE_VERIFIER = "codex_session_call_output_pair_v1"
MAX_OUTBOX = 4096
MAX_DELIVERED_IDS = 16384
MAX_PROVIDER_TURNS = 16384
MAX_PROVIDER_REQUESTS_PER_TURN = 64
MAX_PENDING_TERMINALS = 4096
MAX_PENDING_WAIT_CALLS_PER_FILE = 64
DEFAULT_GENERIC_RELATION_FILES_PER_CYCLE = 4
DEFAULT_WAIT_RELATION_FILES_PER_CYCLE = 4
DEFAULT_GENERIC_RELATION_BACKFILL_BYTES = 8 * 1024 * 1024
DEFAULT_WAIT_RELATION_BACKFILL_BYTES = 8 * 1024 * 1024
WAIT_RELATION_CLASSIFIER_VERSION = "surface-v8-canonical-scheduling-policy"
RELATION_EXTRACTOR_VERSION = "response-relation-extractor.v10"


def load_adapter() -> Any:
    path = Path(__file__).with_name("nando-codex-transition-adapter.py")
    spec = importlib.util.spec_from_file_location("nando_codex_transition_adapter_shared", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load transition adapter: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


ADAPTER = load_adapter()


def load_response_relation_adapter() -> Any:
    path = Path(__file__).with_name("nando-codex-response-relations.py")
    spec = importlib.util.spec_from_file_location("nando_codex_response_relations_shared", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load response relation adapter: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


RESPONSE_RELATIONS = load_response_relation_adapter()
GENERIC_RELATION_CLASSIFIER_VERSION = RESPONSE_RELATIONS.CLASSIFIER_VERSION
MAX_RELATION_ACTION_NAME_BYTES = 128
MAX_RELATION_CARDINALITY = (1 << 32) - 1
WAIT_CONTEXT_ROLES = frozenset(
    {
        "turn_call_count_band",
        "turn_output_count_band",
        "turn_pending_count_band",
        "active_pending_handle_count_band",
        "turn_message_count_band",
        "turn_call_shape_count_band",
    }
)
WAIT_SCHEDULING_ARGUMENT_BOUNDS = {
    "yield_time_ms": (250, 30_000),
    "max_tokens": (1, 100_000),
}
WAIT_CANONICAL_SCHEDULING = {"yield_time_ms": 30_000, "max_tokens": 5_000}


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def sha256_value(value: Any) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def sha256_text(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def count_band(value: int) -> int:
    if value <= 0:
        return 0
    return 1 << (value.bit_length() - 1)


def empty_wait_turn_state() -> dict[str, Any]:
    return {
        "calls": 0,
        "outputs": 0,
        "pending_outputs": 0,
        "messages": 0,
        "call_shapes": [],
    }


def wait_turn_context_atoms(turn: dict[str, Any]) -> list[dict[str, Any]]:
    call_shapes = turn.get("call_shapes")
    distinct_call_shapes = len(set(call_shapes)) if isinstance(call_shapes, list) else 0
    return [
        {
            "kind": "cardinality",
            "role": "turn_call_count_band",
            "count": count_band(int(turn.get("calls") or 0)),
        },
        {
            "kind": "cardinality",
            "role": "turn_output_count_band",
            "count": count_band(int(turn.get("outputs") or 0)),
        },
        {
            "kind": "cardinality",
            "role": "turn_pending_count_band",
            "count": count_band(int(turn.get("pending_outputs") or 0)),
        },
        {
            "kind": "cardinality",
            "role": "turn_message_count_band",
            "count": count_band(int(turn.get("messages") or 0)),
        },
        {
            "kind": "cardinality",
            "role": "turn_call_shape_count_band",
            "count": count_band(distinct_call_shapes),
        },
    ]


def bounded_function_action(payload: dict[str, Any]) -> tuple[str, dict[str, Any] | None]:
    if payload.get("type") != "function_call":
        return "", None
    name = payload.get("name")
    arguments = payload.get("arguments")
    if not isinstance(name, str) or not name or not isinstance(arguments, str):
        return "", None
    try:
        name_size = len(name.encode("utf-8"))
        argument_size = len(arguments.encode("utf-8"))
    except UnicodeEncodeError:
        return "", None
    if (
        name_size > MAX_RELATION_ACTION_NAME_BYTES
        or argument_size > RESPONSE_RELATIONS.MAX_OUTPUT_BYTES
    ):
        return "", None
    try:
        parsed = json.loads(arguments)
    except (ValueError, RecursionError):
        return "", None
    return (name, parsed) if isinstance(parsed, dict) else ("", None)


def bounded_wait_scheduling_atoms(
    arguments: dict[str, Any],
) -> list[dict[str, Any]] | None:
    if set(arguments) - ({"cell_id"} | set(WAIT_SCHEDULING_ARGUMENT_BOUNDS)):
        return None
    for name, (minimum, maximum) in WAIT_SCHEDULING_ARGUMENT_BOUNDS.items():
        if name not in arguments:
            continue
        value = arguments[name]
        if type(value) is not int or not minimum <= value <= maximum:
            return None
    return [
        {"kind": "action_integer_argument", "name": name, "value": value}
        for name, value in WAIT_CANONICAL_SCHEDULING.items()
    ]


def bounded_wait_context_atoms(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list):
        return []
    atoms = []
    seen_roles = set()
    for atom in value[: len(WAIT_CONTEXT_ROLES)]:
        if not isinstance(atom, dict) or atom.get("kind") != "cardinality":
            continue
        role = atom.get("role")
        count = atom.get("count")
        if (
            role not in WAIT_CONTEXT_ROLES
            or role in seen_roles
            or type(count) is not int
            or not 0 <= count <= MAX_RELATION_CARDINALITY
        ):
            continue
        atoms.append({"kind": "cardinality", "role": role, "count": count})
        seen_roles.add(role)
    return atoms


def nested_tool_calls(source: str) -> list[str]:
    calls: list[str] = []
    index = 0
    quote = ""
    escaped = False
    line_comment = False
    block_comment = False
    while index < len(source):
        character = source[index]
        following = source[index + 1] if index + 1 < len(source) else ""
        if line_comment:
            if character == "\n":
                line_comment = False
            index += 1
            continue
        if block_comment:
            if character == "*" and following == "/":
                block_comment = False
                index += 2
            else:
                index += 1
            continue
        if quote:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == quote:
                quote = ""
            index += 1
            continue
        if character in {'"', "'", "`"}:
            quote = character
            index += 1
            continue
        if character == "/" and following == "/":
            line_comment = True
            index += 2
            continue
        if character == "/" and following == "*":
            block_comment = True
            index += 2
            continue
        if source.startswith("tools.", index):
            start = index + len("tools.")
            end = start
            while end < len(source) and (source[end].isalnum() or source[end] == "_"):
                end += 1
            cursor = end
            while cursor < len(source) and source[cursor].isspace():
                cursor += 1
            if end > start and cursor < len(source) and source[cursor] == "(":
                calls.append(source[start:end])
            index = max(end, index + 1)
            continue
        index += 1
    return calls


def normalized_tool(call_name: Any, call_input: Any) -> tuple[str, str]:
    name = str(call_name or "unknown")
    source = call_input if isinstance(call_input, str) else json.dumps(call_input, sort_keys=True)
    calls = nested_tool_calls(source)
    unique_calls = list(dict.fromkeys(calls))
    nested = unique_calls[0] if len(unique_calls) == 1 else "multi_tool" if calls else name
    aliases = {
        "view_image": "view_file",
        "write_stdin": "exec_command",
        "web__run": "web_request",
    }
    tool_name = aliases.get(nested, nested)
    operation = ADAPTER.tool_operation_kind(tool_name, {"command": source})
    return tool_name, operation


def output_outcome(output: Any) -> str:
    if isinstance(output, dict):
        if output.get("is_error") is True or output.get("error") is not None:
            return "failed"
    if isinstance(output, list):
        for item in output:
            if isinstance(item, dict) and item.get("type") == "error":
                return "failed"
    return "completed"


def build_observation(
    session_digest: str,
    pending: dict[str, Any],
    output: Any,
    observed_at: str,
) -> dict[str, Any]:
    output_digest = sha256_value(output)
    event_digest = sha256_value(
        {
            "session_id_sha256": session_digest,
            "call_id_sha256": pending["call_id_sha256"],
            "input_sha256": pending["input_sha256"],
            "output_sha256": output_digest,
            "tool_name": pending["tool_name"],
            "operation": pending["operation"],
        }
    )
    run_id = event_digest[:24]
    outcome = output_outcome(output)
    before, action, after = ADAPTER.lifecycle_transition(
        ADAPTER.tool_family(pending["tool_name"]),
        pending["operation"],
        run_id,
        outcome,
    )
    provenance = {
        "source_session_id_sha256": session_digest,
        "source_event_id_sha256": event_digest,
        "call_input_sha256": pending["input_sha256"],
        "call_output_sha256": output_digest,
    }
    receipt = ADAPTER.evidence_receipt(
        before,
        action,
        after,
        provenance,
        observed_at,
        EVIDENCE_VERIFIER,
    )
    return {
        "schema": OBSERVATION_SCHEMA,
        "trace_id": f"codex-session-{event_digest}",
        "observed_at": observed_at,
        "before": before,
        "action": action,
        "after": after,
        "evidence": {
            "source": EVIDENCE_SOURCE,
            "verifier": EVIDENCE_VERIFIER,
            "receipt_schema": ADAPTER.EVIDENCE_RECEIPT_SCHEMA,
            "receipt_sha256": receipt,
        },
        "provenance": provenance,
        "usage": {"input_tokens": 0, "output_tokens": 0, "total_tokens": 0},
    }


def empty_state() -> dict[str, Any]:
    return {
        "schema": STATE_SCHEMA,
        "files": {},
        "outbox": [],
        "delivered_event_ids": [],
        "provider_events": {"offset": 0, "by_turn": {}},
        "outcome_files": {},
        "pending_terminals": {},
        "outcome_outbox": [],
        "delivered_outcome_ids": [],
        "response_relation_outbox": [],
        "delivered_response_relation_ids": [],
        "response_shadow_outbox": [],
        "delivered_response_shadow_ids": [],
        "relation_frame_outbox": [],
        "delivered_relation_frame_ids": [],
        "generic_relation_files": {},
        "generic_relation_classifier_version": GENERIC_RELATION_CLASSIFIER_VERSION,
        "generic_relation_file_cursor": 0,
        "wait_relation_files": {},
        "wait_relation_file_cursor": 0,
        "wait_relation_classifier_version": WAIT_RELATION_CLASSIFIER_VERSION,
    }


def load_state(path: Path) -> dict[str, Any]:
    try:
        state = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return empty_state()
    if not isinstance(state, dict) or state.get("schema") != STATE_SCHEMA:
        return empty_state()
    state.setdefault("files", {})
    state.setdefault("outbox", [])
    state.setdefault("delivered_event_ids", [])
    state.setdefault("provider_events", {"offset": 0, "by_turn": {}})
    state.setdefault("outcome_files", {})
    state.setdefault("pending_terminals", {})
    state.setdefault("outcome_outbox", [])
    state.setdefault("delivered_outcome_ids", [])
    state.setdefault("response_relation_outbox", [])
    state.setdefault("delivered_response_relation_ids", [])
    state.setdefault("response_shadow_outbox", [])
    state.setdefault("delivered_response_shadow_ids", [])
    state.setdefault("relation_frame_outbox", [])
    state.setdefault("delivered_relation_frame_ids", [])
    state.setdefault("generic_relation_files", {})
    state["generic_relation_file_cursor"] = max(
        0, int(state.get("generic_relation_file_cursor") or 0)
    )
    if state.get("generic_relation_classifier_version") != GENERIC_RELATION_CLASSIFIER_VERSION:
        state["generic_relation_files"] = {}
        state["generic_relation_classifier_version"] = GENERIC_RELATION_CLASSIFIER_VERSION
    for key, file_state in list(state["generic_relation_files"].items()):
        state["generic_relation_files"][key] = RESPONSE_RELATIONS.normalize_file_state(file_state)
    state.setdefault("wait_relation_files", {})
    state["wait_relation_file_cursor"] = max(
        0, int(state.get("wait_relation_file_cursor") or 0)
    )
    if state.get("wait_relation_classifier_version") != WAIT_RELATION_CLASSIFIER_VERSION:
        for source_path, file_state in state["wait_relation_files"].items():
            if isinstance(file_state, dict):
                backfill_offset = 0
                try:
                    source_size = Path(source_path).stat().st_size
                    backfill_offset = max(
                        0, source_size - DEFAULT_WAIT_RELATION_BACKFILL_BYTES
                    )
                except OSError:
                    pass
                file_state.update(
                    {
                        "offset": backfill_offset,
                        "pending": None,
                        "calls": {},
                        "active_pending_handles": [],
                        "wait_calls": {},
                        "turn": empty_wait_turn_state(),
                    }
                )
        state["wait_relation_classifier_version"] = WAIT_RELATION_CLASSIFIER_VERSION
    for file_state in state["wait_relation_files"].values():
        if not isinstance(file_state, dict):
            continue
        turn = file_state.get("turn")
        if not isinstance(turn, dict):
            file_state["turn"] = empty_wait_turn_state()
        else:
            normalized_turn = empty_wait_turn_state()
            for key in ("calls", "outputs", "pending_outputs", "messages"):
                value = turn.get(key)
                normalized_turn[key] = value if isinstance(value, int) and value >= 0 else 0
            shapes = turn.get("call_shapes")
            normalized_turn["call_shapes"] = (
                [str(value)[:80] for value in shapes[-64:]] if isinstance(shapes, list) else []
            )
            file_state["turn"] = normalized_turn
        calls = file_state.get("calls")
        if not isinstance(calls, dict):
            file_state["calls"] = {}
            continue
        compact = [
            (key, value)
            for key, value in calls.items()
            if isinstance(value, dict)
            and isinstance(value.get("surface"), str)
            and isinstance(value.get("command_sha256"), str)
            and value.get("call_shape") in {"function_call", "custom_tool_call"}
        ][-MAX_PENDING_WAIT_CALLS_PER_FILE:]
        file_state["calls"] = dict(compact)
    return state


def initial_generic_relation_file_state(
    path: Path, backfill_bytes: int
) -> dict[str, Any]:
    file_state = RESPONSE_RELATIONS.empty_file_state()
    file_state["session_id_sha256"] = sha256_text(str(path))
    size = path.stat().st_size
    if backfill_bytes <= 0 or size <= backfill_bytes:
        return file_state
    with path.open("rb") as handle:
        handle.seek(size - backfill_bytes)
        handle.readline()
        file_state["offset"] = handle.tell()
    return file_state


def harvest_generic_response_relations(
    path: Path,
    state: dict[str, Any],
    max_events: int,
    backfill_bytes: int = DEFAULT_GENERIC_RELATION_BACKFILL_BYTES,
) -> int:
    file_state = state["generic_relation_files"].get(str(path))
    if not isinstance(file_state, dict):
        file_state = initial_generic_relation_file_state(path, backfill_bytes)
        state["generic_relation_files"][str(path)] = file_state
    frames = RESPONSE_RELATIONS.harvest_relation_frames(path, file_state, max_events)
    known = set(state.get("delivered_relation_frame_ids", []))
    known.update(
        frame.get("frame_id_sha256")
        for frame in state["relation_frame_outbox"]
        if isinstance(frame, dict)
    )
    added = 0
    for frame in frames:
        frame_id = frame.get("frame_id_sha256")
        if isinstance(frame_id, str) and frame_id not in known:
            state["relation_frame_outbox"].append(frame)
            known.add(frame_id)
            added += 1
    return added


def initial_wait_relation_file_state(path: Path, backfill_bytes: int) -> dict[str, Any]:
    offset = 0
    size = path.stat().st_size
    if backfill_bytes > 0 and size > backfill_bytes:
        with path.open("rb") as handle:
            handle.seek(size - backfill_bytes)
            handle.readline()
            offset = handle.tell()
    return {
        "offset": offset,
        "session_id_sha256": sha256_text(str(path)),
        "pending": None,
        "calls": {},
        "turn": empty_wait_turn_state(),
    }


def remember_wait_relation_call(
    file_state: dict[str, Any], payload: dict[str, Any], item_type: str
) -> None:
    call_id = str(payload.get("call_id") or "")
    source = payload.get("input") if item_type == "custom_tool_call" else payload.get("arguments")
    if not call_id or not isinstance(source, str):
        return
    file_state["turn"]["calls"] += 1
    file_state["turn"]["call_shapes"].append(item_type[:80])
    file_state["turn"]["call_shapes"] = file_state["turn"]["call_shapes"][-64:]
    file_state["calls"][sha256_text(call_id)] = {
        "surface": build_or_test_surface(source),
        "command_sha256": sha256_text(source),
        "call_shape": item_type,
    }
    while len(file_state["calls"]) > MAX_PENDING_WAIT_CALLS_PER_FILE:
        file_state["calls"].pop(next(iter(file_state["calls"])))


def harvest_wait_relations(
    path: Path,
    state: dict[str, Any],
    max_events: int,
    max_lines: int | None = None,
    backfill_bytes: int = DEFAULT_WAIT_RELATION_BACKFILL_BYTES,
) -> int:
    file_state = state["wait_relation_files"].setdefault(
        str(path),
        initial_wait_relation_file_state(path, backfill_bytes),
    )
    file_state.setdefault("calls", {})
    file_state.setdefault("turn", empty_wait_turn_state())
    size = path.stat().st_size
    if int(file_state.get("offset") or 0) > size:
        file_state.update(
            {
                "offset": 0,
                "session_id_sha256": "",
                "pending": None,
                "turn": empty_wait_turn_state(),
            }
        )
    harvested = 0
    line_budget = max_lines if max_lines is not None else max(256, min(4_096, max_events * 64))
    lines_read = 0
    active_handles = set(file_state.setdefault("active_pending_handles", []))
    wait_calls = file_state.setdefault("wait_calls", {})
    with path.open("rb") as handle:
        handle.seek(int(file_state.get("offset") or 0))
        while harvested < max_events and lines_read < max(1, line_budget):
            line_start = handle.tell()
            line = handle.readline()
            if not line:
                break
            if not line.endswith(b"\n"):
                handle.seek(line_start)
                break
            file_state["offset"] = handle.tell()
            lines_read += 1
            try:
                row = json.loads(line)
            except (UnicodeDecodeError, json.JSONDecodeError):
                continue
            payload = row.get("payload")
            if not isinstance(payload, dict):
                continue
            if row.get("type") == "turn_context" or (
                row.get("type") == "event_msg"
                and payload.get("type") in {"context_compacted", "user_message"}
            ):
                file_state["pending"] = None
                file_state["turn"] = empty_wait_turn_state()
                active_handles.clear()
                wait_calls.clear()
                file_state["active_pending_handles"] = []
                continue
            if row.get("type") == "session_meta":
                session_id = payload.get("id")
                if isinstance(session_id, str) and session_id:
                    file_state["session_id_sha256"] = sha256_text(session_id)
                continue
            if row.get("type") != "response_item":
                continue
            item_type = payload.get("type")
            if item_type == "message":
                if payload.get("role") == "user":
                    file_state["pending"] = None
                    file_state["turn"] = empty_wait_turn_state()
                    active_handles.clear()
                    wait_calls.clear()
                    file_state["active_pending_handles"] = []
                elif payload.get("role") == "assistant":
                    file_state["turn"]["messages"] += 1
                continue
            if item_type in {"custom_tool_call", "function_call"} and file_state.get("pending") is None:
                remember_wait_relation_call(file_state, payload, item_type)
                continue
            if item_type in {"custom_tool_call_output", "function_call_output"}:
                file_state["turn"]["outputs"] += 1
                output = payload.get("output")
                call_key = sha256_text(str(payload.get("call_id") or ""))
                completed_wait = wait_calls.pop(call_key, None)
                command = file_state["calls"].pop(call_key, "")
                completed_wait_handle = (
                    completed_wait.get("handle")
                    if isinstance(completed_wait, dict)
                    else completed_wait
                )
                if not command and isinstance(completed_wait, dict):
                    command = completed_wait.get("command", "")
                if isinstance(completed_wait_handle, str):
                    active_handles.discard(completed_wait_handle)
                    file_state["active_pending_handles"] = sorted(active_handles)
                if isinstance(output, str) and output.startswith("Script running with cell ID "):
                    command_surface = command.get("surface", "") if isinstance(command, dict) else command
                    if not command_surface:
                        continue
                    cell_id = output.removeprefix("Script running with cell ID ").split()[0]
                    active_handles.add(sha256_text(cell_id))
                    file_state["active_pending_handles"] = sorted(active_handles)
                    file_state["turn"]["pending_outputs"] += 1
                    file_state["pending"] = {
                        "cell_id": cell_id,
                        "surface": command_surface,
                        "command_sha256": command.get("command_sha256", "")
                        if isinstance(command, dict)
                        else sha256_text(command_surface),
                        "call_shape": command.get("call_shape", "")
                        if isinstance(command, dict)
                        else "",
                        "output_sha256": sha256_text(output),
                        "context_atoms": wait_turn_context_atoms(file_state["turn"])
                        + [{
                            "kind": "cardinality",
                            "role": "active_pending_handle_count_band",
                            "count": count_band(len(active_handles)),
                        }],
                    }
                continue
            if item_type not in {"custom_tool_call", "function_call"}:
                continue
            pending = file_state.get("pending")
            if not isinstance(pending, dict):
                continue
            action_function, action_arguments = bounded_function_action(payload)
            action_cell_id = ""
            scheduling_atoms = None
            if action_function == "wait" and action_arguments is not None:
                candidate_cell_id = action_arguments.get("cell_id")
                scheduling_atoms = bounded_wait_scheduling_atoms(action_arguments)
                if isinstance(candidate_cell_id, str) and scheduling_atoms is not None:
                    action_cell_id = candidate_cell_id
                    call_key = sha256_text(str(payload.get("call_id") or ""))
                    wait_calls[call_key] = {
                        "handle": sha256_text(action_cell_id),
                        "command": {
                            "surface": pending["surface"],
                            "command_sha256": pending["command_sha256"],
                            "call_shape": item_type,
                        },
                    }
            verifier_ok = bool(action_cell_id) and action_cell_id == pending["cell_id"]
            session_digest = str(file_state.get("session_id_sha256") or "")
            if not valid_sha256(session_digest):
                file_state["pending"] = None
                continue
            specific_wait = is_specific_wait_surface(pending["surface"])
            guard_schema = (
                "wait_long_running_batch_guard.v5"
                if specific_wait
                else "wait_yielded_surface_guard.v3"
            )
            relation_id = sha256_value(
                {
                    "schema": RESPONSE_RELATION_SCHEMA,
                    "session": session_digest,
                    "output": pending["output_sha256"],
                    "action": sha256_value(payload),
                    "guard_schema": guard_schema,
                }
            )
            relation = {
                "schema": RESPONSE_RELATION_SCHEMA,
                "relation_id": f"response-relation-{relation_id}",
                "observed_at": str(row.get("timestamp") or ""),
                "relation": "yielded_cell_to_wait_function_call"
                if specific_wait
                else "yielded_cell_to_surface_wait_function_call",
                "program_hint": {
                    "op": "wait_on_yielded_cell"
                    if specific_wait
                    else "wait_on_yielded_surfaces",
                    "prefix": "" if specific_wait else pending["surface"],
                },
                "source_session_id_sha256": session_digest,
                "source_turn_id_sha256": sha256_text(str(payload.get("call_id") or relation_id)),
                "surface_id_sha256": pending["command_sha256"],
                "verifier_ok": verifier_ok,
                "guard_schema": guard_schema,
                "request_text_sha256": pending["output_sha256"],
                "outcome_sha256": sha256_value(payload),
                "raw_request_stored": False,
                "raw_outcome_stored": False,
            }
            observation_value_sha256 = sha256_text(pending["cell_id"])
            atoms = [
                {"kind": "tool_kind", "value": pending["surface"]},
                {"kind": "observation_call_shape", "value": pending["call_shape"]},
                {"kind": "completion_state", "value": "pending"},
                {"kind": "response_shape", "value": item_type},
                {
                    "kind": "typed_slot",
                    "slot_id": 1,
                    "value_type": "identifier",
                    "source": "observation",
                    "value_sha256": observation_value_sha256,
                },
                {"kind": "unique_slot", "slot_id": 1},
                {
                    "kind": "observation_selector",
                    "slot_id": 1,
                    "selector": {
                        "kind": "content_line_prefix",
                        "prefix": "Script running with cell ID ",
                        "value_type": "identifier",
                    },
                },
            ]
            atoms.extend(bounded_wait_context_atoms(pending.get("context_atoms")))
            if action_function:
                atoms.append({"kind": "action_function", "value": action_function})
            if action_cell_id:
                action_value_sha256 = sha256_text(action_cell_id)
                atoms.append(
                    {
                        "kind": "typed_slot",
                        "slot_id": 2,
                        "value_type": "identifier",
                        "source": "action",
                        "value_sha256": action_value_sha256,
                    }
                )
                atoms.append({"kind": "action_role_argument", "name": "cell_id", "slot_id": 2})
                atoms.extend(scheduling_atoms or [])
                if action_value_sha256 == observation_value_sha256:
                    atoms.append({"kind": "slot_equality", "left_slot": 1, "right_slot": 2})
            frame_id = sha256_value(
                    {
                        "schema": RELATION_FRAME_SCHEMA,
                        "event": relation_id,
                        "atoms": atoms,
                        "extractor": RELATION_EXTRACTOR_VERSION,
                    }
            )
            frame = {
                    "schema": RELATION_FRAME_SCHEMA,
                    "frame_id_sha256": frame_id,
                    "event_id_sha256": relation_id,
                    "client_intent_id_sha256": sha256_text(
                        f"{session_digest}:{payload.get('call_id') or relation_id}"
                    ),
                    "session_id_sha256": session_digest,
                    "observed_at_unix_nanos": iso_to_unix_nanos(str(row.get("timestamp") or "")),
                    "extractor_version": RELATION_EXTRACTOR_VERSION,
                    "verifier_label": verifier_ok,
                    "atoms": atoms,
                    "evidence_ref_sha256": relation["outcome_sha256"],
            }
            known_frames = set(state.get("delivered_relation_frame_ids", []))
            known_frames.update(
                    item.get("frame_id_sha256")
                    for item in state["relation_frame_outbox"]
                    if isinstance(item, dict)
            )
            if frame_id not in known_frames:
                state["relation_frame_outbox"].append(frame)
            known = set(state.get("delivered_response_relation_ids", []))
            known.update(
                item.get("relation_id")
                for item in state["response_relation_outbox"]
                if isinstance(item, dict)
            )
            if relation["relation_id"] not in known:
                state["response_relation_outbox"].append(relation)
                harvested += 1
            file_state["pending"] = None
            if not (item_type == "function_call" and payload.get("name") == "wait"):
                remember_wait_relation_call(file_state, payload, item_type)
    return harvested


def build_or_test_command(source: str) -> bool:
    return is_specific_wait_surface(build_or_test_surface(source))


def is_specific_wait_surface(surface: str) -> bool:
    return surface.startswith(("cargo_", "pytest", "unittest", "npm_", "pnpm_")) or surface in {
        "graphify_update",
        "rust_action_memory",
        "find_xargs_batch",
        "apt_get",
    }


def build_or_test_surface(source: str) -> str:
    lower = source.casefold()
    if "cargo " in lower:
        for operation in ("clippy", "test", "build", "check", "bench"):
            if operation in lower:
                return f"cargo_{operation}"
    for tool in ("pytest", "unittest", "npm test", "pnpm test"):
        if tool in lower:
            return tool.replace(" ", "_")
    if "graphify update" in lower:
        return "graphify_update"
    if "rust-action-memory" in lower:
        return "rust_action_memory"
    if "find " in lower and "xargs" in lower:
        return "find_xargs_batch"
    if "apt-get " in lower:
        return "apt_get"
    if not lower.strip():
        return ""
    if "nando-live-transition-gate" in lower:
        return "live_transition_gate"
    if "systemctl " in lower or "journalctl " in lower:
        return "service_observation"
    if "curl " in lower or "wget " in lower or "ping " in lower:
        return "network_observation"
    if "git " in lower:
        return "version_control"
    if "python" in lower:
        return "python_batch"
    if "nando-" in lower:
        return "nando_ops"
    if "nginx" in lower:
        return "nginx_ops"
    if "sleep " in lower or "timeout " in lower:
        return "timed_wait"
    if any(term in lower for term in ("install ", "mkdir ", " cp ", " mv ", "chmod ", "chown ")):
        return "filesystem_mutation"
    if any(term in lower for term in ("tar ", "gzip ", "zstd ", "xz ")):
        return "archive_batch"
    if "sha256sum" in lower or "b2sum" in lower:
        return "checksum_batch"
    if "ps " in lower or "ss " in lower or "lsof " in lower:
        return "process_observation"
    if any(tool in lower for tool in ("rg ", "find ", "sed ", "jq ", "ls ")):
        return "filesystem_observation"
    if "&&" in lower or ";" in lower or "set -" in lower:
        return "shell_batch"
    return "generic_long_command"


def build_response_relation(
    request_text: str,
    terminal_text: str,
    session_digest: str,
    turn_digest: str,
    observed_at: str,
) -> dict[str, Any] | None:
    if not request_text or not terminal_text or not request_text.endswith(terminal_text):
        return None
    prefix = request_text[: -len(terminal_text)]
    if (
        not prefix
        or not prefix.isascii()
        or len(prefix.encode("utf-8")) > 256
        or "\n" in prefix
        or "\r" in prefix
    ):
        return None
    normalized_prefix = prefix.strip().casefold()
    if not normalized_prefix:
        return None
    relation_id = sha256_value(
        {
            "schema": RESPONSE_RELATION_SCHEMA,
            "session": session_digest,
            "turn": turn_digest,
            "request": sha256_text(request_text),
            "outcome": sha256_text(terminal_text),
        }
    )
    return {
        "schema": RESPONSE_RELATION_SCHEMA,
        "relation_id": f"response-relation-{relation_id}",
        "observed_at": observed_at,
        "relation": "outcome_equals_request_suffix",
        "program_hint": {"op": "copy_after_prefix", "prefix": normalized_prefix},
        "source_session_id_sha256": session_digest,
        "source_turn_id_sha256": turn_digest,
        "surface_id_sha256": sha256_text(normalized_prefix),
        "request_text_sha256": sha256_text(request_text),
        "outcome_sha256": sha256_text(terminal_text),
        "raw_request_stored": False,
        "raw_outcome_stored": False,
    }


def load_response_shadow_candidates(path: Path) -> list[dict[str, Any]]:
    try:
        registry = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return []
    candidates = []
    for package in registry.get("packages", []):
        if not isinstance(package, dict) or package.get("state") != "quarantine":
            continue
        operation = package.get("program", {}).get("operation", {})
        prefixes = operation.get("prefixes") if operation.get("op") == "copy_after_prefix" else None
        if not isinstance(prefixes, list) or not prefixes:
            continue
        clean = [prefix for prefix in prefixes if isinstance(prefix, str) and prefix and prefix.isascii()]
        if clean:
            candidates.append({"package_id": package.get("package_id"), "prefixes": clean})
    return candidates


def build_response_shadows(
    request_text: str,
    terminal_text: str,
    session_digest: str,
    turn_digest: str,
    observed_at: str,
    candidates: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    request = request_text.lstrip()
    lowered = request.lower()
    rows = []
    for candidate in candidates:
        matches = [prefix for prefix in candidate["prefixes"] if lowered.startswith(prefix.lower())]
        if len(matches) != 1:
            continue
        prefix = matches[0]
        predicted = request[len(prefix) :].strip()
        package_id = candidate.get("package_id")
        if not package_id or not predicted:
            continue
        shadow_id = sha256_value(
            {
                "schema": RESPONSE_SHADOW_SCHEMA,
                "package_id": package_id,
                "session": session_digest,
                "turn": turn_digest,
                "request": sha256_text(request_text),
                "outcome": sha256_text(terminal_text),
            }
        )
        rows.append(
            {
                "schema": RESPONSE_SHADOW_SCHEMA,
                "shadow_id": f"response-shadow-{shadow_id}",
                "package_id": package_id,
                "observed_at": observed_at,
                "source_session_id_sha256": session_digest,
                "source_turn_id_sha256": turn_digest,
                "surface_id_sha256": sha256_text(prefix.lower()),
                "matched_guard": True,
                "verifier_ok": predicted == terminal_text,
                "request_text_sha256": sha256_text(request_text),
                "outcome_sha256": sha256_text(terminal_text),
                "predicted_outcome_sha256": sha256_text(predicted),
                "raw_text_stored": False,
            }
        )
    return rows


def save_state(path: Path, state: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        mode="w",
        encoding="utf-8",
        dir=path.parent,
        prefix=f".{path.name}.",
        delete=False,
    ) as handle:
        json.dump(state, handle, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        handle.write("\n")
        temporary = Path(handle.name)
    os.chmod(temporary, 0o600)
    temporary.replace(path)


def harvest_file(path: Path, state: dict[str, Any], max_events: int) -> int:
    files = state["files"]
    file_state = files.setdefault(
        str(path),
        {"offset": 0, "session_id_sha256": "", "pending": {}},
    )
    size = path.stat().st_size
    if int(file_state.get("offset") or 0) > size:
        file_state.update({"offset": 0, "session_id_sha256": "", "pending": {}})
    delivered = set(state.get("delivered_event_ids", []))
    outbox_ids = {row.get("trace_id") for row in state["outbox"] if isinstance(row, dict)}
    harvested = 0
    with path.open("rb") as handle:
        handle.seek(int(file_state.get("offset") or 0))
        while harvested < max_events and len(state["outbox"]) < MAX_OUTBOX:
            line_start = handle.tell()
            line = handle.readline()
            if not line:
                break
            if not line.endswith(b"\n"):
                handle.seek(line_start)
                break
            file_state["offset"] = handle.tell()
            try:
                row = json.loads(line)
            except (UnicodeDecodeError, json.JSONDecodeError):
                continue
            if row.get("type") == "session_meta":
                session_id = row.get("payload", {}).get("id")
                if isinstance(session_id, str) and session_id:
                    file_state["session_id_sha256"] = sha256_text(session_id)
                continue
            if row.get("type") != "response_item":
                continue
            payload = row.get("payload")
            if not isinstance(payload, dict):
                continue
            call_id = payload.get("call_id")
            if not isinstance(call_id, str) or not call_id:
                continue
            call_key = sha256_text(call_id)
            if payload.get("type") == "custom_tool_call":
                tool_name, operation = normalized_tool(payload.get("name"), payload.get("input"))
                file_state["pending"][call_key] = {
                    "call_id_sha256": call_key,
                    "input_sha256": sha256_value(payload.get("input")),
                    "tool_name": tool_name,
                    "operation": operation,
                }
                continue
            if payload.get("type") != "custom_tool_call_output":
                continue
            pending = file_state["pending"].pop(call_key, None)
            session_digest = str(file_state.get("session_id_sha256") or "")
            if not isinstance(pending, dict) or len(session_digest) != 64:
                continue
            observation = build_observation(
                session_digest,
                pending,
                payload.get("output"),
                str(row.get("timestamp") or ""),
            )
            trace_id = observation["trace_id"]
            if trace_id not in delivered and trace_id not in outbox_ids:
                state["outbox"].append(observation)
                outbox_ids.add(trace_id)
                harvested += 1
    return harvested


def valid_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def message_text(payload: dict[str, Any]) -> str:
    content = payload.get("content")
    if isinstance(content, str):
        return content
    if not isinstance(content, list):
        return ""
    parts = []
    for item in content:
        if not isinstance(item, dict):
            continue
        text = item.get("text")
        if isinstance(text, str) and text:
            parts.append(text)
    return "\n".join(parts)


def provider_turn_key(session_digest: str, turn_digest: str) -> str:
    return f"{session_digest}:{turn_digest}"


def bounded_provider_request(row: dict[str, Any]) -> dict[str, Any] | None:
    shape = row.get("request_shape")
    if not isinstance(shape, dict) or shape.get("raw_text_stored") is not False:
        return None
    identity = shape.get("client_identity_sha256")
    if not isinstance(identity, dict):
        return None
    session_digest = identity.get("session_id")
    turn_digest = identity.get("turn_id")
    request_digest = row.get("request_sha256")
    request_text_digest = shape.get("request_text_sha256")
    if not all(
        valid_sha256(value)
        for value in (
            session_digest,
            turn_digest,
            request_digest,
            request_text_digest,
        )
    ):
        return None
    return {
        "timestamp_unix": int(row.get("timestamp_unix") or 0),
        "request_sha256": request_digest,
        "request_text_sha256": request_text_digest,
        "request_text_bytes": int(shape.get("request_text_bytes") or 0),
        "shape_sha256": sha256_value(shape),
        "shape": shape,
    }


def harvest_provider_events(path: Path, state: dict[str, Any], max_events: int) -> int:
    provider_state = state["provider_events"]
    provider_state.setdefault("offset", 0)
    provider_state.setdefault("by_turn", {})
    if not path.exists():
        return 0
    size = path.stat().st_size
    if int(provider_state.get("offset") or 0) > size:
        provider_state["offset"] = 0
        provider_state["by_turn"] = {}
    harvested = 0
    with path.open("rb") as handle:
        handle.seek(int(provider_state.get("offset") or 0))
        while harvested < max_events:
            line_start = handle.tell()
            line = handle.readline()
            if not line:
                break
            if not line.endswith(b"\n"):
                handle.seek(line_start)
                break
            provider_state["offset"] = handle.tell()
            try:
                row = json.loads(line)
            except (UnicodeDecodeError, json.JSONDecodeError):
                continue
            if row.get("event") != "bridge_request":
                continue
            request = bounded_provider_request(row)
            if request is None:
                continue
            identity = row["request_shape"]["client_identity_sha256"]
            key = provider_turn_key(identity["session_id"], identity["turn_id"])
            requests = provider_state["by_turn"].setdefault(key, [])
            if not any(
                current.get("request_sha256") == request["request_sha256"]
                for current in requests
                if isinstance(current, dict)
            ):
                requests.append(request)
                del requests[:-MAX_PROVIDER_REQUESTS_PER_TURN]
                harvested += 1
    by_turn = provider_state["by_turn"]
    if len(by_turn) > MAX_PROVIDER_TURNS:
        for key in list(by_turn)[: len(by_turn) - MAX_PROVIDER_TURNS]:
            del by_turn[key]
    return harvested


def harvest_terminal_outcomes(
    path: Path,
    state: dict[str, Any],
    max_events: int,
    shadow_candidates: list[dict[str, Any]] | None = None,
) -> int:
    file_state = state["outcome_files"].setdefault(
        str(path),
        {
            "offset": 0,
            "session_id_sha256": "",
            "turn_id_sha256": "",
            "request_text_sha256": "",
            "request_text_bytes": 0,
        },
    )
    size = path.stat().st_size
    if int(file_state.get("offset") or 0) > size:
        file_state.update(
            {
                "offset": 0,
                "session_id_sha256": "",
                "turn_id_sha256": "",
                "request_text_sha256": "",
                "request_text_bytes": 0,
            }
        )
    harvested = 0
    request_text = ""
    turn_start_offset = 0
    with path.open("rb") as handle:
        handle.seek(int(file_state.get("offset") or 0))
        while harvested < max_events:
            line_start = handle.tell()
            line = handle.readline()
            if not line:
                break
            if not line.endswith(b"\n"):
                handle.seek(line_start)
                break
            file_state["offset"] = handle.tell()
            try:
                row = json.loads(line)
            except (UnicodeDecodeError, json.JSONDecodeError):
                continue
            payload = row.get("payload")
            if not isinstance(payload, dict):
                continue
            if row.get("type") == "session_meta":
                session_id = payload.get("id")
                if isinstance(session_id, str) and session_id:
                    file_state["session_id_sha256"] = sha256_text(session_id)
                continue
            if row.get("type") == "turn_context":
                turn_id = payload.get("turn_id")
                if isinstance(turn_id, str) and turn_id:
                    file_state["turn_id_sha256"] = sha256_text(turn_id)
                    file_state["request_text_sha256"] = ""
                    file_state["request_text_bytes"] = 0
                    request_text = ""
                    turn_start_offset = line_start
                continue
            if (
                row.get("type") == "response_item"
                and payload.get("type") == "message"
                and payload.get("role") == "user"
            ):
                text = message_text(payload)
                if text:
                    request_text = text
                    file_state["request_text_sha256"] = sha256_text(text)
                    file_state["request_text_bytes"] = len(text.encode("utf-8"))
                continue
            if row.get("type") != "event_msg" or payload.get("type") != "task_complete":
                continue
            terminal_text = payload.get("last_agent_message")
            session_digest = str(file_state.get("session_id_sha256") or "")
            turn_digest = str(file_state.get("turn_id_sha256") or "")
            request_text_digest = str(file_state.get("request_text_sha256") or "")
            if not isinstance(terminal_text, str) or not terminal_text:
                continue
            if not all(
                valid_sha256(value)
                for value in (session_digest, turn_digest, request_text_digest)
            ):
                continue
            key = provider_turn_key(session_digest, turn_digest)
            relation = build_response_relation(
                request_text, terminal_text, session_digest, turn_digest, str(row.get("timestamp") or "")
            )
            if relation is not None:
                known = set(state.get("delivered_response_relation_ids", []))
                known.update(
                    item.get("relation_id")
                    for item in state["response_relation_outbox"]
                    if isinstance(item, dict)
                )
                if relation["relation_id"] not in known:
                    state["response_relation_outbox"].append(relation)
            shadows = build_response_shadows(
                request_text,
                terminal_text,
                session_digest,
                turn_digest,
                str(row.get("timestamp") or ""),
                shadow_candidates or [],
            )
            known_shadows = set(state.get("delivered_response_shadow_ids", []))
            known_shadows.update(
                item.get("shadow_id")
                for item in state["response_shadow_outbox"]
                if isinstance(item, dict)
            )
            state["response_shadow_outbox"].extend(
                row for row in shadows if row["shadow_id"] not in known_shadows
            )
            state["pending_terminals"][key] = {
                "observed_at": str(row.get("timestamp") or ""),
                "session_id_sha256": session_digest,
                "turn_id_sha256": turn_digest,
                "request_text_sha256": request_text_digest,
                "request_text_bytes": int(file_state.get("request_text_bytes") or 0),
                "outcome_kind": "assistant_text",
                "outcome_sha256": sha256_text(terminal_text),
                "outcome_bytes": len(terminal_text.encode("utf-8")),
                "source_rollout_sha256": sha256_text(str(path)),
            }
            harvested += 1
            request_text = ""
            turn_start_offset = 0
    if request_text and turn_start_offset:
        file_state["offset"] = turn_start_offset
    pending = state["pending_terminals"]
    if len(pending) > MAX_PENDING_TERMINALS:
        for key in list(pending)[: len(pending) - MAX_PENDING_TERMINALS]:
            del pending[key]
    return harvested


def build_provider_outcome(
    key: str, terminal: dict[str, Any], requests: list[dict[str, Any]]
) -> dict[str, Any]:
    request_digests = [request["request_sha256"] for request in requests]
    client_intent_id = sha256_value(
        {
            "schema": "nando.client-intent.v1",
            "session_id_sha256": terminal["session_id_sha256"],
            "turn_id_sha256": terminal["turn_id_sha256"],
        }
    )
    receipt_material = {
        "schema": PROVIDER_OUTCOME_SCHEMA,
        "client_intent_id": client_intent_id,
        "request_sha256": request_digests,
        "request_text_sha256": terminal["request_text_sha256"],
        "outcome_sha256": terminal["outcome_sha256"],
        "observed_at": terminal["observed_at"],
    }
    receipt = sha256_value(receipt_material)
    return {
        "schema": PROVIDER_OUTCOME_SCHEMA,
        "outcome_id": f"provider-outcome-{receipt}",
        "observed_at": terminal["observed_at"],
        "client_intent_id": client_intent_id,
        "source_session_id_sha256": terminal["session_id_sha256"],
        "source_turn_id_sha256": terminal["turn_id_sha256"],
        "source_rollout_sha256": terminal["source_rollout_sha256"],
        "request_text_sha256": terminal["request_text_sha256"],
        "request_text_bytes": terminal["request_text_bytes"],
        "provider_request_count": len(requests),
        "provider_requests": requests,
        "outcome": {
            "kind": terminal["outcome_kind"],
            "sha256": terminal["outcome_sha256"],
            "bytes": terminal["outcome_bytes"],
        },
        "evidence": {
            "source": "codex_session_terminal",
            "verifier": "provider_session_turn_hash_join_v1",
            "receipt_sha256": receipt,
        },
        "raw_text_stored": False,
        "join_key_sha256": sha256_text(key),
    }


def reconcile_provider_outcomes(state: dict[str, Any], max_events: int) -> int:
    delivered = set(state.get("delivered_outcome_ids", []))
    queued = {
        row.get("outcome_id")
        for row in state.get("outcome_outbox", [])
        if isinstance(row, dict)
    }
    by_turn = state["provider_events"]["by_turn"]
    count = 0
    for key in list(state["pending_terminals"]):
        if count >= max_events:
            break
        terminal = state["pending_terminals"][key]
        requests = by_turn.get(key)
        if not isinstance(requests, list) or not requests:
            continue
        if not any(
            request.get("request_text_sha256") == terminal["request_text_sha256"]
            for request in requests
            if isinstance(request, dict)
        ):
            continue
        outcome = build_provider_outcome(key, terminal, requests)
        if outcome["outcome_id"] not in delivered and outcome["outcome_id"] not in queued:
            state["outcome_outbox"].append(outcome)
            queued.add(outcome["outcome_id"])
            count += 1
        del state["pending_terminals"][key]
        by_turn.pop(key, None)
    return count


def append_provider_outcomes(
    path: Path, state: dict[str, Any], max_events: int
) -> int:
    delivered = list(state.get("delivered_outcome_ids", []))
    existing = set()
    if path.exists():
        with path.open(encoding="utf-8") as handle:
            for line in handle:
                try:
                    row = json.loads(line)
                except json.JSONDecodeError:
                    continue
                outcome_id = row.get("outcome_id")
                if isinstance(outcome_id, str):
                    existing.add(outcome_id)
    count = 0
    processed = 0
    if state["outcome_outbox"]:
        path.parent.mkdir(parents=True, exist_ok=True)
    while state["outcome_outbox"] and processed < max_events:
        outcome = state["outcome_outbox"][0]
        outcome_id = outcome["outcome_id"]
        if outcome_id not in existing:
            with path.open("a", encoding="utf-8") as handle:
                handle.write(
                    json.dumps(
                        outcome,
                        ensure_ascii=False,
                        sort_keys=True,
                        separators=(",", ":"),
                    )
                )
                handle.write("\n")
            os.chmod(path, 0o600)
            existing.add(outcome_id)
            count += 1
        delivered.append(outcome_id)
        state["outcome_outbox"].pop(0)
        processed += 1
    state["delivered_outcome_ids"] = delivered[-MAX_DELIVERED_IDS:]
    return count


def append_response_relations(path: Path, state: dict[str, Any], max_events: int) -> int:
    delivered = list(state.get("delivered_response_relation_ids", []))
    existing: set[str] = set()
    if path.exists():
        with path.open(encoding="utf-8") as handle:
            for line in handle:
                try:
                    relation_id = json.loads(line).get("relation_id")
                except json.JSONDecodeError:
                    continue
                if isinstance(relation_id, str):
                    existing.add(relation_id)
    count = 0
    processed = 0
    if state["response_relation_outbox"]:
        path.parent.mkdir(parents=True, exist_ok=True)
    while state["response_relation_outbox"] and processed < max_events:
        relation = state["response_relation_outbox"].pop(0)
        relation_id = relation["relation_id"]
        if relation_id not in existing:
            with path.open("a", encoding="utf-8") as handle:
                handle.write(json.dumps(relation, ensure_ascii=False, sort_keys=True, separators=(",", ":")))
                handle.write("\n")
            os.chmod(path, 0o600)
            existing.add(relation_id)
            count += 1
        delivered.append(relation_id)
        processed += 1
    state["delivered_response_relation_ids"] = delivered[-MAX_DELIVERED_IDS:]
    return count


def append_response_shadows(path: Path, state: dict[str, Any], max_events: int) -> int:
    delivered = list(state.get("delivered_response_shadow_ids", []))
    existing: set[str] = set()
    if path.exists():
        with path.open(encoding="utf-8") as handle:
            for line in handle:
                try:
                    shadow_id = json.loads(line).get("shadow_id")
                except json.JSONDecodeError:
                    continue
                if isinstance(shadow_id, str):
                    existing.add(shadow_id)
    count = 0
    processed = 0
    if state["response_shadow_outbox"]:
        path.parent.mkdir(parents=True, exist_ok=True)
    while state["response_shadow_outbox"] and processed < max_events:
        row = state["response_shadow_outbox"].pop(0)
        shadow_id = row["shadow_id"]
        if shadow_id not in existing:
            with path.open("a", encoding="utf-8") as handle:
                handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True, separators=(",", ":")))
                handle.write("\n")
            os.chmod(path, 0o600)
            existing.add(shadow_id)
            count += 1
        delivered.append(shadow_id)
        processed += 1
    state["delivered_response_shadow_ids"] = delivered[-MAX_DELIVERED_IDS:]
    return count


def append_relation_frames(path: Path, state: dict[str, Any], max_events: int) -> int:
    delivered = list(state.get("delivered_relation_frame_ids", []))
    existing = set(delivered)
    count = 0
    if state["relation_frame_outbox"]:
        path.parent.mkdir(parents=True, exist_ok=True)
        if path.exists():
            with path.open(encoding="utf-8") as handle:
                for line in handle:
                    try:
                        row = json.loads(line)
                    except json.JSONDecodeError:
                        continue
                    frame_id = row.get("frame_id_sha256")
                    if isinstance(frame_id, str):
                        existing.add(frame_id)
    while state["relation_frame_outbox"] and count < max_events:
        frame = state["relation_frame_outbox"].pop(0)
        frame_id = frame["frame_id_sha256"]
        if frame_id not in existing:
            with path.open("a", encoding="utf-8") as handle:
                handle.write(json.dumps(frame, ensure_ascii=False, sort_keys=True, separators=(",", ":")))
                handle.write("\n")
            os.chmod(path, 0o600)
            existing.add(frame_id)
            count += 1
        delivered.append(frame_id)
    state["delivered_relation_frame_ids"] = delivered[-MAX_DELIVERED_IDS:]
    return count


def iso_to_unix_nanos(value: str) -> int:
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return 0
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return int(parsed.timestamp() * 1_000_000_000)


def session_files(root: Path, lookback_days: int) -> list[Path]:
    cutoff = datetime.now(timezone.utc) - timedelta(days=max(0, lookback_days))
    paths = []
    for path in root.glob("**/rollout-*.jsonl"):
        try:
            modified = datetime.fromtimestamp(path.stat().st_mtime, timezone.utc)
        except OSError:
            continue
        if modified >= cutoff:
            paths.append(path)
    return sorted(paths, key=lambda path: (path.stat().st_mtime, str(path)))


def generic_relation_schedule(
    paths: list[Path], cursor: int, budget: int
) -> tuple[list[Path], int, int]:
    if not paths or budget <= 0:
        return [], 0, 0
    hot = paths[-1]
    backlog = paths[:-1]
    scheduled = [hot]
    if not backlog or budget == 1:
        return scheduled, 0, 1
    cursor %= len(backlog)
    rotated = backlog[cursor:] + backlog[:cursor]
    selected_backlog = rotated[: min(budget - 1, len(backlog))]
    scheduled.extend(selected_backlog)
    next_cursor = (cursor + len(selected_backlog)) % len(backlog)
    return scheduled, next_cursor, 1


def submit_observation(url: str, observation: dict[str, Any], timeout: float) -> bool:
    request = urllib.request.Request(
        url,
        data=canonical_bytes(observation),
        headers={"content-type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            payload = json.loads(response.read())
    except (OSError, urllib.error.HTTPError, json.JSONDecodeError):
        return False
    return response.status == 202 and payload.get("accepted") is True


def flush_outbox(state: dict[str, Any], url: str, timeout: float, max_events: int) -> int:
    delivered = list(state.get("delivered_event_ids", []))
    count = 0
    while state["outbox"] and count < max_events:
        observation = state["outbox"][0]
        if not submit_observation(url, observation, timeout):
            break
        delivered.append(observation["trace_id"])
        state["outbox"].pop(0)
        count += 1
    state["delivered_event_ids"] = delivered[-MAX_DELIVERED_IDS:]
    return count


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--sessions-root",
        type=Path,
        default=Path.home() / ".codex/sessions",
    )
    parser.add_argument(
        "--state",
        type=Path,
        default=Path.home() / ".local/state/nando-wave/codex-session-capture.json",
    )
    parser.add_argument(
        "--observe-url",
        default="http://127.0.0.1:8787/v2/transitions/observe",
    )
    parser.add_argument(
        "--provider-events",
        type=Path,
        default=Path("/var/lib/nando-wave/transition/execution-events.jsonl"),
    )
    parser.add_argument(
        "--provider-outcomes",
        type=Path,
        default=Path("/var/lib/nando-wave/transition/provider-outcomes.jsonl"),
    )
    parser.add_argument(
        "--response-relations",
        type=Path,
        default=Path("/var/lib/nando-wave/transition/response-relations.jsonl"),
    )
    parser.add_argument(
        "--response-shadow-registry",
        type=Path,
        default=Path("/var/lib/nando-wave/transition/response-registry.json"),
    )
    parser.add_argument(
        "--response-shadows",
        type=Path,
        default=Path("/var/lib/nando-wave/transition/response-shadows.jsonl"),
    )
    parser.add_argument(
        "--relation-frames",
        type=Path,
        default=Path("/var/lib/nando-wave/transition/response-relation-frames.jsonl"),
    )
    parser.add_argument("--lookback-days", type=int, default=2)
    parser.add_argument("--max-events", type=int, default=256)
    parser.add_argument("--max-events-per-file", type=int, default=64)
    parser.add_argument(
        "--generic-relation-files-per-cycle",
        type=int,
        default=DEFAULT_GENERIC_RELATION_FILES_PER_CYCLE,
    )
    parser.add_argument(
        "--wait-relation-files-per-cycle",
        type=int,
        default=DEFAULT_WAIT_RELATION_FILES_PER_CYCLE,
    )
    parser.add_argument(
        "--generic-relation-backfill-bytes",
        type=int,
        default=DEFAULT_GENERIC_RELATION_BACKFILL_BYTES,
    )
    parser.add_argument("--timeout", type=float, default=0.8)
    parser.add_argument("--dry-run", action="store_true")
    return parser.parse_args()


def acquire_capture_lock(state_path: Path) -> int | None:
    state_path.parent.mkdir(parents=True, exist_ok=True)
    lock_path = state_path.with_name(f".{state_path.name}.lock")
    descriptor = os.open(lock_path, os.O_CREAT | os.O_RDWR, 0o600)
    try:
        fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        os.close(descriptor)
        return None
    return descriptor


def main() -> int:
    args = parse_args()
    _lock_descriptor = acquire_capture_lock(args.state)
    if _lock_descriptor is None:
        print(
            json.dumps(
                {
                    "schema": "nando.codex-session-capture-run.v1",
                    "concurrent_capture_skipped": True,
                },
                separators=(",", ":"),
            )
        )
        return 0
    state = empty_state() if args.dry_run else load_state(args.state)
    remaining = max(1, args.max_events)
    harvested = 0
    provider_requests = harvest_provider_events(
        args.provider_events, state, max(1, args.max_events)
    )
    terminal_outcomes = 0
    wait_relations = 0
    generic_relations = 0
    shadow_candidates = load_response_shadow_candidates(args.response_shadow_registry)
    paths = session_files(args.sessions_root, args.lookback_days)
    for path in paths:
        count = harvest_file(path, state, min(remaining, max(1, args.max_events_per_file)))
        harvested += count
        remaining -= count
        if remaining <= 0:
            break
    wait_cursor = int(state.get("wait_relation_file_cursor") or 0)
    wait_paths, next_wait_cursor, wait_hot_files_processed = generic_relation_schedule(
        paths, wait_cursor, max(1, args.wait_relation_files_per_cycle)
    )
    for path in wait_paths:
        wait_relations += harvest_wait_relations(
            path, state, max(1, args.max_events_per_file)
        )
    if paths:
        state["wait_relation_file_cursor"] = next_wait_cursor
    generic_remaining = max(1, args.max_events)
    generic_files_processed = 0
    generic_cursor = int(state.get("generic_relation_file_cursor") or 0)
    generic_paths, next_generic_cursor, generic_hot_files_processed = (
        generic_relation_schedule(
            paths, generic_cursor, max(1, args.generic_relation_files_per_cycle)
        )
    )
    for path in generic_paths:
        count = harvest_generic_response_relations(
            path,
            state,
            min(generic_remaining, max(1, args.max_events_per_file)),
            max(0, args.generic_relation_backfill_bytes),
        )
        generic_files_processed += 1
        generic_relations += count
        generic_remaining -= count
        if generic_remaining <= 0:
            break
    if paths:
        state["generic_relation_file_cursor"] = next_generic_cursor
    for path in paths:
        terminal_outcomes += harvest_terminal_outcomes(
            path, state, max(1, args.max_events_per_file), shadow_candidates
        )
    correlated_outcomes = reconcile_provider_outcomes(state, max(1, args.max_events))
    if args.dry_run:
        for observation in state["outbox"]:
            print(json.dumps(observation, ensure_ascii=False, separators=(",", ":")))
        for outcome in state["outcome_outbox"]:
            print(json.dumps(outcome, ensure_ascii=False, separators=(",", ":")))
        for relation in state["response_relation_outbox"]:
            print(json.dumps(relation, ensure_ascii=False, separators=(",", ":")))
        for shadow in state["response_shadow_outbox"]:
            print(json.dumps(shadow, ensure_ascii=False, separators=(",", ":")))
        for frame in state["relation_frame_outbox"]:
            print(json.dumps(frame, ensure_ascii=False, separators=(",", ":")))
        return 0
    save_state(args.state, state)
    flush_outbox(state, args.observe_url, max(0.05, args.timeout), max(1, args.max_events))
    written_outcomes = append_provider_outcomes(
        args.provider_outcomes, state, max(1, args.max_events)
    )
    written_relations = append_response_relations(
        args.response_relations, state, max(1, args.max_events)
    )
    written_shadows = append_response_shadows(
        args.response_shadows, state, max(1, args.max_events)
    )
    written_frames = append_relation_frames(
        args.relation_frames, state, max(1, args.max_events)
    )
    save_state(args.state, state)
    print(
        json.dumps(
            {
                "schema": "nando.codex-session-capture-run.v1",
                "harvested": harvested,
                "pending_delivery": len(state["outbox"]),
                "delivered_total": len(state["delivered_event_ids"]),
                "provider_requests_harvested": provider_requests,
                "terminal_outcomes_harvested": terminal_outcomes,
                "wait_relations_harvested": wait_relations,
                "wait_relation_files_processed": len(wait_paths),
                "wait_hot_files_processed": wait_hot_files_processed,
                "wait_backfill_files_processed": len(wait_paths)
                - wait_hot_files_processed,
                "generic_response_relations_harvested": generic_relations,
                "generic_relation_files_processed": generic_files_processed,
                "generic_hot_files_processed": generic_hot_files_processed,
                "generic_backfill_files_processed": generic_files_processed
                - generic_hot_files_processed,
                "provider_outcomes_correlated": correlated_outcomes,
                "provider_outcomes_written": written_outcomes,
                "provider_outcomes_pending": len(state["outcome_outbox"]),
                "provider_outcomes_total": len(state["delivered_outcome_ids"]),
                "response_relations_written": written_relations,
                "response_relations_total": len(state["delivered_response_relation_ids"]),
                "response_shadows_written": written_shadows,
                "response_shadows_total": len(state["delivered_response_shadow_ids"]),
                "relation_frames_written": written_frames,
                "relation_frames_total": len(state["delivered_relation_frame_ids"]),
            },
            separators=(",", ":"),
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
