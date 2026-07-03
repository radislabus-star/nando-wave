#!/usr/bin/env python3
"""Build balanced v3 ordered position-binding tasks."""

from __future__ import annotations

import json
import math
import os
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parent
OUTPUT_DIR = Path(os.environ.get("POSITION_SEQUENCE_OUTPUT_DIR", ROOT)).resolve()
OUT = OUTPUT_DIR / "accepted_position_sequence_tasks_v3.jsonl"
MANIFEST = OUTPUT_DIR / "manifest.json"

DEFAULT_LENGTHS = [3, 4, 5, 6, 7, 8]
DEFAULT_SURFACE_FAMILIES = ["symbols", "ru_words", "business", "network"]
DEFAULT_NOISE_TYPES = ["clean", "prefix_suffix", "punctuation", "distractor", "instruction_noise"]
DEFAULT_RULE_FAMILIES = [
    "full_mirror",
    "rotate_left_1",
    "rotate_right_1",
    "rotate_left_2",
    "pair_swap",
    "block_swap",
    "edge_to_center",
    "even_odd_split",
]


def env_int(name: str, default: int) -> int:
    raw = os.environ.get(name)
    if raw is None or raw.strip() == "":
        return default
    value = int(raw)
    if value <= 0:
        raise ValueError(f"{name} must be positive")
    return value


def env_nonnegative_int(name: str, default: int) -> int:
    raw = os.environ.get(name)
    if raw is None or raw.strip() == "":
        return default
    value = int(raw)
    if value < 0:
        raise ValueError(f"{name} must be non-negative")
    return value


def env_csv(name: str, default: list[str]) -> list[str]:
    raw = os.environ.get(name)
    if raw is None or raw.strip() == "":
        return default
    values = [item.strip() for item in raw.split(",") if item.strip()]
    if not values:
        raise ValueError(f"{name} must not be empty")
    return values


def env_int_csv(name: str, default: list[int]) -> list[int]:
    return [int(item) for item in env_csv(name, [str(item) for item in default])]


LENGTHS = env_int_csv("POSITION_SEQUENCE_LENGTHS", DEFAULT_LENGTHS)
SURFACE_FAMILIES = env_csv("POSITION_SEQUENCE_SURFACE_FAMILIES", DEFAULT_SURFACE_FAMILIES)
NOISE_TYPES = env_csv("POSITION_SEQUENCE_NOISE_TYPES", DEFAULT_NOISE_TYPES)
RULE_FAMILIES = env_csv("POSITION_SEQUENCE_RULE_FAMILIES", DEFAULT_RULE_FAMILIES)
TRAIN_PER_CELL = env_int("POSITION_SEQUENCE_TRAIN_PER_CELL", 2)
HELDOUT_PER_CELL = env_int("POSITION_SEQUENCE_HELDOUT_PER_CELL", 1)
SEED = env_nonnegative_int("POSITION_SEQUENCE_SEED", 0)

TRAIN_POOLS = {
    "symbols": [f"A{i}" for i in range(16)],
    "ru_words": [
        "akt",
        "schet",
        "sklad",
        "vyhod",
        "sever",
        "utro",
        "krug",
        "vhod",
        "zayavka",
        "platezh",
        "dostavka",
        "ostatok",
        "zapis",
        "kontur",
        "linia",
        "otchet",
        "razdel",
        "modul",
        "signal",
        "punkt",
    ],
    "business": [
        "invoice",
        "client",
        "sku",
        "route",
        "margin",
        "stock",
        "payment",
        "handoff",
        "warehouse",
        "contract",
        "pickup",
        "reserve",
        "broker",
        "tariff",
        "claim",
        "ledger",
        "vendor",
        "cargo",
        "permit",
        "seal",
    ],
    "network": [
        "vpn",
        "dns",
        "peer",
        "route",
        "gateway",
        "subnet",
        "token",
        "endpoint",
        "resolver",
        "tunnel",
        "policy",
        "snapshot",
        "bridge",
        "packet",
        "cipher",
        "socket",
        "daemon",
        "uplink",
        "metric",
        "prefix",
    ],
}

HELDOUT_POOLS = {
    "symbols": [f"Z{i}" for i in range(16)],
    "ru_words": [
        "okno",
        "klient",
        "server",
        "metka",
        "kanal",
        "rezerv",
        "terminal",
        "pravilo",
        "podpis",
        "paket",
        "smena",
        "uzel",
        "forma",
        "reyestr",
        "skhema",
        "oblast",
        "shag",
        "rebro",
        "versiya",
        "blok",
    ],
    "business": [
        "order",
        "buyer",
        "batch",
        "terminal",
        "price",
        "allocation",
        "deal",
        "carrier",
        "customs",
        "label",
        "pallet",
        "invoice2",
        "manifest",
        "agent",
        "freight",
        "quota",
        "debit",
        "origin",
        "sample",
        "broker2",
    ],
    "network": [
        "wireguard",
        "router",
        "cidr",
        "relay",
        "key",
        "profile",
        "nat",
        "mtu",
        "firewall",
        "session",
        "probe",
        "link",
        "wan",
        "lan",
        "route2",
        "queue",
        "kernel",
        "port",
        "mask",
        "lease",
    ],
}


