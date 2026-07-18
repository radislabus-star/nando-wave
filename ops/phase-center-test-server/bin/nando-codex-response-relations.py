#!/usr/bin/env python3
"""Extract source-neutral response relations from Codex session events.

Hot relation atoms contain only hashes and bounded structure. A separately
tagged cold synthesis example may retain a privacy-screened JSON collection;
it is never part of runtime routing or a compiled package.
"""

from __future__ import annotations

import hashlib
import json
import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


RELATION_FRAME_SCHEMA = "nando.response-relation-frame.v1"
RELATION_EXTRACTOR_VERSION = "response-relation-extractor.v15"
CLASSIFIER_VERSION = "generic-role-transfer-v15-multi-claim-self-training"
MAX_OUTPUT_BYTES = 16_384
MAX_SCALARS = 64
MAX_DEPTH = 8
MAX_CALLS = 64
MAX_ARGUMENTS = 16
MAX_PROJECT_STATUS_CODE = 1_000_000
MAX_INTEGER_CONSTANT = MAX_PROJECT_STATUS_CODE
# Conservative wire/state bound that remains exactly representable by u64 consumers.
MAX_LAST_INPUT_TOKENS = (1 << 32) - 1
MAX_PENDING_FRAMES = MAX_SCALARS
MAX_READY_FRAMES = MAX_SCALARS
MAX_STORED_FRAME_BYTES = 65_536
VALUE_TYPES = {"identifier", "string", "integer", "boolean"}
CONTEXT_ROLES = {
    "turn_call_count_band",
    "turn_output_count_band",
    "turn_pending_count_band",
    "active_pending_handle_count_band",
    "turn_message_count_band",
    "turn_call_shape_count_band",
}
PRIVATE_IDENTIFIER_PART = re.compile(
    r"(?:auth|cookie|credential|passwd|password|secret|token|api_?key|private_?key)",
    re.IGNORECASE,
)
RENDERER_STATIC_WORDS = {
    "selected", "select", "value", "values", "result", "results", "count", "total",
    "status", "item", "items", "row", "rows", "record", "records", "entry", "entries",
    "matching", "matched", "filtered", "found", "output", "data", "is", "are", "was",
    "were", "success", "failure", "passed", "failed", "true", "false", "none", "empty",
    "выбрано", "выбранные", "значение", "значения", "результат", "результаты",
    "количество", "всего", "статус", "элемент", "элементы", "строка", "строки", "запись",
    "записи", "найдено", "найденные", "отфильтровано", "успешно", "ошибка", "да", "нет",
    "пусто", "данные",
    "на", "не", "это", "его", "уже", "был", "есть", "то", "только",
    "проверка", "готово", "заблокирован", "подтвердила", "должен", "обновлять",
    "автоматически", "обновил", "остался", "тронул", "apt", "chrome", "hold",
}
RENDERER_STATIC_PUNCTUATION = frozenset(".,:;!?()[]{}'\"-/_*`#+|<>%")


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


def stable_atom_id(value: str) -> int:
    result = 0xCBF29CE484222325
    for byte in value.encode("utf-8"):
        result = ((result ^ byte) * 0x100000001B3) & ((1 << 64) - 1)
    return result


def request_phase_atom_ids(value: Any) -> list[int]:
    if isinstance(value, list):
        value = " ".join(
            part.get("text", "")
            for part in value
            if isinstance(part, dict) and part.get("type") in {"text", "input_text"}
        )
    if not isinstance(value, str):
        return []
    tokens = [
        token
        for token in re.findall(r"\w+", value.lower(), flags=re.UNICODE)
        if len(token.encode("utf-8")) <= 32
    ][:64]
    atoms = [stable_atom_id(f"request_token:{token}") for token in tokens]
    atoms.extend(
        stable_atom_id(f"request_bigram:{left}:{right}")
        for left, right in zip(tokens, tokens[1:])
    )
    return sorted(set(atoms))[:64]


def count_band(value: int) -> int:
    if value <= 0:
        return 0
    return 1 << (value.bit_length() - 1)


def empty_turn() -> dict[str, Any]:
    return {
        "calls": 0,
        "outputs": 0,
        "pending_outputs": 0,
        "messages": 0,
        "call_shapes": [],
        "ordinal": 0,
    }


def empty_file_state() -> dict[str, Any]:
    return {
        "offset": 0,
        "session_id_sha256": "",
        "calls": {},
        "observations": [],
        "turn_selector_values": {},
        "active_selector_values": {},
        "latest_output_sha256": "",
        "collection_observation": None,
        "collection_observation_ambiguous": False,
        "request_phase_atom_ids": [],
        "turn": empty_turn(),
        "pending_model_action": [],
        "ready_frames": [],
    }


def normalize_file_state(value: Any) -> dict[str, Any]:
    state = value if isinstance(value, dict) else empty_file_state()
    normalized = empty_file_state()
    normalized["offset"] = _nonnegative_int(state.get("offset"))
    session = state.get("session_id_sha256")
    if isinstance(session, str) and _valid_sha256(session):
        normalized["session_id_sha256"] = session
    turn = state.get("turn")
    if isinstance(turn, dict):
        for key in ("calls", "outputs", "pending_outputs", "messages", "ordinal"):
            normalized["turn"][key] = _nonnegative_int(turn.get(key))
        shapes = turn.get("call_shapes")
        if isinstance(shapes, list):
            normalized["turn"]["call_shapes"] = [
                item[:80] for item in shapes[-MAX_CALLS:] if isinstance(item, str)
            ]
    calls = state.get("calls")
    if isinstance(calls, dict):
        for key, call in list(calls.items())[-MAX_CALLS:]:
            if (
                isinstance(key, str)
                and isinstance(call, dict)
                and isinstance(call.get("name"), str)
                and isinstance(call.get("shape"), str)
            ):
                normalized["calls"][key] = {
                    "name": call["name"][:128],
                    "shape": call["shape"][:80],
                }
    observations = state.get("observations")
    if isinstance(observations, list):
        normalized["observations"] = [
            _compact_observation(observation)
            for observation in observations[:MAX_SCALARS]
            if _valid_observation(observation)
        ]
    selector_values = state.get("turn_selector_values")
    if isinstance(selector_values, dict):
        for key, values in list(selector_values.items())[:MAX_SCALARS]:
            if not _valid_sha256(key) or not isinstance(values, list):
                continue
            bounded = sorted(
                {value for value in values if isinstance(value, str) and _valid_sha256(value)}
            )[:MAX_SCALARS]
            if bounded:
                normalized["turn_selector_values"][key] = bounded
    active_values = state.get("active_selector_values")
    if isinstance(active_values, dict):
        for key, values in list(active_values.items())[:MAX_SCALARS]:
            if not _valid_sha256(key) or not isinstance(values, list):
                continue
            bounded = sorted(
                {value for value in values if isinstance(value, str) and _valid_sha256(value)}
            )[:MAX_SCALARS]
            if bounded:
                normalized["active_selector_values"][key] = bounded
    latest_output = state.get("latest_output_sha256")
    if _valid_sha256(latest_output):
        normalized["latest_output_sha256"] = latest_output
    collection = _canonical_collection_observation(state.get("collection_observation"))
    if collection is not None:
        normalized["collection_observation"] = collection
    normalized["collection_observation_ambiguous"] = bool(
        state.get("collection_observation_ambiguous")
    )
    request_atoms = state.get("request_phase_atom_ids")
    if isinstance(request_atoms, list):
        normalized["request_phase_atom_ids"] = sorted(
            {atom for atom in request_atoms if type(atom) is int and 0 < atom < (1 << 64)}
        )[:64]
    for key, limit in (
        ("pending_model_action", MAX_PENDING_FRAMES),
        ("ready_frames", MAX_READY_FRAMES),
    ):
        entries = state.get(key)
        if isinstance(entries, list):
            normalized[key] = [
                frame
                for entry in entries[:limit]
                if (frame := _normalize_stored_frame(entry)) is not None
            ]
    return normalized


def context_atoms(turn: dict[str, Any]) -> list[dict[str, Any]]:
    shapes = turn.get("call_shapes")
    distinct_shapes = len(set(shapes)) if isinstance(shapes, list) else 0
    return [
        _cardinality("turn_call_count_band", count_band(int(turn.get("calls") or 0))),
        _cardinality("turn_output_count_band", count_band(int(turn.get("outputs") or 0))),
        _cardinality(
            "turn_pending_count_band", count_band(int(turn.get("pending_outputs") or 0))
        ),
        _cardinality("turn_message_count_band", count_band(int(turn.get("messages") or 0))),
        _cardinality("turn_call_shape_count_band", count_band(distinct_shapes)),
    ]


def extract_unique_scalar(output: str) -> dict[str, str] | None:
    if not isinstance(output, str):
        return None
    encoded = output.encode("utf-8")
    if not output.strip() or len(encoded) > MAX_OUTPUT_BYTES:
        return None
    stripped = output.strip()
    try:
        value = json.loads(stripped)
    except json.JSONDecodeError:
        if "\n" in stripped or "\r" in stripped:
            return None
        value = stripped
    scalars: list[tuple[Any, str]] = []
    if not _collect_scalars(value, 0, scalars) or len(scalars) != 1:
        return None
    scalar, _ = scalars[0]
    return _observation_scalar_descriptor(scalar)


