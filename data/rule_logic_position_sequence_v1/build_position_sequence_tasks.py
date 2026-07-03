#!/usr/bin/env python3
"""Build persisted ordered position-binding tasks for L3 Step 12."""

from __future__ import annotations

import json
from collections import Counter
from itertools import permutations
from pathlib import Path


ROOT = Path(__file__).resolve().parent
OUT = ROOT / "accepted_position_sequence_tasks_v1.jsonl"
MANIFEST = ROOT / "manifest.json"

TRAIN_PER_RULE = 180
HELDOUT_PER_RULE = 72

RULES = {
    "mirror3_full_sequence": {
        "length": 3,
        "action": (
            "action_demo: demo_a demo_b demo_c -> demo_c demo_b demo_a; "
            "action: emit full mirror sequence"
        ),
    },
    "rotate4_left_sequence": {
        "length": 4,
        "action": (
            "action_demo: demo_a demo_b demo_c demo_d -> demo_b demo_c demo_d demo_a; "
            "action: emit rotate-left sequence"
        ),
    },
    "pair_swap4_sequence": {
        "length": 4,
        "action": (
            "action_demo: demo_a demo_b demo_c demo_d -> demo_b demo_a demo_d demo_c; "
            "action: emit pair-swap sequence"
        ),
    },
    "mirror5_full_sequence": {
        "length": 5,
        "action": (
            "action_demo: demo_a demo_b demo_c demo_d demo_e -> demo_e demo_d demo_c demo_b demo_a; "
            "action: emit five-item mirror sequence"
        ),
    },
    "rotate5_right_sequence": {
        "length": 5,
        "action": (
            "action_demo: demo_a demo_b demo_c demo_d demo_e -> demo_e demo_a demo_b demo_c demo_d; "
            "action: emit rotate-right sequence"
        ),
    },
    "rotate6_left2_sequence": {
        "length": 6,
        "action": (
            "action_demo: demo_a demo_b demo_c demo_d demo_e demo_f -> demo_c demo_d demo_e demo_f demo_a demo_b; "
            "action: emit rotate-left-two sequence"
        ),
    },
    "pair_swap6_sequence": {
        "length": 6,
        "action": (
            "action_demo: demo_a demo_b demo_c demo_d demo_e demo_f -> demo_b demo_a demo_d demo_c demo_f demo_e; "
            "action: emit three pair-swaps sequence"
        ),
    },
}

TRAIN_POOLS = {
    "symbols": ["A", "B", "C", "D", "E", "F", "G", "H"],
    "ru_words": ["akt", "schet", "sklad", "vyhod", "sever", "utro", "krug", "vhod"],
    "business": ["invoice", "client", "sku", "route", "margin", "stock", "payment", "handoff"],
}

HELDOUT_POOLS = {
    "symbols": ["U", "V", "W", "X", "Y", "Z"],
    "ru_words": ["client", "server", "token", "route", "window", "backup"],
    "business": ["order", "buyer", "batch", "terminal", "price", "reserve", "contract", "pickup"],
}


def pick_tokens(pool: list[str], index: int, length: int) -> list[str]:
    return [pool[(index + offset) % len(pool)] for offset in range(length)]


def correct_tokens(rule_id: str, tokens: list[str]) -> list[str]:
    if rule_id == "mirror3_full_sequence":
        return [tokens[2], tokens[1], tokens[0]]
    if rule_id == "rotate4_left_sequence":
        return [tokens[1], tokens[2], tokens[3], tokens[0]]
    if rule_id == "pair_swap4_sequence":
        return [tokens[1], tokens[0], tokens[3], tokens[2]]
    if rule_id == "mirror5_full_sequence":
        return [tokens[4], tokens[3], tokens[2], tokens[1], tokens[0]]
    if rule_id == "rotate5_right_sequence":
        return [tokens[4], tokens[0], tokens[1], tokens[2], tokens[3]]
    if rule_id == "rotate6_left2_sequence":
        return [tokens[2], tokens[3], tokens[4], tokens[5], tokens[0], tokens[1]]
    if rule_id == "pair_swap6_sequence":
        return [tokens[1], tokens[0], tokens[3], tokens[2], tokens[5], tokens[4]]
    raise ValueError(f"unknown rule: {rule_id}")