def pick_tokens(pool: list[str], index: int, length: int, seed: int) -> list[str]:
    if length > len(pool):
        raise ValueError(
            f"sequence length {length} needs at least {length} unique tokens, "
            f"but pool has only {len(pool)}"
        )
    start = index + seed * 7 + length * (seed % 5)
    stride_candidates = [1, 5, 7, 11, 13, 17, 19]
    rotated = stride_candidates[seed % len(stride_candidates) :] + stride_candidates[
        : seed % len(stride_candidates)
    ]
    stride = next(candidate for candidate in rotated if math.gcd(candidate, len(pool)) == 1)
    return [pool[(start + offset * stride) % len(pool)] for offset in range(length)]


def demo_tokens(prefix: str, length: int) -> list[str]:
    return [f"{prefix}{index}" for index in range(length)]


def transform(rule_family: str, tokens: list[str]) -> list[str]:
    if rule_family == "full_mirror":
        return list(reversed(tokens))
    if rule_family == "rotate_left_1":
        return tokens[1:] + tokens[:1]
    if rule_family == "rotate_right_1":
        return tokens[-1:] + tokens[:-1]
    if rule_family == "rotate_left_2":
        return tokens[2:] + tokens[:2]
    if rule_family == "pair_swap":
        out = tokens[:]
        for index in range(0, len(out) - 1, 2):
            out[index], out[index + 1] = out[index + 1], out[index]
        if len(out) % 2 == 1:
            out = out[-1:] + out[:-1]
        return out
    if rule_family == "block_swap":
        midpoint = len(tokens) // 2
        return tokens[midpoint:] + tokens[:midpoint]
    if rule_family == "edge_to_center":
        out = []
        left = 0
        right = len(tokens) - 1
        while left <= right:
            out.append(tokens[right])
            if left != right:
                out.append(tokens[left])
            left += 1
            right -= 1
        return out
    if rule_family == "even_odd_split":
        return tokens[::2] + tokens[1::2]
    raise ValueError(f"unknown rule family: {rule_family}")


def unique_rule_families_for_length(length: int) -> list[str]:
    seen: dict[tuple[int, ...], str] = {}
    selected: list[str] = []
    slots = list(range(length))
    for rule_family in RULE_FAMILIES:
        signature = tuple(transform(rule_family, slots))
        if signature in seen:
            continue
        seen[signature] = rule_family
        selected.append(rule_family)
    return selected


def skipped_equivalent_rules() -> dict[str, str]:
    skipped: dict[str, str] = {}
    for length in LENGTHS:
        seen: dict[tuple[int, ...], str] = {}
        slots = list(range(length))
        for rule_family in RULE_FAMILIES:
            signature = tuple(transform(rule_family, slots))
            previous = seen.get(signature)
            rule_id = f"{rule_family}_len{length}"
            if previous is not None:
                skipped[rule_id] = f"{previous}_len{length}"
                continue
            seen[signature] = rule_family
    return skipped


def deranged(candidate: list[str], good: list[str]) -> bool:
    return sorted(candidate) == sorted(good) and all(left != right for left, right in zip(candidate, good))


def rotate(items: list[str], shift: int) -> list[str]:
    shift %= len(items)
    return items[shift:] + items[:shift]


def make_derangement(candidate: list[str], good: list[str], salt: int) -> list[str]:
    if deranged(candidate, good):
        return candidate
    for shift in range(1, len(good)):
        rotated = rotate(candidate, shift + salt)
        if deranged(rotated, good):
            return rotated
    for shift in range(1, len(good)):
        rotated = rotate(good, shift)
        if deranged(rotated, good):
            return rotated
    raise ValueError("cannot construct derangement")


def negative_tokens(rule_family: str, source: list[str], good: list[str], task_index: int) -> tuple[str, list[str]]:
    strategy_index = (task_index + SEED) % 6
    if strategy_index == 0:
        return "phase_shift", rotate(good, 1)
    if strategy_index == 1:
        adjacent = good[:]
        for index in range(0, len(adjacent) - 1, 2):
            adjacent[index], adjacent[index + 1] = adjacent[index + 1], adjacent[index]
        return "adjacent_slot_swap", make_derangement(adjacent, good, task_index)
    if strategy_index == 2:
        midpoint = len(good) // 2
        return "block_swap_wrong", make_derangement(good[midpoint:] + good[:midpoint], good, task_index)
    if strategy_index == 3:
        inverse = rotate(source, -1 if "left" in rule_family else 1)
        return "inverse_rotation_trap", make_derangement(inverse, good, task_index)
    if strategy_index == 4:
        mirror = list(reversed(source))
        return "mirror_vs_rotate_trap", make_derangement(mirror, good, task_index)
    return "sampled_derangement", rotate(good, 2 if len(good) > 3 else 1)


def action_example(rule_family: str, length: int) -> str:
    slots = list(range(length))
    transformed_slots = transform(rule_family, slots)
    slot_order = " ".join(f"src{slot}" for slot in transformed_slots)
    before = demo_tokens("d", length)
    after = transform(rule_family, before)
    return (
        f"operator_slots: {slot_order}; "
        f"demo: {' '.join(before)} -> {' '.join(after)}; "
        "apply the same source-slot order"
    )


