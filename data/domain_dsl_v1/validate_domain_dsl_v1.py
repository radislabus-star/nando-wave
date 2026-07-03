#!/usr/bin/env python3
"""Validate Domain DSL v1 registry and domain component references."""

from __future__ import annotations

import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent
DOMAINS_PATH = ROOT / "domains.json"


def load_json(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise ValueError(f"{path}: top-level JSON must be an object")
    return value


def unique_ids(rows: list[dict], key: str, label: str) -> tuple[set[str], list[str]]:
    ids: set[str] = set()
    errors: list[str] = []
    for index, row in enumerate(rows, 1):
        value = row.get(key)
        if not isinstance(value, str) or not value:
            errors.append(f"{label}[{index}]: missing {key}")
            continue
        if value in ids:
            errors.append(f"{label}[{index}]: duplicate {key}: {value}")
        ids.add(value)
    return ids, errors


def require_list(row: dict, key: str, label: str, errors: list[str]) -> list:
    value = row.get(key)
    if not isinstance(value, list) or not value:
        errors.append(f"{label}: {key} must be a non-empty list")
        return []
    return value


def validate_domain(domain: dict) -> list[str]:
    errors: list[str] = []
    domain_id = domain.get("domain_id")
    if not isinstance(domain_id, str) or not domain_id:
        return ["domain registry entry missing domain_id"]

    files = domain.get("files")
    if not isinstance(files, dict):
        return [f"{domain_id}: files must be an object"]

    required_files = ["domain_lock", "entities", "operators", "templates", "negative_rules"]
    missing = [key for key in required_files if key not in files]
    if missing:
        errors.append(f"{domain_id}: missing files: {', '.join(missing)}")
        return errors

    domain_lock_doc = load_json(ROOT / files["domain_lock"])
    entities_doc = load_json(ROOT / files["entities"])
    operators_doc = load_json(ROOT / files["operators"])
    templates_doc = load_json(ROOT / files["templates"])
    negative_rules_doc = load_json(ROOT / files["negative_rules"])

    for doc_name, doc in [
        ("domain_lock", domain_lock_doc),
        ("entities", entities_doc),
        ("operators", operators_doc),
        ("templates", templates_doc),
        ("negative_rules", negative_rules_doc),
    ]:
        if doc.get("schema_version") != "domain_dsl_v1":
            errors.append(f"{domain_id}/{doc_name}: schema_version must be domain_dsl_v1")
        if doc.get("domain_id") != domain_id:
            errors.append(f"{domain_id}/{doc_name}: domain_id mismatch")

    claim_boundary = domain.get("claim_boundary")
    if not isinstance(claim_boundary, dict):
        errors.append(f"{domain_id}: claim_boundary must be an object")
        claim_boundary = {}

    if domain.get("status") == "locked_first_proving_domain":
        if claim_boundary.get("domain_locked") is not True:
            errors.append(f"{domain_id}: locked domain must set claim_boundary.domain_locked=true")
        lock_contract = domain.get("lock_contract")
        if not isinstance(lock_contract, dict):
            errors.append(f"{domain_id}: locked domain must define lock_contract")
            lock_contract = {}
        if lock_contract.get("locked") is not True:
            errors.append(f"{domain_id}: lock_contract.locked must be true")
        if lock_contract.get("lock_scope") != "first_proving_domain":
            errors.append(f"{domain_id}: lock_contract.lock_scope must be first_proving_domain")
        if domain_lock_doc.get("lock_status") != "locked":
            errors.append(f"{domain_id}/domain_lock: lock_status must be locked")
        if domain_lock_doc.get("lock_scope") != "first_proving_domain":
            errors.append(f"{domain_id}/domain_lock: lock_scope must be first_proving_domain")
        if not domain_lock_doc.get("included_boundaries"):
            errors.append(f"{domain_id}/domain_lock: included_boundaries must be non-empty")
        if not domain_lock_doc.get("excluded_boundaries"):
            errors.append(f"{domain_id}/domain_lock: excluded_boundaries must be non-empty")
        task_scope = domain_lock_doc.get("task_scope")
        if not isinstance(task_scope, dict):
            errors.append(f"{domain_id}/domain_lock: task_scope must be an object")
            task_scope = {}
        if not task_scope.get("allowed_task_shapes"):
            errors.append(f"{domain_id}/domain_lock: allowed_task_shapes must be non-empty")
        if not task_scope.get("required_shortcut_gates"):
            errors.append(f"{domain_id}/domain_lock: required_shortcut_gates must be non-empty")
        if domain_lock_doc.get("claim_boundary", {}).get("runtime_authority") is not False:
            errors.append(f"{domain_id}/domain_lock: runtime_authority must remain false")
        if domain_lock_doc.get("claim_boundary", {}).get("training_dataset_ready") is not False:
            errors.append(f"{domain_id}/domain_lock: training_dataset_ready must remain false")
        step_5_ready = lock_contract.get("step_5_full_pack_ready") is True
        if domain_lock_doc.get("claim_boundary", {}).get("step_5_full_pack_ready") != step_5_ready:
            errors.append(f"{domain_id}/domain_lock: step_5_full_pack_ready mismatch")

    entities = require_list(entities_doc, "entities", f"{domain_id}/entities", errors)
    operators = require_list(operators_doc, "operators", f"{domain_id}/operators", errors)
    templates = require_list(templates_doc, "templates", f"{domain_id}/templates", errors)
    negative_rules = require_list(
        negative_rules_doc, "negative_rules", f"{domain_id}/negative_rules", errors
    )

    entity_ids, entity_errors = unique_ids(entities, "entity_id", f"{domain_id}/entities")
    operator_families, operator_errors = unique_ids(
        operators, "operator_family", f"{domain_id}/operators"
    )
    template_ids, template_errors = unique_ids(templates, "template_id", f"{domain_id}/templates")
    negative_rule_ids, negative_errors = unique_ids(
        negative_rules, "negative_rule_id", f"{domain_id}/negative_rules"
    )
    errors.extend(entity_errors + operator_errors + template_errors + negative_errors)

    for row in entities:
        label = f"{domain_id}/entity:{row.get('entity_id', '?')}"
        for key in ["surface_ru", "surface_en", "tags"]:
            require_list(row, key, label, errors)
        if not isinstance(row.get("entity_type"), str) or not row.get("entity_type"):
            errors.append(f"{label}: entity_type missing")

    for row in operators:
        label = f"{domain_id}/operator:{row.get('operator_family', '?')}"
        for key in ["operator_kind", "input_signal", "target_rule", "anti_rule"]:
            if not isinstance(row.get(key), str) or not row.get(key):
                errors.append(f"{label}: {key} missing")
        require_list(row, "evidence_need", label, errors)

    for row in negative_rules:
        label = f"{domain_id}/negative_rule:{row.get('negative_rule_id', '?')}"
        for operator_family in require_list(row, "target_operator_families", label, errors):
            if operator_family not in operator_families:
                errors.append(f"{label}: unknown target_operator_family {operator_family}")
        for entity_id in row.get("must_share_entities", []):
            if entity_id not in entity_ids:
                errors.append(f"{label}: unknown must_share_entity {entity_id}")
        for key in ["negative_type", "description", "must_flip"]:
            if not isinstance(row.get(key), str) or not row.get(key):
                errors.append(f"{label}: {key} missing")

    for row in templates:
        label = f"{domain_id}/template:{row.get('template_id', '?')}"
        operator_family = row.get("operator_family")
        if operator_family not in operator_families:
            errors.append(f"{label}: unknown operator_family {operator_family}")
        for entity_id in require_list(row, "required_entity_ids", label, errors):
            if entity_id not in entity_ids:
                errors.append(f"{label}: unknown required_entity_id {entity_id}")
        for negative_rule_id in require_list(row, "negative_rule_ids", label, errors):
            if negative_rule_id not in negative_rule_ids:
                errors.append(f"{label}: unknown negative_rule_id {negative_rule_id}")
        for key in ["task_kind", "input_pattern", "target_pattern", "near_negative_pattern"]:
            if not isinstance(row.get(key), str) or not row.get(key):
                errors.append(f"{label}: {key} missing")
        require_list(row, "constraint_tags", label, errors)

    if not template_ids:
        errors.append(f"{domain_id}: no templates validated")

    if domain.get("lock_contract", {}).get("step_5_full_pack_ready") is True:
        if len(entity_ids) < 48:
            errors.append(f"{domain_id}: full pack requires at least 48 entities")
        if len(operator_families) < 24:
            errors.append(f"{domain_id}: full pack requires at least 24 operators")
        if len(negative_rule_ids) < 24:
            errors.append(f"{domain_id}: full pack requires at least 24 negative_rules")
        if len(template_ids) < 24:
            errors.append(f"{domain_id}: full pack requires at least 24 templates")
        covered_operator_families = {
            row.get("operator_family")
            for row in templates
            if isinstance(row.get("operator_family"), str)
        }
        missing_template_coverage = sorted(operator_families - covered_operator_families)
        if missing_template_coverage:
            errors.append(
                f"{domain_id}: operators without template coverage: "
                + ", ".join(missing_template_coverage)
            )

    return errors


def main() -> int:
    errors: list[str] = []
    domains_doc = load_json(DOMAINS_PATH)
    if domains_doc.get("schema_version") != "domain_dsl_v1":
        errors.append("domains.json: schema_version must be domain_dsl_v1")

    domains = domains_doc.get("domains")
    if not isinstance(domains, list) or not domains:
        errors.append("domains.json: domains must be a non-empty list")
    else:
        domain_ids, domain_errors = unique_ids(domains, "domain_id", "domains")
        errors.extend(domain_errors)
        for domain in domains:
            errors.extend(validate_domain(domain))
        if not domain_ids:
            errors.append("domains.json: no valid domain ids")

    if errors:
        print("domain_dsl_v1 validation FAILED")
        for error in errors:
            print(error)
        return 1

    print(f"domain_dsl_v1 validation OK: domains={len(domains)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
