#!/usr/bin/env python3
"""Validate rule_action_trace_v1 JSONL rows."""

from __future__ import annotations

import json
import sys
from pathlib import Path


REQUIRED = [
    "schema_version",
    "task_id",
    "language",
    "surface_family",
    "source_group",
    "state_before",
    "rule_action_example",
    "state_after_correct",
    "state_after_wrong",
    "answer_status",
    "proof_rule_id",
    "proof_rule_family",
    "why_target_is_correct",
    "why_negative_is_wrong",
    "shortcut_risk",
    "quality_status",
]
STATUSES = {"PROVEN", "LIKELY", "UNSETTLED", "CONFLICT"}


def non_empty_string(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def validate_row(row: object, line_no: int, seen: set[str]) -> list[str]:
    if not isinstance(row, dict):
        return [f"line {line_no}: row is not object"]
    errors = []
    missing = [field for field in REQUIRED if field not in row]
    if missing:
        errors.append(f"line {line_no}: missing fields: {', '.join(missing)}")
    extras = sorted(set(row) - set(REQUIRED))
    if extras:
        errors.append(f"line {line_no}: unknown fields: {', '.join(extras)}")
    if row.get("schema_version") != "rule_action_trace_v1":
        errors.append(f"line {line_no}: schema_version must be rule_action_trace_v1")
    for field in REQUIRED:
        if field == "shortcut_risk" or field not in row:
            continue
        if not non_empty_string(row[field]):
            errors.append(f"line {line_no}: {field} must be non-empty string")
    if not isinstance(row.get("shortcut_risk"), list) or not row.get("shortcut_risk"):
        errors.append(f"line {line_no}: shortcut_risk must be non-empty list")
    if row.get("answer_status") not in STATUSES:
        errors.append(f"line {line_no}: invalid answer_status")
    if row.get("quality_status") != "accepted":
        errors.append(f"line {line_no}: quality_status must be accepted")
    task_id = row.get("task_id")
    if isinstance(task_id, str):
        if task_id in seen:
            errors.append(f"line {line_no}: duplicate task_id: {task_id}")
        seen.add(task_id)
    if row.get("state_after_correct") == row.get("state_after_wrong"):
        errors.append(f"line {line_no}: correct state equals wrong state")
    return errors


def validate(path: Path) -> int:
    errors = []
    seen: set[str] = set()
    rows = 0
    with path.open("r", encoding="utf-8") as handle:
        for line_no, line in enumerate(handle, 1):
            if not line.strip():
                continue
            rows += 1
            try:
                row = json.loads(line)
            except json.JSONDecodeError as err:
                errors.append(f"line {line_no}: invalid JSON: {err}")
                continue
            errors.extend(validate_row(row, line_no, seen))
    if rows == 0:
        errors.append("file has no JSONL rows")
    if errors:
        print(f"rule_action_trace validation FAILED: {path}")
        for error in errors:
            print(error)
        return 1
    print(f"rule_action_trace validation OK: {path} rows={rows}")
    return 0


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: validate_action_trace_tasks.py <file.jsonl> [...]", file=sys.stderr)
        return 2
    status = 0
    for arg in sys.argv[1:]:
        status |= validate(Path(arg))
    return status


if __name__ == "__main__":
    raise SystemExit(main())