def wrong_tokens(rule_id: str, tokens: list[str], task_index: int) -> list[str]:
    good = correct_tokens(rule_id, tokens)
    candidates = [
        list(candidate)
        for candidate in permutations(good)
        if all(left != right for left, right in zip(candidate, good))
    ]
    if not candidates:
        raise ValueError(f"no same-bag derangement for rule: {rule_id}")
    return candidates[(task_index * 17 + len(rule_id)) % len(candidates)]


def state_before(split: str, task_index: int, tokens: list[str]) -> str:
    if split == "heldout":
        return f"state: noisy_prefix_{task_index}; sequence: {' '.join(tokens)}; noisy_suffix order_probe"
    return f"state: sequence: {' '.join(tokens)}"


def row(task_index: int, split: str, rule_id: str, surface_family: str, tokens: list[str]) -> dict[str, object]:
    good = correct_tokens(rule_id, tokens)
    bad = wrong_tokens(rule_id, tokens, task_index)
    assert sorted(good) == sorted(bad)
    assert all(left != right for left, right in zip(good, bad))
    return {
        "schema_version": "position_sequence_v2",
        "task_id": f"posseq_v1_{task_index:06d}",
        "language": "mixed-symbolic-ru-en",
        "source_group": f"position_sequence_{split}_{rule_id}",
        "surface_family": surface_family,
        "proof_rule_id": rule_id,
        "proof_rule_family": "ordered_position_binding",
        "state_before": state_before(split, task_index, tokens),
        "rule_action_example": RULES[rule_id]["action"],
        "state_after_correct": f"state: {' '.join(good)}",
        "state_after_wrong": f"state: {' '.join(bad)}",
        "correct_tokens": good,
        "wrong_tokens": bad,
        "answer_status": "PROVEN",
        "why_target_is_correct": "The target applies the ordered position transform to every slot.",
        "why_negative_is_wrong": "The negative is a same-bag derangement: every output slot is wrong.",
        "negative_strategy": "sampled_derangement_same_bag",
        "shortcut_risk": [
            "exact_lookup",
            "bag_of_tokens",
            "surface_family_majority",
            "proof_rule_majority",
            "markov_bigram",
        ],
        "quality_status": "accepted",
    }


def build_split(split: str, per_rule: int, task_index: int) -> tuple[list[dict[str, object]], int]:
    pools = TRAIN_POOLS if split == "train" else HELDOUT_POOLS
    rows: list[dict[str, object]] = []
    for index in range(per_rule):
        surface_family = ["symbols", "ru_words", "business"][index % 3]
        pool = pools[surface_family]
        for rule_id, spec in RULES.items():
            tokens = pick_tokens(pool, index, int(spec["length"]))
            rows.append(row(task_index, split, rule_id, surface_family, tokens))
            task_index += 1
    return rows, task_index


def main() -> int:
    task_index = 0
    train, task_index = build_split("train", TRAIN_PER_RULE, task_index)
    heldout, task_index = build_split("heldout", HELDOUT_PER_RULE, task_index)
    rows = train + heldout

    OUT.write_text(
        "".join(json.dumps(item, ensure_ascii=False, sort_keys=True) + "\n" for item in rows),
        encoding="utf-8",
    )

    rules = Counter(str(item["proof_rule_id"]) for item in rows)
    surfaces = Counter(str(item["surface_family"]) for item in rows)
    split_counts = Counter(str(item["source_group"]).split("_")[2] for item in rows)
    manifest = {
        "schema_version": "position_sequence_manifest_v2",
        "rows": len(rows),
        "train_rows": split_counts["train"],
        "heldout_rows": split_counts["heldout"],
        "rules": dict(sorted(rules.items())),
        "surface_families": dict(sorted(surfaces.items())),
        "train_heldout_symbol_overlap": sorted(set(TRAIN_POOLS["symbols"]) & set(HELDOUT_POOLS["symbols"])),
        "train_heldout_word_overlap": sorted(set(TRAIN_POOLS["ru_words"]) & set(HELDOUT_POOLS["ru_words"])),
        "train_heldout_business_overlap": sorted(set(TRAIN_POOLS["business"]) & set(HELDOUT_POOLS["business"])),
        "heldout_has_context_noise": True,
        "sequence_lengths": sorted({int(spec["length"]) for spec in RULES.values()}),
        "correct_wrong_same_bag": True,
    }
    MANIFEST.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
