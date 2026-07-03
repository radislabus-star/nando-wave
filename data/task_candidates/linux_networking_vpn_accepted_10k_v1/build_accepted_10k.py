#!/usr/bin/env python3
"""Build a 10k accepted Wave Task V2 corpus from the compact VPN task cases."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
COMPACT_ROOT = REPO / "data" / "task_candidates" / "linux_networking_vpn_compact_v1"
DOMAIN_ROOT = REPO / "data" / "domain_dsl_v1"
OUT_JSONL = ROOT / "accepted_wave_task_v2.jsonl"
OUT_MANIFEST = ROOT / "manifest.json"

TARGET_ROWS = 10_000

SHIFTS = [
    "смена A",
    "смена B",
    "ночной инцидент",
    "утренний rollout",
    "проверка после миграции",
    "дежурный канал",
    "проверка филиала",
    "post-change контроль",
]

CONDITIONS = [
    "снимок до изменения есть частично",
    "оператор получил свежий лог",
    "изменения запрещены без baseline",
    "пользователи уже в сети",
    "старый профиль еще доступен",
    "симптом повторяется на двух клиентах",
    "затронут только один сегмент",
    "нужно сохранить rollback option",
]

PROBES = [
    "контрольный ping/curl повторен дважды",
    "проверка выполнена с клиента и сервера",
    "есть один успешный и один отрицательный probe",
    "нужен минимальный безопасный следующий шаг",
    "нужно не смешать route и auth слой",
    "требуется отделить evidence от догадки",
    "нужно сохранить scope исправления",
    "smoke test должен остаться bounded",
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
    "edge gateway",
    "mobile client",
    "dns profile",
    "firewall chain",
    "return path",
    "auth profile",
]

EVIDENCE_WINDOWS = [
    "window red",
    "window blue",
    "window green",
    "window amber",
    "window white",
    "window black",
    "window north",
    "window south",
]

PRESSURE_NOTES = [
    "не делать broad fix без evidence",
    "не менять соседний слой без доказательства",
    "не повышать гипотезу до answer authority",
    "не путать симптом и cause",
    "не смешивать client-side и server-side scope",
    "не расширять blast radius",
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


def variant_input(base_input: str, variant_index: int) -> str:
    shift = SHIFTS[variant_index % len(SHIFTS)]
    condition = CONDITIONS[(variant_index // len(SHIFTS)) % len(CONDITIONS)]
    probe = PROBES[(variant_index // (len(SHIFTS) * len(CONDITIONS))) % len(PROBES)]
    scope = SCOPES[variant_index % len(SCOPES)]
    window = EVIDENCE_WINDOWS[(variant_index // 3) % len(EVIDENCE_WINDOWS)]
    pressure = PRESSURE_NOTES[(variant_index // 5) % len(PRESSURE_NOTES)]
    return (
        f"{base_input} Контекст: {shift}; {condition}; {probe}. "
        f"Область проверки: {scope}. Evidence window: {window}. {pressure}."
    )


def variant_target(base_target: str, variant_index: int) -> str:
    scope = SCOPES[variant_index % len(SCOPES)]
    window = EVIDENCE_WINDOWS[(variant_index // 3) % len(EVIDENCE_WINDOWS)]
    return (
        f"Следующий ход для области {scope}: {base_target} "
        f"Evidence-boundary: {window}, слой и минимальное действие."
    )


def assign_cross_operator_negatives(rows: list[dict], variants_per_case: int) -> None:
    for index, row in enumerate(rows):
        decoy = rows[(index + variants_per_case) % len(rows)]
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

    cases = compact["cases"]
    variants_per_case = (TARGET_ROWS + len(cases) - 1) // len(cases)
    template_by_id = {row["template_id"]: row for row in templates}
    operator_by_family = {row["operator_family"]: row for row in operators}
    negative_by_id = {row["negative_rule_id"]: row for row in negative_rules}

    rows: list[dict] = []
    for case in cases:
        template = template_by_id[case["template_id"]]
        operator = operator_by_family[template["operator_family"]]
        first_negative = negative_by_id[template["negative_rule_ids"][0]]
        for variant_index in range(variants_per_case):
            if len(rows) >= TARGET_ROWS:
                break
            serial = len(rows) + 1
            input_text = variant_input(case["input"], variant_index)
            target_text = variant_target(case["target"], variant_index)
            domain_tags = unique_list(domain["tags"] + template["constraint_tags"])
            rows.append(
                {
                    "schema_version": "wave_task_v2",
                    "task_id": f"lnvpn_accepted_10k_v1_{serial:06d}",
                    "language": case["language"],
                    "task_kind": template["task_kind"],
                    "domain_path": domain["domain_path"],
                    "domain_tags": domain_tags,
                    "source_family": "domain_dsl_accepted_10k_v1",
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
                            "near_negative_must_survive_markov_bigram",
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

    assign_cross_operator_negatives(rows, variants_per_case)

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
            "step_10_target_rows": TARGET_ROWS,
        },
    }
    with OUT_MANIFEST.open("w", encoding="utf-8") as handle:
        json.dump(manifest, handle, ensure_ascii=False, indent=2)
        handle.write("\n")

    print(f"built accepted 10k rows={len(rows)} -> {OUT_JSONL}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
