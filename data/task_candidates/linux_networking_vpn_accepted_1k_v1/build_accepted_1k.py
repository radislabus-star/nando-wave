#!/usr/bin/env python3
"""Build a 1k accepted Wave Task V2 corpus from the compact VPN task cases."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
COMPACT_ROOT = REPO / "data" / "task_candidates" / "linux_networking_vpn_compact_v1"
DOMAIN_ROOT = REPO / "data" / "domain_dsl_v1"
OUT_JSONL = ROOT / "accepted_wave_task_v2.jsonl"
OUT_MANIFEST = ROOT / "manifest.json"

TARGET_ROWS = 1_000
VARIANTS_PER_CASE = 42

INPUT_VARIANTS = [
    ("смена А", "снимок до изменения есть частично", "контрольный ping/curl повторен дважды"),
    ("смена B", "оператор получил свежий лог", "проверка выполнена с клиента и сервера"),
    ("ночной инцидент", "изменения запрещены без baseline", "есть один успешный и один отрицательный probe"),
    ("утренний rollout", "пользователи уже в сети", "нужен минимальный безопасный следующий шаг"),
    ("проверка после миграции", "старый профиль еще доступен", "нужно не смешать route и auth слой"),
    ("дежурный канал", "симптом повторяется на двух клиентах", "требуется отделить evidence от догадки"),
    ("проверка филиала", "затронут только один сегмент", "нужно сохранить scope исправления"),
]

SCOPES = [
    "peer alpha",
    "peer beta",
    "office subnet",
    "branch subnet",
    "user session",
    "server side",
    "client side",
    "rollout window",
    "incident window",
    "post-change check",
]


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


def variant_input(base_input: str, variant_index: int, case_id: str) -> str:
    shift, condition, probe = INPUT_VARIANTS[variant_index % len(INPUT_VARIANTS)]
    scope = SCOPES[variant_index % len(SCOPES)]
    ticket = f"T{variant_index:03d}-{case_id[-3:]}"
    return (
        f"{base_input} Контекст {ticket}: {shift}; {condition}; {probe}. "
        f"Область проверки: {scope}. Не делать broad fix без evidence."
    )


def variant_target(base_target: str, variant_index: int) -> str:
    scope = SCOPES[variant_index % len(SCOPES)]
    return (
        f"Следующий ход для области {scope}: {base_target} "
        "Evidence-boundary тот же: scope, слой и минимальное действие."
    )


def assign_cross_operator_negatives(rows: list[dict]) -> None:
    for index, row in enumerate(rows):
        decoy = rows[(index + VARIANTS_PER_CASE) % len(rows)]
        row["near_negative"] = decoy["target"]
        row["why_negative_is_wrong"] = (
            "Near-negative выглядит как правильный target-ответ из того же VPN-домена, "
            f"но принадлежит другому оператору/слою ({decoy['source_group']}) и не "
            f"следует evidence текущего запроса ({row['source_group']})."
        )


def main() -> int:
    compact = load_json(COMPACT_ROOT / "compact_cases.json")
    domains = load_json(DOMAIN_ROOT / "domains.json")
    domain = next(row for row in domains["domains"] if row["domain_id"] == compact["domain_id"])
    templates = load_json(DOMAIN_ROOT / domain["files"]["templates"])["templates"]
    operators = load_json(DOMAIN_ROOT / domain["files"]["operators"])["operators"]
    negative_rules = load_json(DOMAIN_ROOT / domain["files"]["negative_rules"])["negative_rules"]

    template_by_id = {row["template_id"]: row for row in templates}
    operator_by_family = {row["operator_family"]: row for row in operators}
    negative_by_id = {row["negative_rule_id"]: row for row in negative_rules}

    rows: list[dict] = []
    for case in compact["cases"]:
        template = template_by_id[case["template_id"]]
        operator = operator_by_family[template["operator_family"]]
        first_negative = negative_by_id[template["negative_rule_ids"][0]]
        for variant_index in range(VARIANTS_PER_CASE):
            if len(rows) >= TARGET_ROWS:
                break
            serial = len(rows) + 1
            input_text = variant_input(case["input"], variant_index, case["case_id"])
            target_text = variant_target(case["target"], variant_index)
            domain_tags = unique_list(domain["tags"] + template["constraint_tags"])
            rows.append(
                {
                    "schema_version": "wave_task_v2",
                    "task_id": f"lnvpn_accepted_1k_v1_{serial:06d}",
                    "language": case["language"],
                    "task_kind": template["task_kind"],
                    "domain_path": domain["domain_path"],
                    "domain_tags": domain_tags,
                    "source_family": "domain_dsl_accepted_1k_v1",
                    "source_group": case["source_group"],
                    "input": input_text,
                    "target": target_text,
                    "near_negative": "",
                    "operator_family": operator["operator_family"],
                    "why_target_is_correct": (
                        "Цель выбирает правильный следующий ход по оператору "
                        f"{operator['operator_family']}: {target_text}"
                    ),
                    "why_negative_is_wrong": (
                        "Near-negative будет назначен после сборки как target-like "
                        f"ошибка соседнего оператора; базовое правило: {first_negative['negative_rule_id']}."
                    ),
                    "shortcut_risk": unique_list(
                        [
                            "generated_from_domain_dsl",
                            "cross_operator_hard_negative",
                            "template_family_holdout_required",
                            "near_negative_must_survive_bayesian",
                            "near_negative_must_survive_l2_neighbor",
                        ]
                        + template["constraint_tags"][:2]
                    ),
                    "quality_status": "candidate",
                }
            )
        if len(rows) >= TARGET_ROWS:
            break

    if len(rows) != TARGET_ROWS:
        raise RuntimeError(f"expected {TARGET_ROWS} rows, built {len(rows)}")

    assign_cross_operator_negatives(rows)

    with OUT_JSONL.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")

    manifest = {
        "schema_version": "wave_accepted_task_manifest_v1",
        "domain_id": compact["domain_id"],
        "source_compact_file": str((COMPACT_ROOT / "compact_cases.json").relative_to(REPO)),
        "accepted_file": str(OUT_JSONL.relative_to(REPO)),
        "rows": len(rows),
        "quality_status": "candidate",
        "accepted_training_tasks": 0,
        "source_groups": len({row["source_group"] for row in rows}),
        "task_kinds": sorted({row["task_kind"] for row in rows}),
        "boundary": {
            "built_from_compact_cases": True,
            "not_runtime_authority": True,
            "requires_shortcut_gate_pass": True,
            "step_9_target_rows": TARGET_ROWS,
        },
    }
    with OUT_MANIFEST.open("w", encoding="utf-8") as handle:
        json.dump(manifest, handle, ensure_ascii=False, indent=2)
        handle.write("\n")

    print(f"built accepted 1k rows={len(rows)} -> {OUT_JSONL}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