def extract_observation_candidates(output: Any) -> list[dict[str, Any]]:
    texts = _output_text_parts(output)
    candidates: list[dict[str, Any]] = []
    rejected_structural_selector = False
    for text in texts:
        for line in text.splitlines():
            match = re.fullmatch(
                r"([A-Za-z_][A-Za-z0-9_]{0,63}(?:=|:\s*))([^\s].*)",
                line.strip(),
            )
            if match is None:
                continue
            if not _safe_structural_identifier(match.group(1)[:-1]):
                rejected_structural_selector = True
                continue
            scalar = _parse_scalar_text(match.group(2))
            if scalar is not None:
                candidates.append(
                    {
                        **scalar,
                        "selector": {
                            "kind": "content_line_prefix",
                            "prefix": match.group(1),
                            "value_type": scalar["value_type"],
                        },
                    }
                )
        try:
            parsed = json.loads(text)
        except json.JSONDecodeError:
            continue
        if isinstance(parsed, dict):
            for field, value in parsed.items():
                scalar = _observation_scalar_descriptor(value)
                if scalar is None:
                    continue
                if not _safe_structural_identifier(field):
                    rejected_structural_selector = True
                    continue
                candidates.append(
                    {
                        **scalar,
                        "selector": {
                            "kind": "json_field",
                            "field": field,
                            "value_type": scalar["value_type"],
                        },
                    }
                )
    if len(texts) == 1 and not rejected_structural_selector and not candidates:
        scalar = extract_unique_scalar(texts[0])
        if scalar is not None:
            candidates = [
                {
                    **scalar,
                    "selector": {
                        "kind": "unique_scalar",
                        "value_type": scalar["value_type"],
                    },
                }
            ]
    deduped: dict[tuple[str, str], dict[str, Any]] = {}
    for candidate in candidates:
        key = (candidate["value_sha256"], sha256_value(candidate["selector"]))
        if key in deduped:
            # Repeated matches for one selector are ambiguous at replay time.
            deduped[key].pop("integer_status_class", None)
        else:
            deduped[key] = candidate
    return list(deduped.values())[:MAX_SCALARS]


def extract_turn_output_line_candidates(output: Any, output_ordinal: int) -> list[dict[str, Any]]:
    if not 1 <= output_ordinal <= MAX_CALLS:
        return []
    lines = [
        line.strip()
        for text in _output_text_parts(output)
        for line in text.splitlines()
        if line.strip()
    ]
    candidates = []
    for line_index, line in enumerate(lines[:256]):
        if len(line.encode("utf-8")) > 512 or any(
            not character.isprintable() for character in line
        ):
            continue
        candidates.append(
            {
                "value_sha256": sha256_value(line),
                "value_type": "string",
                "selector": {
                    "kind": "turn_output_line",
                    "output_ordinal": output_ordinal,
                    "line_index": line_index,
                    "value_type": "string",
                },
            }
        )
    return candidates


def harvest_relation_frames(
    path: Path,
    file_state: dict[str, Any],
    max_events: int,
    max_lines: int | None = None,
) -> list[dict[str, Any]]:
    normalized = normalize_file_state(file_state)
    file_state.clear()
    file_state.update(normalized)
    size = path.stat().st_size
    if int(file_state.get("offset") or 0) > size:
        file_state.clear()
        file_state.update(empty_file_state())
    frames: list[dict[str, Any]] = []
    _drain_ready(file_state, frames, max(1, max_events))
    line_budget = max_lines if max_lines is not None else max(256, min(4_096, max_events * 64))
    lines_read = 0
    with path.open("rb") as handle:
        handle.seek(int(file_state.get("offset") or 0))
        while len(frames) < max(1, max_events) and lines_read < max(1, line_budget):
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
            _queue_ready(file_state, _observe_row(row, file_state))
            _drain_ready(file_state, frames, max(1, max_events))
    return frames


def _observe_row(row: Any, state: dict[str, Any]) -> list[dict[str, Any]]:
    if not isinstance(row, dict):
        return []
    payload = row.get("payload")
    if not isinstance(payload, dict):
        return []
    row_type = row.get("type")
    if row_type == "session_meta":
        session_id = payload.get("id")
        if isinstance(session_id, str) and session_id:
            state["session_id_sha256"] = sha256_text(session_id)
        return []
    if row_type == "turn_context" or (
        row_type == "event_msg" and payload.get("type") == "context_compacted"
    ):
        flushed = _take_pending(state, 0)
        _start_turn(state)
        return flushed
    if row_type == "event_msg" and payload.get("type") == "user_message":
        flushed = _take_pending(state, 0)
        _start_turn(state)
        state["request_phase_atom_ids"] = request_phase_atom_ids(payload.get("message"))
        return flushed
    if (
        row_type == "event_msg"
        and payload.get("type") == "agent_message"
        and payload.get("phase") == "final_answer"
    ):
        flushed = _take_pending(state, 0)
        normalized_row = {
            **row,
            "payload": {
                "type": "message",
                "role": "assistant",
                "phase": payload.get("phase"),
                "content": [
                    {"type": "output_text", "text": payload.get("message")}
                ],
            },
        }
        state["pending_model_action"] = _frame_for_assistant_message(
            normalized_row, state
        )
        state["turn"]["messages"] += 1
        state["observations"] = []
        state["collection_observation"] = None
        state["collection_observation_ambiguous"] = False
        return flushed
    if row_type == "event_msg" and payload.get("type") == "token_count":
        info = payload.get("info")
        usage = info.get("last_token_usage") if isinstance(info, dict) else None
        input_tokens = usage.get("input_tokens") if isinstance(usage, dict) else None
        return _take_pending(state, _bounded_input_tokens(input_tokens))
    if row_type != "response_item":
        return []

    item_type = payload.get("type")
    if item_type == "message":
        role = payload.get("role")
        if role == "user":
            flushed = _take_pending(state, 0)
            _start_turn(state)
            state["request_phase_atom_ids"] = request_phase_atom_ids(payload.get("content"))
            return flushed
        if role == "assistant":
            flushed = _take_pending(state, 0)
            state["pending_model_action"] = (
                _frame_for_assistant_message(row, state)
                if payload.get("phase") == "final_answer"
                else []
            )
            state["turn"]["messages"] += 1
            state["observations"] = []
            state["collection_observation"] = None
            state["collection_observation_ambiguous"] = False
            return flushed
        return []

    if item_type in {"custom_tool_call", "function_call"}:
        flushed = _take_pending(state, 0)
        state["pending_model_action"] = _frame_for_action(row, state)
        _remember_call(payload, item_type, state)
        state["observations"] = _replayable_turn_observations(state)
        return flushed

    if item_type in {"custom_tool_call_output", "function_call_output"}:
        _remember_output(payload, state)
    return []


def _start_turn(state: dict[str, Any]) -> None:
    ordinal = int(state.get("turn", {}).get("ordinal") or 0) + 1
    state["calls"] = {}
    state["observations"] = []
    state["turn_selector_values"] = {}
    state["active_selector_values"] = {}
    state["latest_output_sha256"] = ""
    state["collection_observation"] = None
    state["collection_observation_ambiguous"] = False
    state["request_phase_atom_ids"] = []
    state["turn"] = empty_turn()
    state["turn"]["ordinal"] = ordinal


def _remember_call(payload: dict[str, Any], item_type: str, state: dict[str, Any]) -> None:
    call_id = payload.get("call_id")
    name = payload.get("name")
    source = payload.get("arguments") if item_type == "function_call" else payload.get("input")
    if not isinstance(call_id, str) or not call_id or not isinstance(name, str) or not name:
        return
    if not isinstance(source, str):
        return
    shape = item_type
    state["turn"]["calls"] += 1
    state["turn"]["call_shapes"].append(shape)
    state["turn"]["call_shapes"] = state["turn"]["call_shapes"][-MAX_CALLS:]
    state["calls"][sha256_text(call_id)] = {"name": name[:128], "shape": shape}
    while len(state["calls"]) > MAX_CALLS:
        state["calls"].pop(next(iter(state["calls"])))


