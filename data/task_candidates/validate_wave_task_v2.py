#!/usr/bin/env python3
"""Strict lightweight validator for wave_task_v2 JSONL files."""

from __future__ import annotations

import json
import sys
from pathlib import Path


REQUIRED_FIELDS = [
    "schema_version",
    "task_id",
    "language",
    "task_kind",
    "domain_path",
    "domain_tags",
    "source_family",
    "source_group",
    "input",
    "target",
    "near_negative",
    "operator_family",
    "why_target_is_correct",
    "why_negative_is_wrong",
    "shortcut_risk",
    "quality_status",
]

ALLOWED_STATUS = {"candidate", "reference", "accepted", "rejected"}
FORBIDDEN_FIELDS = {"hidden_operator"}


def non_empty_string(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def non_empty_string_list(value: object) -> bool:
    return (
        isinstance(value, list)
        and bool(value)
        and all(non_empty_string(item) for item in value)
    )


def validate_row(row: object, line_number: int, seen_task_ids: set[str]) -> list[str]:
    if not isinstance(row, dict):
        return [f"line {line_number}: row is not an object"]

    errors: list[str] = []
    missing = [field for field in REQUIRED_FIELDS if field not in row]
    if missing:
        errors.append(f"line {line_number}: missing fields: {', '.join(missing)}")

    extras = sorted(set(row) - set(REQUIRED_FIELDS))
    if extras:
        errors.append(f"line {line_number}: unknown fields: {', '.join(extras)}")

    forbidden = sorted(set(row) & FORBIDDEN_FIELDS)
    if forbidden:
        errors.append(f"line {line_number}: forbidden fields: {', '.join(forbidden)}")

    if row.get("schema_version") != "wave_task_v2":
        errors.append(f"line {line_number}: schema_version must be wave_task_v2")

    for field in REQUIRED_FIELDS:
        if field in {"domain_tags", "shortcut_risk"} or field not in row:
            continue
        if not non_empty_string(row[field]):
            errors.append(f"line {line_number}: {field} must be a non-empty string")

    if "domain_tags" in row and not non_empty_string_list(row["domain_tags"]):
        errors.append(f"line {line_number}: domain_tags must be a non-empty string array")

    if "shortcut_risk" in row and not non_empty_string_list(row["shortcut_risk"]):
        errors.append(f"line {line_number}: shortcut_risk must be a non-empty string array")

    task_id = row.get("task_id")
    if isinstance(task_id, str):
        if task_id in seen_task_ids:
            errors.append(f"line {line_number}: duplicate task_id: {task_id}")
        seen_task_ids.add(task_id)

    if row.get("quality_status") not in ALLOWED_STATUS:
        errors.append(
            f"line {line_number}: quality_status must be one of "
            f"{', '.join(sorted(ALLOWED_STATUS))}"
        )

    domain_path = row.get("domain_path")
    if isinstance(domain_path, str) and "." not in domain_path:
        errors.append(f"line {line_number}: domain_path must be hierarchical")

    return errors


def validate_file(path: Path) -> int:
    errors: list[str] = []
    seen_task_ids: set[str] = set()
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
            errors.extend(validate_row(row, line_number, seen_task_ids))

    if rows == 0:
        errors.append("file has no JSONL rows")

    if errors:
        print(f"wave_task_v2 validation FAILED: {path}")
        for error in errors:
            print(error)
        return 1

    print(f"wave_task_v2 validation OK: {path} rows={rows}")
    return 0


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: validate_wave_task_v2.py <file.jsonl> [...]", file=sys.stderr)
        return 2

    status = 0
    for arg in sys.argv[1:]:
        status |= validate_file(Path(arg))
    return status


if __name__ == "__main__":
    raise SystemExit(main())
