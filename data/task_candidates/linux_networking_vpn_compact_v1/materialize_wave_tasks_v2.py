#!/usr/bin/env python3
"""Materialize compact cases into strict wave_task_v2 JSONL candidates."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
DOMAIN_ROOT = REPO / "data" / "domain_dsl_v1"
COMPACT_PATH = ROOT / "compact_cases.json"
OUT_JSONL = ROOT / "generated_wave_task_v2.jsonl"
OUT_MANIFEST = ROOT / "manifest.json"


def load_json(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise ValueError(f"{path}: top-level JSON must be an object")
    return value


def unique_list(values: list[str]) -> list[str]:
    seen: set[str] = set()
    result: list[str] = []
    for value in values:
        if value not in seen:
            result.append(value)
            seen.add(value)
    return result


def main() -> int:
    compact = load_json(COMPACT_PATH)
    domains_doc = load_json(DOMAIN_ROOT / "domains.json")
    domain_id = compact["domain_id"]
    domain = next(row for row in domains_doc["domains"] if row["domain_id"] == domain_id)

    files = domain["files"]
    templates = load_json(DOMAIN_ROOT / files["templates"])["templates"]
    operators = load_json(DOMAIN_ROOT / files["operators"])["operators"]
    negative_rules = load_json(DOMAIN_ROOT / files["negative_rules"])["negative_rules"]

    template_by_id = {row["template_id"]: row for row in templates}
    operator_by_family = {row["operator_family"]: row for row in operators}
    negative_by_id = {row["negative_rule_id"]: row for row in negative_rules}

    rows: list[dict] = []
    for index, case in enumerate(compact["cases"], 1):
        template = template_by_id[case["template_id"]]
        operator = operator_by_family[template["operator_family"]]
        first_negative = negative_by_id[template["negative_rule_ids"][0]]
        task_id = f"lnvpn_compact_v1_{index:06d}"
        domain_tags = unique_list(domain["tags"] + template["constraint_tags"])
        shortcut_risk = unique_list(
            [
                "generated_from_domain_dsl",
                "template_repetition",
                "near_negative_must_survive_l2_neighbor",
            ]
            + template["constraint_tags"][:2]
        )
        rows.append(
            {
                "schema_version": "wave_task_v2",
                "task_id": task_id,
                "language": case["language"],
                "task_kind": template["task_kind"],
                "domain_path": domain["domain_path"],
                "domain_tags": domain_tags,
                "source_family": compact["source_family"],
                "source_group": case["source_group"],
                "input": case["input"],
                "target": case["target"],
                "near_negative": case["near_negative"],
                "operator_family": operator["operator_family"],
                "why_target_is_correct": (
                    "Цель выбирает правильный следующий ход по оператору "
                    f"{operator['operator_family']}: {case['target']}"
                ),
                "why_negative_is_wrong": (
                    "Near-negative близок по теме, но нарушает правило "
                    f"{first_negative['negative_rule_id']}: {case['near_negative']}"
                ),
                "shortcut_risk": shortcut_risk,
                "quality_status": compact["quality_status"],
            }
        )

    with OUT_JSONL.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")

    manifest = {
        "schema_version": "wave_compact_task_manifest_v1",
        "domain_id": domain_id,
        "source_file": str(COMPACT_PATH.relative_to(REPO)),
        "materialized_file": str(OUT_JSONL.relative_to(REPO)),
        "rows": len(rows),
        "quality_status": compact["quality_status"],
        "accepted_training_tasks": 0,
        "boundary": {
            "compact_cases_are_candidate_only": True,
            "not_runtime_authority": True,
            "shortcut_gates_pending": True,
            "step_7_required_before_acceptance": True,
        },
    }
    with OUT_MANIFEST.open("w", encoding="utf-8") as handle:
        json.dump(manifest, handle, ensure_ascii=False, indent=2)
        handle.write("\n")

    print(f"materialized wave_task_v2 rows={len(rows)} -> {OUT_JSONL}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