def _remember_output(payload: dict[str, Any], state: dict[str, Any]) -> None:
    state["turn"]["outputs"] += 1
    call_key = sha256_text(str(payload.get("call_id") or ""))
    call = state["calls"].pop(call_key, None)
    output = payload.get("output")
    if not isinstance(call, dict) or not isinstance(output, (str, list)):
        state["observations"] = []
        return
    if isinstance(output, str) and output.startswith("Script running with cell ID "):
        state["turn"]["pending_outputs"] += 1
        state["observations"] = []
        return
    candidates = extract_observation_candidates(output)
    candidates.extend(
        extract_turn_output_line_candidates(output, int(state["turn"]["outputs"]))
    )
    selector_values = state.setdefault("turn_selector_values", {})
    if not isinstance(selector_values, dict):
        state["turn_selector_values"] = selector_values = {}
    active_values = state.setdefault("active_selector_values", {})
    if not isinstance(active_values, dict):
        state["active_selector_values"] = active_values = {}
    if _output_has_completion_marker(output):
        for selector_key, values in list(active_values.items()):
            if isinstance(values, list) and len(values) == 1:
                active_values[selector_key] = []
    for candidate in candidates:
        selector = candidate.get("selector")
        if not isinstance(selector, dict) or selector.get("kind") != "json_field":
            continue
        selector_key = sha256_value(selector)
        values = selector_values.setdefault(selector_key, [])
        if not isinstance(values, list):
            selector_values[selector_key] = values = []
        value_sha256 = candidate["value_sha256"]
        if value_sha256 not in values and len(values) < MAX_SCALARS:
            values.append(value_sha256)
        active = active_values.setdefault(selector_key, [])
        if not isinstance(active, list):
            active_values[selector_key] = active = []
        if value_sha256 not in active and len(active) < MAX_SCALARS:
            active.append(value_sha256)
        if len(active) == 1:
            candidate["selector"] = {**selector, "kind": "unique_active_turn_json_field"}
        elif len(values) == 1:
            candidate["selector"] = {**selector, "kind": "unique_turn_json_field"}
    collection = _collection_observation(output, call)
    if collection is not None and not state.get("collection_observation_ambiguous"):
        current = _canonical_collection_observation(state.get("collection_observation"))
        if current is None:
            state["collection_observation"] = collection
        elif canonical_bytes(current) != canonical_bytes(collection):
            state["collection_observation"] = None
            state["collection_observation_ambiguous"] = True
    output_sha256 = sha256_value(output)
    state["latest_output_sha256"] = output_sha256
    current_observations = [
        {
            **candidate,
            "tool_kind": call["name"],
            "call_shape": call["shape"],
            "completion_state": (
                "completed"
                if candidate["selector"]["kind"] == "unique_scalar"
                else "pending"
            ),
            "output_sha256": output_sha256,
            "context_atoms": context_atoms(state["turn"]),
        }
        for candidate in candidates
    ]
    state["observations"] = _merge_turn_observations(
        _replayable_turn_observations(state), current_observations
    )


def _replayable_turn_observations(state: dict[str, Any]) -> list[dict[str, Any]]:
    turn_values = state.get("turn_selector_values")
    active_values = state.get("active_selector_values")
    if not isinstance(turn_values, dict) or not isinstance(active_values, dict):
        return []
    replayable: list[dict[str, Any]] = []
    for observation in state.get("observations", [])[:MAX_SCALARS]:
        if not _valid_observation(observation):
            continue
        selector = observation.get("selector")
        if not isinstance(selector, dict) or selector.get("kind") not in {
            "json_field",
            "unique_turn_json_field",
            "unique_active_turn_json_field",
            "turn_output_line",
        }:
            continue
        if selector.get("kind") == "turn_output_line":
            replayable.append(observation)
            continue
        base_selector = {
            "kind": "json_field",
            "field": selector.get("field"),
            "value_type": selector.get("value_type"),
        }
        selector_key = sha256_value(base_selector)
        value_sha256 = observation.get("value_sha256")
        turn_matches = turn_values.get(selector_key)
        active_matches = active_values.get(selector_key)
        if isinstance(active_matches, list) and active_matches == [value_sha256]:
            kind = "unique_active_turn_json_field"
        elif isinstance(turn_matches, list) and turn_matches == [value_sha256]:
            kind = "unique_turn_json_field"
        else:
            continue
        replayable.append(
            {
                **observation,
                "selector": {**base_selector, "kind": kind},
            }
        )
    return replayable


