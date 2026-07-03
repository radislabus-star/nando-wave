#!/usr/bin/env python3
"""Validate rule_task_v1 JSONL rows."""

from __future__ import annotations

import json
import sys
from pathlib import Path


REQUIRED_FIELDS = [
    "schema_version",
    "task_id",
    "language",
    "surface_family",
    "source_group",
    "input",
    "target",
    "near_negative",
    "answer_status",
    "proof_rule_id",
    "proof_rule_family",
    "why_target_is_correct",
    "why_negative_is_wrong",
    "shortcut_risk",
    "quality_status",
]

ALLOWED_STATUSES = {"PROVEN", "LIKELY", "UNSETTLED", "CONFLICT"}
ALLOWED_QUALITY = {"candidate", "accepted", "rejected"}


def non_empty_string(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def non_empty_string_list(value: object) -> bool:
    return (
        isinstance(value, list)
        and bool(value)
        and all(non_empty_string(item) for item in value)
    )


def validate_row(row: object, line_number: int, seen_ids: set[str]) -> list[str]:
    if not isinstance(row, dict):
        return [f"line {line_number}: row is not an object"]

    errors: list[str] = []
    missing = [field for field in REQUIRED_FIELDS if field not in row]
    if missing:
        errors.append(f"line {line_number}: missing fields: {', '.join(missing)}")

    extras = sorted(set(row) - set(REQUIRED_FIELDS))
    if extras:
        errors.append(f"line {line_number}: unknown fields: {', '.join(extras)}")

    if row.get("schema_version") != "rule_task_v1":
        errors.append(f"line {line_number}: schema_version must be rule_task_v1")

    for field in REQUIRED_FIELDS:
        if field == "shortcut_risk" or field not in row:
            continue
        if not non_empty_string(row[field]):
            errors.append(f"line {line_number}: {field} must be a non-empty string")

    if row.get("answer_status") not in ALLOWED_STATUSES:
        errors.append(f"line {line_number}: invalid answer_status")
    if row.get("quality_status") not in ALLOWED_QUALITY:
        errors.append(f"line {line_number}: invalid quality_status")
    if "shortcut_risk" in row and not non_empty_string_list(row["shortcut_risk"]):
        errors.append(f"line {line_number}: shortcut_risk must be a non-empty string array")

    task_id = row.get("task_id")
    if isinstance(task_id, str):
        if task_id in seen_ids:
            errors.append(f"line {line_number}: duplicate task_id: {task_id}")
        seen_ids.add(task_id)

    if row.get("target") == row.get("near_negative"):
        errors.append(f"line {line_number}: target equals near_negative")

    return errors


def validate_file(path: Path) -> int:
    errors: list[str] = []
    seen_ids: set[str] = set()
    rows = 0

    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, 1):
            if not line.strip():
                continue
            rows += 1
            try:
                row = json.loads(line)
            except json.JSONDecodeError as error:
                errors.append(f"line {line_number}: invalid JSON: {error}")
                continue
            errors.extend(validate_row(row, line_number, seen_ids))

    if rows == 0:
        errors.append("file has no JSONL rows")

    if errors:
        print(f"rule_task_v1 validation FAILED: {path}")
        for error in errors:
            print(error)
        return 1

    print(f"rule_task_v1 validation OK: {path} rows={rows}")
    return 0


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: validate_rule_tasks.py <file.jsonl> [...]", file=sys.stderr)
        return 2

    status = 0
    for arg in sys.argv[1:]:
        status |= validate_file(Path(arg))
    return status


if __name__ == "__main__":
    raise SystemExit(main())
