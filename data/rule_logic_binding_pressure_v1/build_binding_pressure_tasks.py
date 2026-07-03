#!/usr/bin/env python3
"""Build binding-pressure action traces for L3."""

from __future__ import annotations

import json
import random
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parent
OUT = ROOT / "accepted_binding_pressure_tasks_v1.jsonl"
MANIFEST = ROOT / "manifest.json"
RNG_SEED = 20260630

RULE_FAMILY = {
    "missing_variable_bind": "binding",
    "conflict_fact_bind": "binding",
    "mirror_first_bind": "position_binding",
    "alternation_second_bind": "position_binding",
}
RULE_IDS = list(RULE_FAMILY)

ROUTES = [
    "spb_msk",
    "port_terminal",
    "office_airport",
    "warehouse_client",
    "branch_gateway",
    "mobile_peer",
]

TRAIN_VALUES = [
    "start_time",
    "cargo_weight",
    "destination_point",
    "transport_mode",
    "route_scope",
    "auth_token",
    "peer_name",
    "dns_profile",
    "allowed_ips",
    "rollback_point",
    "baseline_log",
    "incident_window",
    "client_subnet",
    "server_side",
    "evidence_window",
    "config_snapshot",
]

HELDOUT_VALUES = [
    "handoff_time",
    "package_volume",
    "arrival_terminal",
    "vehicle_class",
    "return_path",
    "session_secret",
    "endpoint_label",
    "resolver_policy",
    "route_prefix",
    "restore_marker",
    "smoke_trace",
    "change_window",
    "office_vlan",
    "gateway_side",
    "proof_window",
    "wg_snapshot",
]

TRAIN_SYMBOLS = list("ABCDEFGHJKLMNPQRST")
HELDOUT_SYMBOLS = list("UVWXYZ")
TRAIN_WORDS = [
    "schet",
    "akt",
    "sklad",
    "vyhod",
    "platezh",
    "zayavka",
    "krug",
    "siniy",
    "vhod",
    "sever",
    "utro",
    "dostavka",
]
HELDOUT_WORDS = ["client", "server", "token", "route", "window", "backup"]


def row(
    index: int,
    split: str,
    rule_id: str,
    surface_family: str,
    before: str,
    demo: str,
    correct: str,
    wrong: str,
    answer_status: str,
    why_target: str,
    why_wrong: str,
    risks: list[str],
) -> dict[str, object]:
    return {
        "schema_version": "rule_action_trace_v1",
        "task_id": f"bind_v1_{index:06d}",
        "language": "mixed-symbolic-ru-en",
        "surface_family": surface_family,
        "source_group": f"binding_{split}_{rule_id}",
        "state_before": before,
        "rule_action_example": demo,
        "state_after_correct": correct,
        "state_after_wrong": wrong,
        "answer_status": answer_status,
        "proof_rule_id": rule_id,
        "proof_rule_family": RULE_FAMILY[rule_id],
        "why_target_is_correct": why_target,
        "why_negative_is_wrong": why_wrong,
        "shortcut_risk": risks,
        "quality_status": "accepted",
    }


def value_pool(split: str) -> list[str]:
    return TRAIN_VALUES if split == "train" else HELDOUT_VALUES


def token_pool(split: str, surface: str) -> list[str]:
    if surface == "symbols":
        return TRAIN_SYMBOLS if split == "train" else HELDOUT_SYMBOLS
    return TRAIN_WORDS if split == "train" else HELDOUT_WORDS


def choose_other(rng: random.Random, pool: list[str], value: str) -> str:
    candidates = [item for item in pool if item != value]
    return rng.choice(candidates)


def make_missing(index: int, split: str, rng: random.Random) -> dict[str, object]:
    pool = value_pool(split)
    variable = rng.choice(pool)
    wrong = choose_other(rng, pool, variable)
    route = rng.choice(ROUTES)
    before = f"state: estimate {route}; missing {variable}"
    demo = "action_demo: missing demo_slot -> ask demo_slot; action: transfer missing variable"
    return row(
        index,
        split,
        "missing_variable_bind",
        "binding_trace",
        before,
        demo,
        f"state: UNSETTLED ask {variable}",
        f"state: UNSETTLED ask {wrong}",
        "UNSETTLED",
        "The action transfers the missing variable into the ask state.",
        "The wrong state asks for a different variable.",
        ["exact_variable_lookup", "ask_prior", "surface_copy"],
    )