def state_before(split: str, task_index: int, noise_type: str, tokens: list[str]) -> str:
    sequence = " ".join(tokens)
    if noise_type == "clean":
        return f"state: sequence: {sequence}"
    if noise_type == "prefix_suffix":
        return f"state: note_{split}_{SEED}_{task_index}; sequence: {sequence}; tail order_probe"
    if noise_type == "punctuation":
        return f"state: check-order? sequence: {', '.join(tokens)}; done."
    if noise_type == "distractor":
        distractor = " ".join(reversed(tokens))
        return f"state: distractor: {distractor}; sequence: {sequence}; use real span only"
    if noise_type == "instruction_noise":
        return f"state: please transform carefully; keep slots stable; sequence: {sequence}; thanks"
    raise ValueError(f"unknown noise type: {noise_type}")


def row(
    task_index: int,
    split: str,
    length: int,
    rule_family: str,
    surface_family: str,
    noise_type: str,
    sample_index: int,
) -> dict[str, object]:
    pool = TRAIN_POOLS[surface_family] if split == "train" else HELDOUT_POOLS[surface_family]
    source = pick_tokens(pool, task_index + sample_index, length, SEED)
    good = transform(rule_family, source)
    negative_strategy, bad = negative_tokens(rule_family, source, good, task_index)
    assert deranged(bad, good), (rule_family, source, good, bad)
    rule_id = f"{rule_family}_len{length}"
    return {
        "schema_version": "position_sequence_v3",
        "task_id": f"posseq_v3_s{SEED:03d}_{task_index:06d}",
        "language": "mixed-symbolic-ru-en",
        "source_group": f"position_sequence_{split}_{rule_id}",
        "surface_family": surface_family,
        "noise_type": noise_type,
        "sequence_length": length,
        "proof_rule_id": rule_id,
        "proof_rule_family": rule_family,
        "state_before": state_before(split, task_index, noise_type, source),
        "rule_action_example": action_example(rule_family, length),
        "state_after_correct": f"state: {' '.join(good)}",
        "state_after_wrong": f"state: {' '.join(bad)}",
        "source_tokens": source,
        "correct_tokens": good,
        "wrong_tokens": bad,
        "answer_status": "PROVEN",
        "why_target_is_correct": "The target applies the demonstrated order transform to every output slot.",
        "why_negative_is_wrong": "The negative is a same-bag derangement: every output slot is wrong.",
        "negative_strategy": negative_strategy,
        "shortcut_risk": [
            "exact_lookup",
            "bag_of_tokens",
            "length_only",
            "output_position_prior",
            "template_without_sequence",
            "markov_bigram",
            "l2_neighbor_target_copy",
        ],
        "quality_status": "accepted",
    }


def build_split(split: str, per_cell: int, task_index: int) -> tuple[list[dict[str, object]], int]:
    rows: list[dict[str, object]] = []
    for length in LENGTHS:
        for rule_family in unique_rule_families_for_length(length):
            for surface_family in SURFACE_FAMILIES:
                for noise_type in NOISE_TYPES:
                    for sample_index in range(per_cell):
                        rows.append(row(task_index, split, length, rule_family, surface_family, noise_type, sample_index))
                        task_index += 1
    return rows, task_index


def main() -> int:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    task_index = 0
    train, task_index = build_split("train", TRAIN_PER_CELL, task_index)
    heldout, task_index = build_split("heldout", HELDOUT_PER_CELL, task_index)
    rows = train + heldout

    OUT.write_text(
        "".join(json.dumps(item, ensure_ascii=False, sort_keys=True) + "\n" for item in rows),
        encoding="utf-8",
    )

    manifest = {
        "schema_version": "position_sequence_manifest_v3",
        "rows": len(rows),
        "train_rows": len(train),
        "heldout_rows": len(heldout),
        "train_per_cell": TRAIN_PER_CELL,
        "heldout_per_cell": HELDOUT_PER_CELL,
        "seed": SEED,
        "output_dir": str(OUTPUT_DIR),
        "lengths": LENGTHS,
        "rule_families": RULE_FAMILIES,
        "surface_families": SURFACE_FAMILIES,
        "noise_types": NOISE_TYPES,
        "matrix_cells": sum(len(unique_rule_families_for_length(length)) for length in LENGTHS)
        * len(SURFACE_FAMILIES)
        * len(NOISE_TYPES),
        "rules": dict(sorted(Counter(str(item["proof_rule_id"]) for item in rows).items())),
        "skipped_equivalent_rules": skipped_equivalent_rules(),
        "negative_strategies": dict(sorted(Counter(str(item["negative_strategy"]) for item in rows).items())),
        "train_heldout_overlap_by_surface": {
            surface: sorted(set(TRAIN_POOLS[surface]) & set(HELDOUT_POOLS[surface]))
            for surface in SURFACE_FAMILIES
        },
        "correct_wrong_same_bag": True,
        "same_bag_derangement_required": True,
    }
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