def _merge_turn_observations(
    retained: list[dict[str, Any]], current: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    merged: dict[tuple[str, str], dict[str, Any]] = {}
    for observation in [*retained, *current]:
        if not _valid_observation(observation):
            continue
        selector = observation["selector"]
        field_key = sha256_value(selector)
        key = (observation["value_sha256"], field_key)
        merged[key] = observation
    return list(merged.values())[:MAX_SCALARS]


def _frame_for_action(row: dict[str, Any], state: dict[str, Any]) -> list[dict[str, Any]]:
    if not _valid_sha256(state.get("session_id_sha256")):
        return []
    observations = [
        observation
        for observation in state.get("observations", [])
        if _valid_observation(observation)
        and observation.get("output_sha256") == state.get("latest_output_sha256")
        and observation.get("selector", {}).get("kind") != "turn_output_line"
    ]
    if not observations:
        return []
    payload = row["payload"]
    action = _parse_action(payload)
    matches: list[tuple[dict[str, Any], str]] = []
    if action is not None:
        for observation in observations:
            for name, value in action["arguments"].items():
                if _scalar_descriptor(value) == (
                    observation["value_sha256"],
                    observation["value_type"],
                ):
                    matches.append((observation, name))
    if len(matches) == 1 and action is not None and _safe_action_arguments(action, matches[0][1]):
        observation, matched_name = matches[0]
        return [_build_frame(row, state, observation, action, matched_name)]
    return [
        _build_frame(row, state, observation, action, None)
        for observation in observations
    ]


def _frame_for_assistant_message(
    row: dict[str, Any], state: dict[str, Any]
) -> list[dict[str, Any]]:
    payload = row.get("payload")
    if not isinstance(payload, dict) or payload.get("phase") != "final_answer":
        return []
    collection_frame = _collection_frame(row, state, payload)
    if collection_frame is not None:
        return [collection_frame]
    observations = [
        observation
        for observation in state.get("observations", [])
        if _valid_observation(observation)
    ]
    if not observations or not _valid_sha256(state.get("session_id_sha256")):
        return []
    status_projection = _parse_assistant_status_projection(payload)
    if status_projection is not None:
        integer_observations = [
            observation
            for observation in observations
            if observation["value_type"] == "integer"
        ]
        action = {
            "shape": "assistant_message",
            "arguments": {},
            "valid": True,
            "status_projection": "zero_is_success",
        }
        if (
            len(integer_observations) == 1
            and integer_observations[0].get("integer_status_class")
            == status_projection
        ):
            return [
                _build_frame(
                    row,
                    state,
                    integer_observations[0],
                    action,
                    "assistant_status",
                )
            ]
        return [
            _build_frame(row, state, observation, action, None)
            for observation in observations
            if observation.get("selector", {}).get("kind") != "turn_output_line"
        ]

    projection = _parse_assistant_value_projection(payload)
    matches = []
    if projection is not None:
        descriptor = (projection["value_sha256"], projection["value_type"])
        matches = [
            observation
            for observation in observations
            if (observation["value_sha256"], observation["value_type"])
            == descriptor
        ]
    if not matches and payload is not None:
        sequence_match = _match_assistant_render_sequence(payload, observations)
        if sequence_match is not None:
            observation, renderer = sequence_match
            projection = {
                "value_sha256": observation["value_sha256"],
                "value_type": observation["value_type"],
                "format": "plain_text",
                "renderer": renderer,
            }
            matches = [observation]
    if not matches and payload is not None:
        template_matches = _match_assistant_value_template(payload, observations)
        if len(template_matches) == 1:
            observation, projection = template_matches[0]
            matches = [observation]
    action = (
        {
            "shape": "assistant_message",
            "arguments": {},
            "valid": True,
            "value_projection": projection["format"],
            "value_renderer": projection.get("renderer"),
        }
        if projection is not None
        else None
    )
    if len(matches) == 1 and action is not None:
        return [_build_frame(row, state, matches[0], action, "assistant_output")]
    return [
        _build_frame(row, state, observation, action, None)
        for observation in observations
        if observation.get("selector", {}).get("kind") != "turn_output_line"
    ]


def _parse_assistant_status_projection(payload: Any) -> str | None:
    text = _assistant_final_output_text(payload)
    if text == "success":
        return "zero"
    if text == "failure":
        return "nonzero"
    return None


def _parse_assistant_value_projection(payload: Any) -> dict[str, str] | None:
    text = _assistant_final_output_text(payload)
    if text is None:
        return None
    try:
        value = json.loads(text)
    except json.JSONDecodeError:
        value = text
        projection_format = "plain_text"
    else:
        projection_format = "canonical_json"
        if canonical_bytes(value).decode("utf-8") != text:
            return None
    descriptor = _scalar_descriptor(value)
    if descriptor is None:
        return None
    return {
        "value_sha256": descriptor[0],
        "value_type": descriptor[1],
        "format": projection_format,
    }


def _match_assistant_value_template(
    payload: Any, observations: list[dict[str, Any]]
) -> list[tuple[dict[str, Any], dict[str, Any]]]:
    text = _assistant_final_output_text(payload)
    if text is None or "\n" in text or len(text.encode("utf-8")) > 1_024:
        return []
    tokens = list(re.finditer(r"[^\W_]+", text, flags=re.UNICODE))[:32]
    spans = [
        (tokens[left].start(), tokens[right].end())
        for left in range(len(tokens))
        for right in range(left, min(len(tokens), left + 8))
    ]
    spans.extend(
        (match.start(), match.end())
        for match in re.finditer(r"(?<![\w.])-?\d+(?![\w.])", text)
    )
    spans.extend(
        (match.start(), match.end())
        for match in re.finditer(r"\b(?:true|false)\b", text, flags=re.IGNORECASE)
    )
    matches: list[tuple[dict[str, Any], dict[str, Any]]] = []
    seen: set[tuple[int, int, str]] = set()
    for observation in observations:
        expected_hash = observation["value_sha256"]
        value_type = observation["value_type"]
        for start, end in spans[:256]:
            raw = text[start:end]
            if value_type == "integer":
                try:
                    candidate: Any = int(raw)
                except ValueError:
                    continue
            elif value_type == "boolean":
                if raw.lower() not in {"true", "false"}:
                    continue
                candidate = raw.lower() == "true"
            else:
                candidate = raw
            if sha256_value(candidate) != expected_hash:
                continue
            prefix = text[:start]
            suffix = text[end:]
            key = (start, end, expected_hash)
            if (
                (not prefix and not suffix)
                or key in seen
                or not _safe_collection_template(prefix + suffix)
            ):
                continue
            if len((prefix + suffix).encode("utf-8")) > 512:
                continue
            seen.add(key)
            matches.append(
                (
                    observation,
                    {
                        "value_sha256": expected_hash,
                        "value_type": value_type,
                        "format": "plain_text",
                        "renderer": {
                            "kind": "render_template",
                            "prefix": prefix,
                            "suffix": suffix,
                        },
                    },
                )
            )
    return matches


def _match_assistant_render_sequence(
    payload: Any, observations: list[dict[str, Any]]
) -> tuple[dict[str, Any], dict[str, Any]] | None:
    text = _assistant_final_output_text(payload)
    if text is None or len(text.encode("utf-8")) > 1_024:
        return None
    tokens = list(re.finditer(r"[^\W_]+", text, flags=re.UNICODE))[:64]
    spans = {
        (tokens[left].start(), tokens[right].end())
        for left in range(len(tokens))
        for right in range(left, min(len(tokens), left + 8))
    }
    spans.update(
        (match.start(), match.end())
        for match in re.finditer(r"(?<![\w.])-?\d+(?![\w.])", text)
    )
    spans.update(
        (match.start(), match.end())
        for match in re.finditer(r"\b(?:true|false)\b", text, flags=re.IGNORECASE)
    )
    spans.update((match.start(), match.end()) for match in re.finditer(r"\S+", text))
    by_span: dict[tuple[int, int], list[dict[str, Any]]] = {}
    for observation in observations[:MAX_SCALARS]:
        expected_hash = observation["value_sha256"]
        value_type = observation["value_type"]
        for start, end in sorted(spans)[:512]:
            raw = text[start:end]
            if value_type == "integer":
                try:
                    candidate: Any = int(raw)
                except ValueError:
                    continue
            elif value_type == "boolean":
                if raw.lower() not in {"true", "false"}:
                    continue
                candidate = raw.lower() == "true"
            else:
                candidate = raw
            if sha256_value(candidate) == expected_hash:
                by_span.setdefault((start, end), []).append(observation)
    selected = [
        (start, end, candidates[0])
        for (start, end), candidates in sorted(by_span.items())
        if len(candidates) == 1
    ]
    if not 2 <= len(selected) <= 7:
        return None
    if any(left[1] > right[0] for left, right in zip(selected, selected[1:])):
        return None
    static_text = "".join(
        text[previous_end:start]
        for previous_end, (start, _, _) in zip(
            [0, *(end for _, end, _ in selected)], selected
        )
    ) + text[selected[-1][1]:]
    if not static_text or not _safe_collection_template(static_text):
        return None
    if len(static_text.encode("utf-8")) > 512:
        return None
    segments: list[dict[str, Any]] = []
    cursor = 0
    for index, (start, end, observation) in enumerate(selected):
        if start > cursor:
            segments.append({"kind": "static", "text": text[cursor:start]})
        if index == 0:
            segments.append({"kind": "primary"})
        else:
            segments.append(
                {
                    "kind": "selected",
                    "selector": observation["selector"],
                    "format": "plain_text",
                }
            )
        cursor = end
    if cursor < len(text):
        segments.append({"kind": "static", "text": text[cursor:]})
    if not 3 <= len(segments) <= 16:
        return None
    return selected[0][2], {"kind": "render_sequence", "segments": segments}


def _assistant_final_output_text(payload: Any) -> str | None:
    if not isinstance(payload, dict) or payload.get("phase") != "final_answer":
        return None
    content = payload.get("content")
    if not isinstance(content, list) or len(content) != 1:
        return None
    part = content[0]
    if not isinstance(part, dict) or part.get("type") != "output_text":
        return None
    text = part.get("text")
    if not isinstance(text, str) or not text or len(text.encode("utf-8")) > MAX_OUTPUT_BYTES:
        return None
    if "\r" in text or text != text.strip():
        return None
    return text


def _safe_collection_scalar(value: Any) -> bool:
    if value is None or type(value) in {bool, int}:
        return not isinstance(value, int) or -(1 << 53) <= value <= (1 << 53)
    if not isinstance(value, str) or len(value.encode("utf-8")) > 128:
        return False
    if PRIVATE_IDENTIFIER_PART.search(value):
        return False
    compact = value.replace("_", "").replace("-", "")
    return not (
        len(compact) >= 24
        and re.fullmatch(r"[A-Za-z0-9+/=]+", compact)
        and sum(bool(re.search(pattern, compact)) for pattern in (r"[a-z]", r"[A-Z]", r"[0-9]")) >= 3
    )


def _output_has_completion_marker(output: Any) -> bool:
    for text in _collection_text_blocks(output):
        try:
            value = json.loads(text)
        except json.JSONDecodeError:
            continue
        if (
            isinstance(value, dict)
            and type(value.get("exit_code")) is int
            and -(1 << 31) <= value["exit_code"] <= (1 << 31) - 1
        ):
            return True
    return False


def _collection_text_blocks(output: Any) -> list[str]:
    if isinstance(output, str):
        return [output]
    if not isinstance(output, list):
        return []
    return [
        part["text"]
        for part in output
        if isinstance(part, dict)
        and part.get("type") == "input_text"
        and isinstance(part.get("text"), str)
    ]


def _json_documents(text: str) -> list[Any]:
    if not text or len(text.encode("utf-8")) > MAX_STORED_FRAME_BYTES:
        return []
    sources = [text]
    sources.extend(
        match.group(1)
        for match in re.finditer(
            r"```(?:json)?[ \t]*\r?\n([\s\S]*?)\r?\n```",
            text,
            flags=re.IGNORECASE,
        )
    )
    sources.extend(
        line.strip()
        for line in text.splitlines()
        if line.strip().startswith(("{", "["))
    )
    documents: dict[bytes, Any] = {}
    for source in sources:
        if not source or len(source.encode("utf-8")) > MAX_OUTPUT_BYTES:
            continue
        try:
            parsed = json.loads(source)
        except json.JSONDecodeError:
            continue
        documents[canonical_bytes(parsed)] = parsed
    return list(documents.values())


def _bounded_collection_root(parsed: Any) -> dict[str, Any] | None:
    if isinstance(parsed, list):
        parsed = {"items": parsed}
    if not isinstance(parsed, dict) or len(parsed) > 16:
        return None
    arrays = [(key, value) for key, value in parsed.items() if isinstance(value, list)]
    if len(arrays) != 1:
        return None
    collection_key, rows = arrays[0]
    if not _safe_structural_identifier(collection_key) or not (1 <= len(rows) <= 1024):
        return None
    for row in rows:
        if not isinstance(row, dict) or not (1 <= len(row) <= 16):
            return None
        if any(not _safe_structural_identifier(key) for key in row):
            return None
        if any(not _safe_collection_scalar(value) for value in row.values()):
            return None
    return parsed


def _bounded_collection_json(output: Any) -> dict[str, Any] | None:
    candidates: dict[bytes, dict[str, Any]] = {}
    for text in _collection_text_blocks(output):
        for document in _json_documents(text):
            parsed = _bounded_collection_root(document)
            if parsed is not None:
                candidates[canonical_bytes(parsed)] = parsed
    if len(candidates) != 1:
        return None
    return next(iter(candidates.values()))


def _collection_observation(output: Any, call: dict[str, Any]) -> dict[str, Any] | None:
    parsed = _bounded_collection_json(output)
    if parsed is None:
        return None
    return {
        "provider_payload": {
            "input": [{"type": f"{call['shape']}_output", "output": canonical_bytes(parsed).decode("utf-8")}]
        },
        "tool_kind_sha256": sha256_text(call["name"]),
        "call_shape": call["shape"],
    }


def _canonical_collection_observation(value: Any) -> dict[str, Any] | None:
    if not isinstance(value, dict) or not _valid_sha256(value.get("tool_kind_sha256")):
        return None
    payload = value.get("provider_payload")
    item = payload.get("input", [None])[-1] if isinstance(payload, dict) else None
    parsed = _bounded_collection_json(item.get("output") if isinstance(item, dict) else None)
    if parsed is None or value.get("call_shape") not in {"function_call", "custom_tool_call"}:
        return None
    return _collection_observation(canonical_bytes(parsed).decode("utf-8"), {
        "name": value["tool_kind_sha256"],
        "shape": value["call_shape"],
    }) | {"tool_kind_sha256": value["tool_kind_sha256"]}


def _collection_result_candidates(root: dict[str, Any]) -> set[str]:
    rows = next((value for value in root.values() if isinstance(value, list)), [])
    if not rows or not all(isinstance(row, dict) for row in rows):
        return set()
    values: list[Any] = [rows, len(rows)]
    fields = set(rows[0])
    for row in rows[1:]:
        fields.intersection_update(row)
    for field in fields:
        values.append([row[field] for row in rows])
    for predicate in fields:
        literals = {canonical_bytes(row[predicate]) for row in rows}
        for encoded in literals:
            literal = json.loads(encoded)
            selected = [row for row in rows if row.get(predicate) == literal]
            values.extend((selected, len(selected)))
            for field in fields:
                values.append([row[field] for row in selected])
    candidates = {canonical_bytes(value).decode("utf-8") for value in values}
    candidates.update(
        value
        for value in (
            str(item).lower() if isinstance(item, bool) else str(item)
            for item in values
            if isinstance(item, (str, int, bool))
        )
        if value
    )
    return candidates


def _bounded_assistant_output_text(payload: Any) -> str | None:
    if not isinstance(payload, dict) or payload.get("phase") != "final_answer":
        return None
    content = payload.get("content")
    if not isinstance(content, list) or len(content) != 1:
        return None
    part = content[0]
    text = part.get("text") if isinstance(part, dict) and part.get("type") == "output_text" else None
    if not isinstance(text, str) or not text or len(text.encode("utf-8")) > 1_024:
        return None
    return text


def _safe_collection_template(value: str) -> bool:
    lower = value.lower()
    if any(not character.isprintable() and character != "\n" for character in value):
        return False
    words = re.findall(r"[^\W\d_]+", value.lower(), flags=re.UNICODE)
    if any(word not in RENDERER_STATIC_WORDS for word in words):
        return False
    residue = re.sub(r"[^\W\d_]+", "", value, flags=re.UNICODE)
    if any(
        not character.isspace() and character not in RENDERER_STATIC_PUNCTUATION
        for character in residue
    ):
        return False
    if PRIVATE_IDENTIFIER_PART.search(value) or any(
        term in lower for term in ("authorization", "bearer ", "credential", "cookie")
    ):
        return False
    if re.search(r"(?:https?://|www\.)|\b[\w.+-]+@[\w.-]+\.[A-Za-z]{2,}\b", value):
        return False
    if any(path in lower for path in ("/home/", "/etc/", "/var/", "/opt/", "/root/", "/tmp/")):
        return False
    if re.search(r"[A-Za-z]:[\\/]", value) or "\\\\" in value:
        return False
    if any(
        (len(run) >= 32 and all(character in "0123456789abcdefABCDEF" for character in run))
        or (
            len(run) >= 24
            and any(character.islower() for character in run)
            and any(character.isupper() for character in run)
            and any(character.isdigit() for character in run)
        )
        for run in re.findall(r"[A-Za-z0-9+/_-]+", value)
    ):
        return False
    return "-----begin " not in lower


def _collection_expected_response(payload: Any, root: dict[str, Any]) -> str | None:
    text = _bounded_assistant_output_text(payload)
    if text is None:
        return None
    try:
        parsed = json.loads(text)
    except json.JSONDecodeError:
        parsed = None
    if parsed is not None and canonical_bytes(parsed).decode("utf-8") == text:
        return text
    matches = [
        candidate
        for candidate in _collection_result_candidates(root)
        if candidate and text.count(candidate) == 1
    ]
    if not matches:
        return None
    result = max(matches, key=len)
    offset = text.index(result)
    static = text[:offset] + text[offset + len(result):]
    if len(static.encode("utf-8")) > 512 or not _safe_collection_template(static):
        return None
    return text


def _collection_frame(row: dict[str, Any], state: dict[str, Any], payload: dict[str, Any]) -> dict[str, Any] | None:
    if state.get("collection_observation_ambiguous"):
        return None
    observation = _canonical_collection_observation(state.get("collection_observation"))
    session = state.get("session_id_sha256")
    if observation is None or not _valid_sha256(session):
        return None
    observed_root = json.loads(observation["provider_payload"]["input"][0]["output"])
    expected = _collection_expected_response(payload, observed_root)
    if expected is None:
        return None
    cold = {
        "schema": "nando.response-collection-synthesis-example.v1",
        "provider_payload": observation["provider_payload"],
        "expected_response": expected,
    }
    observed_rows = next(value for value in observed_root.values() if isinstance(value, list))
    event_id = sha256_value({"session": session, "cold": cold})
    atoms = [
        {"kind": "observation_call_shape", "value": observation["call_shape"]},
        {"kind": "collection_shape", "array_fields": 1, "row_fields": len(observed_rows[0])},
        {"kind": "response_shape", "value": "assistant_message"},
        {"kind": "completion_state", "value": "completed"},
    ]
    atoms.extend(
        {"kind": "request_phase_atom", "atom_id": atom_id}
        for atom_id in state.get("request_phase_atom_ids", [])[:64]
        if type(atom_id) is int and 0 < atom_id < (1 << 64)
    )
    frame_id = sha256_value({"schema": RELATION_FRAME_SCHEMA, "event": event_id, "atoms": atoms, "extractor": RELATION_EXTRACTOR_VERSION})
    return {
        "schema": RELATION_FRAME_SCHEMA,
        "frame_id_sha256": frame_id,
        "event_id_sha256": event_id,
        "client_intent_id_sha256": sha256_value({"session": session, "turn": state.get("turn", {}).get("ordinal")}),
        "session_id_sha256": session,
        "observed_at_unix_nanos": _iso_to_unix_nanos(str(row.get("timestamp") or "")),
        "extractor_version": RELATION_EXTRACTOR_VERSION,
        "estimated_input_tokens": 0,
        "verifier_label": True,
        "atoms": atoms,
        "evidence_ref_sha256": sha256_value(cold),
        "cold_collection_example": cold,
    }


def _build_frame(
    row: dict[str, Any],
    state: dict[str, Any],
    observation: dict[str, Any],
    action: dict[str, Any] | None,
    matched_name: str | None,
) -> dict[str, Any]:
    positive = action is not None and matched_name is not None
    response_shape = action.get("shape", "function_call") if action else "function_call"
    atoms = _pre_action_atoms(observation, response_shape)
    if action is not None and action["shape"] == "function_call":
        atoms.append({"kind": "action_function", "value": action["function"]})
    elif (
        action is not None
        and action["shape"] == "custom_tool_call"
        and action.get("valid") is True
    ):
        atoms.extend(
            [
                {"kind": "action_custom_tool", "value": action["custom_tool"]},
                {"kind": "action_inner_tool", "value": action["inner_tool"]},
            ]
        )
        projection = action["projection"]
        if projection["kind"] == "json_stringify_result":
            atoms.append({"kind": "action_json_result_projection"})
        else:
            atoms.append(
                {
                    "kind": "action_result_projection",
                    "output_field": projection["output_field"],
                    "continuation_field": projection["continuation_field"],
                    "continuation_prefix": projection["continuation_prefix"],
                }
            )
    elif action is not None and action["shape"] == "assistant_message":
        if action.get("status_projection") == "zero_is_success":
            atoms.append(
                {
                    "kind": "action_status_projection",
                    "mapping": "zero_is_success",
                }
            )
        else:
            projection_atom = {
                "kind": "action_value_projection",
                "format": action["value_projection"],
            }
            if isinstance(action.get("value_renderer"), dict):
                projection_atom["renderer"] = action["value_renderer"]
            atoms.append(projection_atom)
    if positive and action.get("status_projection") is None:
        atoms.extend(
            [
                {
                    "kind": "typed_slot",
                    "slot_id": 2,
                    "value_type": observation["value_type"],
                    "source": "action",
                    "value_sha256": observation["value_sha256"],
                },
                {"kind": "slot_equality", "left_slot": 1, "right_slot": 2},
            ]
        )
        if action["shape"] != "assistant_message":
            atoms.insert(
                -1,
                {"kind": "action_role_argument", "name": matched_name, "slot_id": 2},
            )
            atoms.extend(_constant_argument_atoms(action["arguments"], matched_name))
    event_material = {
        "session": state.get("session_id_sha256"),
        "turn": state.get("turn", {}).get("ordinal"),
        "observation": observation["output_sha256"],
        "action": sha256_value(row.get("payload")),
    }
    event_id = sha256_value(event_material)
    frame_id = sha256_value(
        {
            "schema": RELATION_FRAME_SCHEMA,
            "event": event_id,
            "atoms": atoms,
            "extractor": RELATION_EXTRACTOR_VERSION,
        }
    )
    session = str(state.get("session_id_sha256") or "")
    frame = {
        "schema": RELATION_FRAME_SCHEMA,
        "frame_id_sha256": frame_id,
        "event_id_sha256": event_id,
        "client_intent_id_sha256": sha256_value(
            {"session": session, "turn": state.get("turn", {}).get("ordinal")}
        ),
        "session_id_sha256": session,
        "observed_at_unix_nanos": _iso_to_unix_nanos(str(row.get("timestamp") or "")),
        "extractor_version": RELATION_EXTRACTOR_VERSION,
        "estimated_input_tokens": 0,
        "verifier_label": positive,
        "atoms": atoms,
        "evidence_ref_sha256": sha256_value(event_material),
    }
    return frame


def _bounded_input_tokens(value: Any) -> int:
    """Return a conservative u64-compatible token estimate or the safe zero default."""
    if type(value) is not int or value < 0 or value > MAX_LAST_INPUT_TOKENS:
        return 0
    return value


def _nonnegative_int(value: Any) -> int:
    return value if type(value) is int and value >= 0 else 0


def _take_pending(state: dict[str, Any], input_tokens: int) -> list[dict[str, Any]]:
    pending = state.get("pending_model_action")
    state["pending_model_action"] = []
    if not isinstance(pending, list):
        return []
    frames = []
    for entry in pending[:MAX_PENDING_FRAMES]:
        frame = _normalize_stored_frame(entry)
        if frame is not None:
            frame["estimated_input_tokens"] = input_tokens
            frames.append(frame)
    return frames


def _queue_ready(state: dict[str, Any], frames: list[dict[str, Any]]) -> None:
    if not frames:
        return
    ready = state.setdefault("ready_frames", [])
    if not isinstance(ready, list):
        state["ready_frames"] = ready = []
    capacity = MAX_READY_FRAMES - len(ready)
    if len(frames) > capacity:
        raise RuntimeError("bounded ready frame queue overflow")
    ready.extend(frames)


def _drain_ready(
    state: dict[str, Any], output: list[dict[str, Any]], max_events: int
) -> None:
    ready = state.get("ready_frames")
    if not isinstance(ready, list) or len(output) >= max_events:
        return
    count = min(len(ready), max_events - len(output))
    output.extend(ready[:count])
    del ready[:count]


def _negative_for_unhandled_action(
    row: dict[str, Any], state: dict[str, Any], action_kind: str
) -> list[dict[str, Any]]:
    if not any(_valid_observation(value) for value in state.get("observations", [])):
        return []
    synthetic = dict(row)
    synthetic["payload"] = {
        "type": action_kind,
        "action_sha256": sha256_value(row.get("payload")),
    }
    return _frame_for_action(synthetic, state)


def _pre_action_atoms(
    observation: dict[str, Any], response_shape: str
) -> list[dict[str, Any]]:
    atoms = [
        {"kind": "tool_kind", "value": observation["tool_kind"]},
        {"kind": "observation_call_shape", "value": observation["call_shape"]},
        {"kind": "completion_state", "value": observation["completion_state"]},
        {"kind": "response_shape", "value": response_shape},
        {
            "kind": "typed_slot",
            "slot_id": 1,
            "value_type": observation["value_type"],
            "source": "observation",
            "value_sha256": observation["value_sha256"],
        },
        {"kind": "unique_slot", "slot_id": 1},
        {
            "kind": "observation_selector",
            "slot_id": 1,
            "selector": observation["selector"],
        },
    ]
    atoms.extend(_valid_context_atoms(observation.get("context_atoms")))
    return atoms


def _parse_arguments(value: Any) -> dict[str, Any] | None:
    if not isinstance(value, str) or len(value.encode("utf-8")) > MAX_OUTPUT_BYTES:
        return None
    try:
        parsed = json.loads(value)
    except json.JSONDecodeError:
        return None
    return parsed if isinstance(parsed, dict) else None


CUSTOM_TOOL_OUTPUT_CONTINUATION_PROGRAM = re.compile(
    r"\A\s*const\s+(?P<var>[A-Za-z_][A-Za-z0-9_]*)\s*=\s*await\s+"
    r"tools\.(?P<inner>[A-Za-z_][A-Za-z0-9_]*)\s*\((?P<args>\{.*?\})\)\s*;\s*"
    r"text\(\s*(?P=var)\.(?P<output>[A-Za-z_][A-Za-z0-9_]*)\s*\)\s*;\s*"
    r"if\s*\(\s*(?P=var)\.(?P<continuation>[A-Za-z_][A-Za-z0-9_]*)\s*\)\s*"
    r"text\(\s*`(?P<prefix>[^`$\r\n]{1,128})\$\{(?P=var)\.(?P=continuation)\}`\s*\)\s*;\s*\Z",
    re.DOTALL,
)

CUSTOM_TOOL_JSON_RESULT_PROGRAM = re.compile(
    r"\A\s*const\s+(?P<var>[A-Za-z_][A-Za-z0-9_]*)\s*=\s*await\s+"
    r"tools\.(?P<inner>[A-Za-z_][A-Za-z0-9_]*)\s*\((?P<args>\{.*?\})\)\s*;\s*"
    r"text\(\s*JSON\.stringify\(\s*(?P=var)\s*\)\s*\)\s*;?\s*\Z",
    re.DOTALL,
)


def _parse_action(payload: dict[str, Any]) -> dict[str, Any] | None:
    item_type = payload.get("type")
    if item_type == "function_call":
        name = payload.get("name")
        arguments = _parse_arguments(payload.get("arguments"))
        if not isinstance(name, str) or not name or arguments is None:
            return None
        return {
            "shape": "function_call",
            "function": name[:128],
            "arguments": arguments,
            "valid": True,
        }
    if item_type != "custom_tool_call":
        return None
    custom_tool = payload.get("name")
    source = payload.get("input")
    if not isinstance(custom_tool, str) or not custom_tool or not isinstance(source, str):
        return None
    parsed = _parse_custom_tool_program(source)
    if parsed is None:
        return {
            "shape": "custom_tool_call",
            "custom_tool": custom_tool[:128],
            "arguments": {},
            "valid": False,
        }
    return {
        "shape": "custom_tool_call",
        "custom_tool": custom_tool[:128],
        "inner_tool": parsed["inner_tool"],
        "arguments": parsed["arguments"],
        "projection": parsed["projection"],
        "valid": True,
    }


def _parse_custom_tool_program(source: str) -> dict[str, Any] | None:
    if len(source.encode("utf-8")) > 8_192 or "//" in source or "/*" in source:
        return None
    match = CUSTOM_TOOL_OUTPUT_CONTINUATION_PROGRAM.fullmatch(source)
    projection: dict[str, Any]
    if match is not None:
        projection = {
            "kind": "output_and_continuation",
            "output_field": match.group("output"),
            "continuation_field": match.group("continuation"),
            "continuation_prefix": match.group("prefix"),
        }
    else:
        match = CUSTOM_TOOL_JSON_RESULT_PROGRAM.fullmatch(source)
        projection = {"kind": "json_stringify_result"}
    if match is None:
        return None
    arguments = _parse_javascript_object(match.group("args"))
    if arguments is None:
        return None
    return {
        "inner_tool": match.group("inner"),
        "arguments": arguments,
        "projection": projection,
    }


def _parse_javascript_object(value: str) -> dict[str, Any] | None:
    if any(token in value for token in ("=>", "function", "await", "tools.", "`")):
        return None
    quoted = re.sub(
        r"([,{]\s*)([A-Za-z_][A-Za-z0-9_]*)\s*:",
        lambda match: f'{match.group(1)}"{match.group(2)}":',
        value,
    )
    try:
        parsed = json.loads(quoted)
    except json.JSONDecodeError:
        return None
    return parsed if isinstance(parsed, dict) else None


def _safe_action_arguments(action: dict[str, Any], matched_name: str) -> bool:
    if action.get("valid") is not True:
        return False
    arguments = action.get("arguments")
    if not isinstance(arguments, dict) or not (1 <= len(arguments) <= MAX_ARGUMENTS):
        return False
    if matched_name not in arguments:
        return False
    for name, value in arguments.items():
        if not isinstance(name, str) or not name or len(name) > 128:
            return False
        if name == matched_name:
            continue
        if isinstance(value, bool):
            continue
        if isinstance(value, int) and 0 <= value <= MAX_INTEGER_CONSTANT:
            continue
        if isinstance(value, str) and value == "":
            continue
        return False
    return True


def _constant_argument_atoms(
    arguments: dict[str, Any], matched_name: str
) -> list[dict[str, Any]]:
    atoms = []
    for name, value in arguments.items():
        if name == matched_name:
            continue
        if isinstance(value, bool):
            atoms.append({"kind": "action_boolean_argument", "name": name, "value": value})
        elif isinstance(value, int):
            atoms.append({"kind": "action_integer_argument", "name": name, "value": value})
        elif isinstance(value, str):
            atoms.append({"kind": "action_string_argument", "name": name, "value": value})
    return atoms


def _collect_scalars(value: Any, depth: int, scalars: list[tuple[Any, str]]) -> bool:
    if depth > MAX_DEPTH or len(scalars) >= MAX_SCALARS:
        return False
    descriptor = _scalar_descriptor(value)
    if descriptor is not None:
        scalars.append((value, descriptor[1]))
        return True
    if value is None:
        return True
    if isinstance(value, list):
        return all(_collect_scalars(item, depth + 1, scalars) for item in value)
    if isinstance(value, dict):
        return all(_collect_scalars(item, depth + 1, scalars) for item in value.values())
    return False


def _scalar_descriptor(value: Any) -> tuple[str, str] | None:
    if isinstance(value, bool):
        return sha256_value(value), "boolean"
    if isinstance(value, int):
        return sha256_value(value), "integer"
    if isinstance(value, float):
        return None
    if isinstance(value, str):
        value_type = "identifier" if _identifier_like(value) else "string"
        return sha256_value(value), value_type
    return None


def _observation_scalar_descriptor(value: Any) -> dict[str, str] | None:
    descriptor = _scalar_descriptor(value)
    if descriptor is None:
        return None
    observation = {
        "value_sha256": descriptor[0],
        "value_type": descriptor[1],
    }
    status_class = _bounded_integer_status_class(value)
    if status_class is not None:
        observation["integer_status_class"] = status_class
    return observation


def _bounded_integer_status_class(value: Any) -> str | None:
    if type(value) is not int or not (0 <= value <= MAX_PROJECT_STATUS_CODE):
        return None
    return "zero" if value == 0 else "nonzero"


def _parse_scalar_text(value: str) -> dict[str, str] | None:
    stripped = value.strip()
    if not stripped or len(stripped.encode("utf-8")) > 1_024:
        return None
    try:
        parsed = json.loads(stripped)
    except json.JSONDecodeError:
        parsed = stripped
    return _observation_scalar_descriptor(parsed)


def _output_text_parts(output: Any) -> list[str]:
    if isinstance(output, str):
        return [output]
    if not isinstance(output, list):
        return []
    return [
        item["text"]
        for item in output
        if isinstance(item, dict)
        and item.get("type") in {"text", "input_text", "output_text"}
        and isinstance(item.get("text"), str)
    ]


def _identifier_like(value: str) -> bool:
    return bool(value) and len(value.encode("utf-8")) <= 128 and all(
        character.isascii() and (character.isalnum() or character in "_-.:/")
        for character in value
    )


def _cardinality(role: str, count: int) -> dict[str, Any]:
    return {"kind": "cardinality", "role": role, "count": count}


def _valid_context_atoms(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list):
        return []
    atoms: list[dict[str, Any]] = []
    for atom in value[:8]:
        canonical = _canonical_context_atom(atom)
        if canonical is not None:
            atoms.append(canonical)
    return atoms


def _valid_observation(value: Any) -> bool:
    return (
        isinstance(value, dict)
        and _valid_sha256(value.get("value_sha256"))
        and value.get("value_type") in {"identifier", "string", "integer", "boolean"}
        and isinstance(value.get("tool_kind"), str)
        and bool(value.get("tool_kind"))
        and isinstance(value.get("call_shape"), str)
        and value.get("call_shape") in {"function_call", "custom_tool_call"}
        and value.get("completion_state") in {"pending", "completed"}
        and _valid_sha256(value.get("output_sha256"))
        and _valid_selector(value.get("selector"), value.get("value_type"))
        and _valid_integer_status_class(value)
    )


def _valid_integer_status_class(value: dict[str, Any]) -> bool:
    if "integer_status_class" not in value:
        return True
    status_class = value.get("integer_status_class")
    if value.get("value_type") != "integer" or status_class not in {"zero", "nonzero"}:
        return False
    zero_hash = sha256_value(0)
    return (status_class == "zero") == (value.get("value_sha256") == zero_hash)


def _valid_selector(value: Any, value_type: Any) -> bool:
    return _canonical_selector(value, value_type) is not None


def _canonical_selector(value: Any, value_type: Any) -> dict[str, str] | None:
    """Validate a selector and rebuild only its declared fields."""
    if value_type not in VALUE_TYPES or not isinstance(value, dict):
        return None
    if value.get("value_type") != value_type:
        return None
    kind = value.get("kind")
    if kind == "unique_scalar":
        return {"kind": kind, "value_type": value_type}
    if kind == "content_line_prefix":
        prefix = value.get("prefix")
        key = None
        if isinstance(prefix, str) and prefix.endswith("="):
            key = prefix[:-1]
        elif isinstance(prefix, str) and ":" in prefix:
            candidate, separator = prefix.split(":", 1)
            if not separator or separator.isspace():
                key = candidate
        if _safe_structural_identifier(key):
            return {"kind": kind, "prefix": prefix, "value_type": value_type}
        return None
    if kind in {
        "json_field",
        "unique_turn_json_field",
        "unique_active_turn_json_field",
    }:
        field = value.get("field")
        if _safe_structural_identifier(field):
            return {"kind": kind, "field": field, "value_type": value_type}
    if kind == "turn_output_line":
        output_ordinal = value.get("output_ordinal")
        line_index = value.get("line_index")
        if (
            value_type == "string"
            and type(output_ordinal) is int
            and 1 <= output_ordinal <= MAX_CALLS
            and type(line_index) is int
            and 0 <= line_index <= 255
        ):
            return {
                "kind": kind,
                "output_ordinal": output_ordinal,
                "line_index": line_index,
                "value_type": value_type,
            }
    return None


def _safe_structural_identifier(value: Any) -> bool:
    """Accept bounded source-neutral structure, never private-looking selectors."""
    if not isinstance(value, str) or not (1 <= len(value) <= 64):
        return False
    if not value.isascii() or re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", value) is None:
        return False
    if PRIVATE_IDENTIFIER_PART.search(value):
        return False
    digits = sum(character.isdigit() for character in value)
    if digits and digits * 4 > len(value):
        return False
    compact = value.replace("_", "")
    if len(compact) >= 16 and re.fullmatch(r"[0-9A-Fa-f]+", compact):
        return False
    if len(compact) >= 24 and re.fullmatch(r"[A-Za-z0-9+/=_-]+", compact):
        classes = sum(
            bool(re.search(pattern, compact))
            for pattern in (r"[a-z]", r"[A-Z]", r"[0-9]")
        )
        if classes >= 3:
            return False
    return True


def _normalize_stored_frame(value: Any) -> dict[str, Any] | None:
    if not isinstance(value, dict):
        return None
    try:
        encoded = canonical_bytes(value)
    except (TypeError, ValueError):
        return None
    if len(encoded) > MAX_STORED_FRAME_BYTES:
        return None
    if (
        value.get("schema") != RELATION_FRAME_SCHEMA
        or not _valid_sha256(value.get("frame_id_sha256"))
        or not _valid_sha256(value.get("event_id_sha256"))
        or not _valid_sha256(value.get("client_intent_id_sha256"))
        or not _valid_sha256(value.get("session_id_sha256"))
        or not _valid_sha256(value.get("evidence_ref_sha256"))
        or value.get("extractor_version") != RELATION_EXTRACTOR_VERSION
        or type(value.get("observed_at_unix_nanos")) is not int
        or type(value.get("verifier_label")) is not bool
        or not isinstance(value.get("atoms"), list)
        or len(value["atoms"]) > 64
    ):
        return None
    atoms: list[dict[str, Any]] = []
    for atom in value["atoms"]:
        canonical = _canonical_atom(atom)
        if canonical is None:
            return None
        atoms.append(canonical)
    return {
        "schema": RELATION_FRAME_SCHEMA,
        "frame_id_sha256": value["frame_id_sha256"],
        "event_id_sha256": value["event_id_sha256"],
        "client_intent_id_sha256": value["client_intent_id_sha256"],
        "session_id_sha256": value["session_id_sha256"],
        "observed_at_unix_nanos": value["observed_at_unix_nanos"],
        "extractor_version": RELATION_EXTRACTOR_VERSION,
        "estimated_input_tokens": _bounded_input_tokens(
            value.get("estimated_input_tokens")
        ),
        "verifier_label": value["verifier_label"],
        "atoms": atoms,
        "evidence_ref_sha256": value["evidence_ref_sha256"],
        **(
            {"cold_collection_example": value["cold_collection_example"]}
            if _valid_cold_collection_example(value.get("cold_collection_example"))
            else {}
        ),
    }


def _valid_cold_collection_example(value: Any) -> bool:
    if not isinstance(value, dict) or value.get("schema") != "nando.response-collection-synthesis-example.v1":
        return False
    payload = value.get("provider_payload")
    item = payload.get("input", [None])[-1] if isinstance(payload, dict) else None
    if _bounded_collection_json(item.get("output") if isinstance(item, dict) else None) is None:
        return False
    expected = value.get("expected_response")
    return isinstance(expected, str) and bool(expected) and len(expected.encode("utf-8")) <= MAX_OUTPUT_BYTES


def _compact_observation(observation: dict[str, Any]) -> dict[str, Any]:
    selector = _canonical_selector(observation["selector"], observation["value_type"])
    if selector is None:
        raise ValueError("invalid observation selector")
    compact = {
        "value_sha256": observation["value_sha256"],
        "value_type": observation["value_type"],
        "selector": selector,
        "tool_kind": observation["tool_kind"][:128],
        "call_shape": observation["call_shape"][:160],
        "completion_state": observation["completion_state"],
        "output_sha256": observation["output_sha256"],
        "context_atoms": _valid_context_atoms(observation.get("context_atoms")),
    }
    if "integer_status_class" in observation:
        compact["integer_status_class"] = observation["integer_status_class"]
    return compact


def _canonical_context_atom(value: Any) -> dict[str, Any] | None:
    if (
        not isinstance(value, dict)
        or value.get("kind") != "cardinality"
        or value.get("role") not in CONTEXT_ROLES
        or type(value.get("count")) is not int
        or value["count"] < 0
    ):
        return None
    return {"kind": "cardinality", "role": value["role"], "count": value["count"]}


def _bounded_text(value: Any, maximum: int, *, allow_empty: bool = False) -> str | None:
    if not isinstance(value, str) or (not allow_empty and not value):
        return None
    if len(value.encode("utf-8")) > maximum:
        return None
    return value


def _slot_id(value: Any) -> int | None:
    return value if type(value) is int and value in {1, 2} else None


def _canonical_atom(value: Any) -> dict[str, Any] | None:
    """Validate an emitted atom and rebuild only that atom's known fields."""
    if not isinstance(value, dict) or not isinstance(value.get("kind"), str):
        return None
    kind = value["kind"]

    if kind in {"tool_kind", "action_function", "action_custom_tool", "action_inner_tool"}:
        text = _bounded_text(value.get("value"), 128)
        return {"kind": kind, "value": text} if text is not None else None
    if kind == "observation_call_shape":
        call_shape = value.get("value")
        return (
            {"kind": kind, "value": call_shape}
            if call_shape in {"function_call", "custom_tool_call"}
            else None
        )
    if kind == "completion_state":
        completion_state = value.get("value")
        return (
            {"kind": kind, "value": completion_state}
            if completion_state in {"pending", "completed"}
            else None
        )
    if kind == "response_shape":
        response_shape = value.get("value")
        return (
            {"kind": kind, "value": response_shape}
            if response_shape in {"function_call", "custom_tool_call", "assistant_message"}
            else None
        )
    if kind == "typed_slot":
        slot_id = _slot_id(value.get("slot_id"))
        source = value.get("source")
        value_type = value.get("value_type")
        value_sha256 = value.get("value_sha256")
        if (
            slot_id is None
            or source not in {"observation", "action"}
            or value_type not in VALUE_TYPES
            or not _valid_sha256(value_sha256)
        ):
            return None
        return {
            "kind": kind,
            "slot_id": slot_id,
            "value_type": value_type,
            "source": source,
            "value_sha256": value_sha256,
        }
    if kind == "unique_slot":
        slot_id = _slot_id(value.get("slot_id"))
        return {"kind": kind, "slot_id": slot_id} if slot_id is not None else None
    if kind == "observation_selector":
        slot_id = _slot_id(value.get("slot_id"))
        selector = value.get("selector")
        value_type = selector.get("value_type") if isinstance(selector, dict) else None
        canonical_selector = _canonical_selector(selector, value_type)
        if slot_id is None or canonical_selector is None:
            return None
        return {"kind": kind, "slot_id": slot_id, "selector": canonical_selector}
    if kind == "cardinality":
        return _canonical_context_atom(value)
    if kind == "action_json_result_projection":
        return {"kind": kind}
    if kind == "action_result_projection":
        output_field = _bounded_text(value.get("output_field"), 128)
        continuation_field = _bounded_text(value.get("continuation_field"), 128)
        continuation_prefix = _bounded_text(value.get("continuation_prefix"), 128)
        if output_field is None or continuation_field is None or continuation_prefix is None:
            return None
        return {
            "kind": kind,
            "output_field": output_field,
            "continuation_field": continuation_field,
            "continuation_prefix": continuation_prefix,
        }
    if kind == "action_value_projection":
        projection_format = value.get("format")
        if projection_format not in {"plain_text", "canonical_json"}:
            return None
        output = {"kind": kind, "format": projection_format}
        renderer = value.get("renderer")
        if renderer is not None:
            if not isinstance(renderer, dict):
                return None
            if renderer.get("kind") == "render_template":
                prefix = renderer.get("prefix")
                suffix = renderer.get("suffix")
                if not isinstance(prefix, str) or not isinstance(suffix, str):
                    return None
                if len((prefix + suffix).encode("utf-8")) > 512 or not _safe_collection_template(prefix + suffix):
                    return None
                output["renderer"] = {"kind": "render_template", "prefix": prefix, "suffix": suffix}
            elif renderer.get("kind") == "render_sequence":
                segments = renderer.get("segments")
                if not isinstance(segments, list) or not 3 <= len(segments) <= 16:
                    return None
                canonical_segments = []
                static_text = ""
                primary_count = 0
                selected_count = 0
                previous_static = False
                for segment in segments:
                    if not isinstance(segment, dict):
                        return None
                    segment_kind = segment.get("kind")
                    if segment_kind == "static":
                        text = segment.get("text")
                        if not isinstance(text, str) or not text or previous_static:
                            return None
                        canonical_segments.append({"kind": "static", "text": text})
                        static_text += text
                        previous_static = True
                    elif segment_kind == "primary":
                        canonical_segments.append({"kind": "primary"})
                        primary_count += 1
                        previous_static = False
                    elif segment_kind == "selected":
                        selected_format = segment.get("format")
                        selector = segment.get("selector")
                        value_type = selector.get("value_type") if isinstance(selector, dict) else None
                        canonical_selector = _canonical_selector(selector, value_type)
                        if selected_format not in {"plain_text", "canonical_json"} or canonical_selector is None:
                            return None
                        canonical_segments.append({
                            "kind": "selected",
                            "selector": canonical_selector,
                            "format": selected_format,
                        })
                        selected_count += 1
                        previous_static = False
                    else:
                        return None
                if primary_count != 1 or selected_count == 0:
                    return None
                if len(static_text.encode("utf-8")) > 512 or not _safe_collection_template(static_text):
                    return None
                output["renderer"] = {"kind": "render_sequence", "segments": canonical_segments}
            else:
                return None
        return output
    if kind == "action_status_projection":
        return (
            {"kind": kind, "mapping": "zero_is_success"}
            if value.get("mapping") == "zero_is_success"
            else None
        )
    if kind == "collection_shape":
        array_fields = value.get("array_fields")
        row_fields = value.get("row_fields")
        if type(array_fields) is int and type(row_fields) is int and 1 <= array_fields <= 16 and 1 <= row_fields <= 16:
            return {"kind": kind, "array_fields": array_fields, "row_fields": row_fields}
        return None
    if kind == "request_phase_atom":
        atom_id = value.get("atom_id")
        return {"kind": kind, "atom_id": atom_id} if type(atom_id) is int and 0 < atom_id < (1 << 64) else None
    if kind == "action_role_argument":
        name = _bounded_text(value.get("name"), 128)
        slot_id = _slot_id(value.get("slot_id"))
        return (
            {"kind": kind, "name": name, "slot_id": slot_id}
            if name is not None and slot_id == 2
            else None
        )
    if kind == "slot_equality":
        left_slot = _slot_id(value.get("left_slot"))
        right_slot = _slot_id(value.get("right_slot"))
        return (
            {"kind": kind, "left_slot": left_slot, "right_slot": right_slot}
            if left_slot == 1 and right_slot == 2
            else None
        )
    if kind == "action_boolean_argument":
        name = _bounded_text(value.get("name"), 128)
        argument_value = value.get("value")
        return (
            {"kind": kind, "name": name, "value": argument_value}
            if name is not None and type(argument_value) is bool
            else None
        )
    if kind == "action_integer_argument":
        name = _bounded_text(value.get("name"), 128)
        argument_value = value.get("value")
        return (
            {"kind": kind, "name": name, "value": argument_value}
            if (
                name is not None
                and type(argument_value) is int
                and 0 <= argument_value <= MAX_INTEGER_CONSTANT
            )
            else None
        )
    if kind == "action_string_argument":
        name = _bounded_text(value.get("name"), 128)
        argument_value = _bounded_text(value.get("value"), 0, allow_empty=True)
        return (
            {"kind": kind, "name": name, "value": argument_value}
            if name is not None and argument_value == ""
            else None
        )
    return None


def _valid_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdefABCDEF" for character in value)
    )


def _iso_to_unix_nanos(value: str) -> int:
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return 0
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return int(parsed.timestamp() * 1_000_000_000)