def make_conflict(index: int, split: str, rng: random.Random) -> dict[str, object]:
    pool = value_pool(split)
    fact = rng.choice(pool)
    wrong = choose_other(rng, pool, fact)
    before = f"state: source_a {fact}; source_b not_{fact}"
    demo = "action_demo: source_a demo_flag; source_b not_demo_flag -> verify demo_flag; action: transfer conflict fact"
    return row(
        index,
        split,
        "conflict_fact_bind",
        "binding_trace",
        before,
        demo,
        f"state: CONFLICT verify {fact}",
        f"state: CONFLICT verify {wrong}",
        "CONFLICT",
        "The action transfers the conflicted fact into the verify state.",
        "The wrong state verifies a different fact.",
        ["exact_fact_lookup", "verify_prior", "surface_copy"],
    )


def make_mirror(index: int, split: str, rng: random.Random) -> dict[str, object]:
    surface = rng.choice(["symbols", "ru_words"])
    pool = token_pool(split, surface)
    a, b, c = rng.sample(pool, 3)
    wrong = b
    before = f"state: {a} {b} {c} | {c} {b}"
    demo = "action_demo: demo_a demo_b demo_c | demo_c demo_b -> demo_a; action: transfer mirrored first item"
    return row(
        index,
        split,
        "mirror_first_bind",
        surface,
        before,
        demo,
        f"state: {a} {b} {c} | {c} {b} {a}",
        f"state: {a} {b} {c} | {c} {b} {wrong}",
        "PROVEN",
        "The action transfers the first mirrored item to close the mirror.",
        "The wrong state copies the neighbor instead of the mirrored first item.",
        ["position_lookup", "middle_token_bias", "surface_copy"],
    )


def make_alternation(index: int, split: str, rng: random.Random) -> dict[str, object]:
    surface = rng.choice(["symbols", "ru_words"])
    pool = token_pool(split, surface)
    a, b = rng.sample(pool, 2)
    before = f"state: {a} {b} {a} {b} {a}"
    demo = "action_demo: demo_a demo_b demo_a -> demo_b; action: transfer alternating second item"
    return row(
        index,
        split,
        "alternation_second_bind",
        surface,
        before,
        demo,
        f"state: {a} {b} {a} {b} {a} {b}",
        f"state: {a} {b} {a} {b} {a} {a}",
        "PROVEN",
        "The action transfers the second item of the alternating pair.",
        "The wrong state repeats the previous item.",
        ["position_lookup", "last_token_bias", "surface_copy"],
    )


FACTORIES = {
    "missing_variable_bind": make_missing,
    "conflict_fact_bind": make_conflict,
    "mirror_first_bind": make_mirror,
    "alternation_second_bind": make_alternation,
}


def build(train_per_rule: int, heldout_per_rule: int) -> list[dict[str, object]]:
    rng = random.Random(RNG_SEED)
    rows: list[dict[str, object]] = []
    index = 0
    for split, per_rule in [("train", train_per_rule), ("heldout", heldout_per_rule)]:
        for _ in range(per_rule):
            for rule_id in RULE_IDS:
                rows.append(FACTORIES[rule_id](index, split, rng))
                index += 1
    return rows


def main() -> int:
    rows = build(train_per_rule=500, heldout_per_rule=200)
    OUT.write_text(
        "".join(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )
    counts = Counter(row["proof_rule_id"] for row in rows)
    split_counts = Counter(row["source_group"].split("_")[1] for row in rows)
    manifest = {
        "schema_version": "binding_pressure_manifest_v1",
        "rows": len(rows),
        "train_rows": split_counts["train"],
        "heldout_rows": split_counts["heldout"],
        "rules": dict(sorted(counts.items())),
        "train_values": TRAIN_VALUES,
        "heldout_values": HELDOUT_VALUES,
        "train_heldout_value_overlap": sorted(set(TRAIN_VALUES) & set(HELDOUT_VALUES)),
        "train_heldout_symbol_overlap": sorted(set(TRAIN_SYMBOLS) & set(HELDOUT_SYMBOLS)),
        "train_heldout_word_overlap": sorted(set(TRAIN_WORDS) & set(HELDOUT_WORDS)),
    }
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

