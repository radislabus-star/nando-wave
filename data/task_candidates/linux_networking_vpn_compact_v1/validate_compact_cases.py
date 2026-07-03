#!/usr/bin/env python3
"""Validate compact task cases against the locked Domain DSL pack."""

from __future__ import annotations

import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
DOMAIN_ROOT = REPO / "data" / "domain_dsl_v1"
COMPACT_PATH = ROOT / "compact_cases.json"


def load_json(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise ValueError(f"{path}: top-level JSON must be an object")
    return value


def non_empty_string(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def main() -> int:
    errors: list[str] = []
    compact = load_json(COMPACT_PATH)
    domains = load_json(DOMAIN_ROOT / "domains.json")

    if compact.get("schema_version") != "wave_compact_task_v1":
        errors.append("compact_cases.json: schema_version must be wave_compact_task_v1")

    domain_id = compact.get("domain_id")
    domain = next((row for row in domains.get("domains", []) if row.get("domain_id") == domain_id), None)
    if not isinstance(domain, dict):
        errors.append(f"compact_cases.json: unknown domain_id {domain_id}")
        domain = {}

    if domain.get("status") != "locked_first_proving_domain":
        errors.append(f"{domain_id}: domain must be locked_first_proving_domain")
    if domain.get("lock_contract", {}).get("step_5_full_pack_ready") is not True:
        errors.append(f"{domain_id}: step_5_full_pack_ready must be true")

    files = domain.get("files", {})
    templates_doc = load_json(DOMAIN_ROOT / files.get("templates", "missing"))
    operators_doc = load_json(DOMAIN_ROOT / files.get("operators", "missing"))
    negative_doc = load_json(DOMAIN_ROOT / files.get("negative_rules", "missing"))

    template_ids = {row.get("template_id") for row in templates_doc.get("templates", [])}
    operator_ids = {row.get("operator_family") for row in operators_doc.get("operators", [])}
    negative_ids = {row.get("negative_rule_id") for row in negative_doc.get("negative_rules", [])}
    template_by_id = {row.get("template_id"): row for row in templates_doc.get("templates", [])}

    cases = compact.get("cases")
    if not isinstance(cases, list) or not cases:
        errors.append("compact_cases.json: cases must be a non-empty list")
        cases = []

    seen_case_ids: set[str] = set()
    covered_templates: set[str] = set()
    for index, row in enumerate(cases, 1):
        if not isinstance(row, dict):
            errors.append(f"case[{index}]: must be an object")
            continue
        for key in ["case_id", "template_id", "language", "source_group", "input", "target", "near_negative"]:
            if not non_empty_string(row.get(key)):
                errors.append(f"case[{index}]: {key} must be a non-empty string")
        case_id = row.get("case_id")
        if isinstance(case_id, str):
            if case_id in seen_case_ids:
                errors.append(f"case[{index}]: duplicate case_id {case_id}")
            seen_case_ids.add(case_id)
        template_id = row.get("template_id")
        if template_id not in template_ids:
            errors.append(f"case[{index}]: unknown template_id {template_id}")
            continue
        covered_templates.add(template_id)
        template = template_by_id[template_id]
        operator = template.get("operator_family")
        if operator not in operator_ids:
            errors.append(f"case[{index}]: template operator is unknown {operator}")
        for negative_rule_id in template.get("negative_rule_ids", []):
            if negative_rule_id not in negative_ids:
                errors.append(f"case[{index}]: template negative rule is unknown {negative_rule_id}")
        if row.get("target") == row.get("near_negative"):
            errors.append(f"case[{index}]: target and near_negative must differ")
        if row.get("target") and row.get("target") in str(row.get("input", "")):
            errors.append(f"case[{index}]: target leaks into input")

    missing_template_cases = sorted(template_ids - covered_templates)
    if missing_template_cases:
        errors.append("templates without compact case: " + ", ".join(missing_template_cases))

    if errors:
        print("compact task validation FAILED")
        for error in errors:
            print(error)
        return 1

    print(
        "compact task validation OK: "
        f"domain={domain_id} cases={len(cases)} templates_covered={len(covered_templates)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

