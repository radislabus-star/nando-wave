#!/usr/bin/env python3
"""Build v4 proof-gated operator battery corpora."""

from __future__ import annotations

import json
import math
import os
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parent
OUTPUT_ROOT = Path(os.environ.get("OPERATOR_BATTERY_OUTPUT_DIR", ROOT)).resolve()

DEFAULT_CLASSES = ["order", "edit", "conditional", "composed"]
DEFAULT_LENGTHS = [9, 10, 11, 12, 13, 14, 15, 16]
DEFAULT_SURFACE_FAMILIES = ["symbols", "ru_words", "business", "network"]
DEFAULT_NOISE_TYPES = ["clean", "prefix_suffix", "distractor", "instruction_noise"]

ORDER_FAMILIES = [
    "full_mirror",
    "rotate_left_1",
    "rotate_right_1",
    "rotate_left_2",
    "rotate_left_3",
    "pair_swap",
    "block_swap",
    "edge_to_center",
    "center_to_edge",
    "even_odd_split",
    "odd_even_split",
    "interleave_halves",
    "stride_gather_3",
    "window_reverse_3",
    "block_reverse_4",
    "perfect_shuffle",
]

EDIT_FAMILIES = [
    "delete_first",
    "delete_last",
    "delete_middle",
    "insert_head_marker",
    "insert_tail_marker",
    "insert_middle_marker",
    "duplicate_first",
    "duplicate_last",
    "duplicate_middle",
    "replace_first_marker",
    "replace_last_marker",
    "drop_every_third",
]

CONDITIONAL_FAMILIES = [
    "if_alpha_mirror_else_rotate_left",
    "if_alpha_pair_swap_else_block_swap",
    "if_alpha_even_odd_else_odd_even",
    "if_alpha_edge_center_else_center_edge",
    "if_beta_rotate_right_else_mirror",
    "if_beta_block_swap_else_pair_swap",
    "if_beta_center_edge_else_rotate_left",
    "if_beta_stride3_else_window3",
]

COMPOSED_FAMILIES = [
    "mirror_then_rotate_left",
    "rotate_left_then_mirror",
    "pair_swap_then_mirror",
    "block_swap_then_rotate_right",
    "even_odd_then_mirror",
    "mirror_then_pair_swap",
    "edge_center_then_rotate_left",
    "rotate_left_then_even_odd",
]

CLASS_FAMILIES = {
    "order": ORDER_FAMILIES,
    "edit": EDIT_FAMILIES,
    "conditional": CONDITIONAL_FAMILIES,
    "composed": COMPOSED_FAMILIES,
}


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


def env_bool(name: str, default: bool = False) -> bool:
    raw = os.environ.get(name)
    if raw is None or raw.strip() == "":
        return default
    return raw.strip().lower() in {"1", "true", "yes", "on"}


def env_int_csv(name: str, default: list[int]) -> list[int]:
    return [int(item) for item in env_csv(name, [str(item) for item in default])]


CLASSES = env_csv("OPERATOR_BATTERY_CLASSES", DEFAULT_CLASSES)
LENGTHS = env_int_csv("OPERATOR_BATTERY_LENGTHS", DEFAULT_LENGTHS)
SURFACE_FAMILIES = env_csv("OPERATOR_BATTERY_SURFACE_FAMILIES", DEFAULT_SURFACE_FAMILIES)
NOISE_TYPES = env_csv("OPERATOR_BATTERY_NOISE_TYPES", DEFAULT_NOISE_TYPES)
TRAIN_PER_CELL = env_int("OPERATOR_BATTERY_TRAIN_PER_CELL", 1)
HELDOUT_PER_CELL = env_int("OPERATOR_BATTERY_HELDOUT_PER_CELL", 1)
SEED = env_nonnegative_int("OPERATOR_BATTERY_SEED", 0)
PAIRED_NOISE = env_bool("OPERATOR_BATTERY_PAIRED_NOISE", False)

TRAIN_POOLS = {
    "symbols": [f"A{i}" for i in range(40)],
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
        "uzor",
        "polosa",
        "metka",
        "sloj",
        "yadro",
        "duga",
        "ramka",
        "vstavka",
        "kolco",
        "sled",
        "forma",
        "rebro",
        "kanal",
        "shag",
        "blok",
        "veter",
        "zerno",
        "skhema",
        "faza",
        "volna",
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
        "quote",
        "batch",
        "sample",
        "origin",
        "customs",
        "freight",
        "terminal",
        "agent",
        "pallet",
        "label",
        "debit",
        "credit",
        "price",
        "buyer_h",
        "seller",
        "lot",
        "gate",
        "stack",
        "case",
        "risk",
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
        "queue",
        "kernel",
        "port",
        "mask",
        "lease",
        "route2",
        "guard",
    ],
}

HELDOUT_POOLS = {
    "symbols": [f"Z{i}" for i in range(40)],
    "ru_words": [
        "okno",
        "klient",
        "server",
        "pravilo",
        "podpis",
        "paket",
        "smena",
        "uzel",
        "reyestr",
        "oblast",
        "versiya",
        "styk",
        "pole",
        "tochka",
        "risunok",
        "perenos",
        "dolina",
        "vershina",
        "proem",
        "karta",
        "sreda",
        "kontrol",
        "povorot",
        "razbor",
        "stek",
        "vybor",
        "most",
        "shina",
        "plita",
        "nabor",
        "otsek",
        "fokus",
        "priem",
        "kraj",
        "center",
        "luch",
        "vklad",
        "ten",
        "osnova",
        "peredacha",
    ],
    "business": [
        "order",
        "buyer",
        "manifest",
        "allocation",
        "deal",
        "carrier",
        "quota",
        "sample2",
        "broker2",
        "counterparty",
        "booking",
        "license",
        "invoice2",
        "routecard",
        "shelf",
        "dock",
        "yard",
        "pickup2",
        "reserve2",
        "claim2",
        "entry",
        "exporter",
        "importer",
        "client2",
        "price2",
        "margin2",
        "stock2",
        "seal2",
        "batch2",
        "tariff2",
        "gate2",
        "case2",
        "lot2",
        "vendor2",
        "seller2",
        "buyer2",
        "agent2",
        "freight2",
        "customs2",
        "label2",
    ],
    "network": [
        "wireguard",
        "router_h",
        "cidr_h",
        "relay_h",
        "profile_h",
        "firewall_h",
        "session_h",
        "probe_h",
        "route3",
        "lease2",
        "prefix2",
        "kernel2",
        "socket2",
        "tunnel2",
        "daemon2",
        "metric2",
        "resolver2",
        "gateway2",
        "peer2",
        "subnet2",
        "wan2",
        "lan2",
        "queue2",
        "port2",
        "mask2",
        "nat2",
        "mtu2",
        "packet2",
        "cipher2",
        "link2",
        "bridge2",
        "token2",
        "endpoint2",
        "policy2",
        "snapshot2",
        "guard2",
        "uplink2",
        "dns2",
        "vpn2",
        "key2",
    ],
}


def pick_tokens(pool: list[str], index: int, length: int, seed: int) -> list[str]:
    if length > len(pool):
        raise ValueError(f"length {length} exceeds pool size {len(pool)}")
    start = index + seed * 11 + length * (seed % 7)
    candidates = [1, 5, 7, 11, 13, 17, 19, 23, 29, 31]
    rotated = candidates[seed % len(candidates) :] + candidates[: seed % len(candidates)]
    stride = next(candidate for candidate in rotated if math.gcd(candidate, len(pool)) == 1)
    return [pool[(start + offset * stride) % len(pool)] for offset in range(length)]


def rotate(items: list[str], shift: int) -> list[str]:
    shift %= len(items)
    return items[shift:] + items[:shift]


def chunks(items: list[str], size: int) -> list[list[str]]:
    return [items[index : index + size] for index in range(0, len(items), size)]


def order_transform(rule_family: str, tokens: list[str]) -> list[str]:
    if rule_family == "full_mirror":
        return list(reversed(tokens))
    if rule_family == "rotate_left_1":
        return rotate(tokens, 1)
    if rule_family == "rotate_right_1":
        return rotate(tokens, -1)
    if rule_family == "rotate_left_2":
        return rotate(tokens, 2)
    if rule_family == "rotate_left_3":
        return rotate(tokens, 3)
    if rule_family == "pair_swap":
        out = tokens[:]
        for index in range(0, len(out) - 1, 2):
            out[index], out[index + 1] = out[index + 1], out[index]
        if len(out) % 2 == 1:
            out = rotate(out, -1)
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
    if rule_family == "center_to_edge":
        order = []
        left = (len(tokens) - 1) // 2
        right = left + 1
        while left >= 0 or right < len(tokens):
            if left >= 0:
                order.append(left)
                left -= 1
            if right < len(tokens):
                order.append(right)
                right += 1
        return [tokens[index] for index in order]
    if rule_family == "even_odd_split":
        return tokens[::2] + tokens[1::2]
    if rule_family == "odd_even_split":
        return tokens[1::2] + tokens[::2]
    if rule_family == "interleave_halves":
        midpoint = (len(tokens) + 1) // 2
        left = tokens[:midpoint]
        right = tokens[midpoint:]
        out = []
        for index in range(midpoint):
            out.append(left[index])
            if index < len(right):
                out.append(right[index])
        return out
    if rule_family == "deinterleave_halves":
        return tokens[::2] + tokens[1::2]
    if rule_family == "stride_gather_3":
        return tokens[::3] + tokens[1::3] + tokens[2::3]
    if rule_family == "window_reverse_3":
        out = []
        for chunk in chunks(tokens, 3):
            out.extend(reversed(chunk))
        return out
    if rule_family == "block_reverse_4":
        out = []
        for chunk in chunks(tokens, 4):
            out.extend(reversed(chunk))
        return out
    if rule_family == "perfect_shuffle":
        midpoint = (len(tokens) + 1) // 2
        left = tokens[:midpoint]
        right = tokens[midpoint:]
        out = []
        for index in range(max(len(left), len(right))):
            if index < len(right):
                out.append(right[index])
            if index < len(left):
                out.append(left[index])
        return out
    raise ValueError(f"unknown order rule: {rule_family}")


def edit_marker(split: str, task_index: int) -> str:
    return f"mark_{split}_{SEED}_{task_index % 97}"


def edit_transform(rule_family: str, tokens: list[str], marker: str) -> list[str]:
    midpoint = len(tokens) // 2
    if rule_family == "delete_first":
        return tokens[1:]
    if rule_family == "delete_last":
        return tokens[:-1]
    if rule_family == "delete_middle":
        return tokens[:midpoint] + tokens[midpoint + 1 :]
    if rule_family == "insert_head_marker":
        return [marker] + tokens
    if rule_family == "insert_tail_marker":
        return tokens + [marker]
    if rule_family == "insert_middle_marker":
        return tokens[:midpoint] + [marker] + tokens[midpoint:]
    if rule_family == "duplicate_first":
        return [tokens[0]] + tokens
    if rule_family == "duplicate_last":
        return tokens + [tokens[-1]]
    if rule_family == "duplicate_middle":
        return tokens[:midpoint] + [tokens[midpoint]] + tokens[midpoint:]
    if rule_family == "replace_first_marker":
        return [marker] + tokens[1:]
    if rule_family == "replace_last_marker":
        return tokens[:-1] + [marker]
    if rule_family == "drop_every_third":
        return [token for index, token in enumerate(tokens) if index % 3 != 2]
    raise ValueError(f"unknown edit rule: {rule_family}")


def conditional_branches(rule_family: str) -> tuple[str, str, str]:
    if rule_family == "if_alpha_mirror_else_rotate_left":
        return "alpha", "full_mirror", "rotate_left_1"
    if rule_family == "if_alpha_pair_swap_else_block_swap":
        return "alpha", "pair_swap", "block_swap"
    if rule_family == "if_alpha_even_odd_else_odd_even":
        return "alpha", "even_odd_split", "odd_even_split"
    if rule_family == "if_alpha_edge_center_else_center_edge":
        return "alpha", "edge_to_center", "center_to_edge"
    if rule_family == "if_beta_rotate_right_else_mirror":
        return "beta", "rotate_right_1", "full_mirror"
    if rule_family == "if_beta_block_swap_else_pair_swap":
        return "beta", "block_swap", "pair_swap"
    if rule_family == "if_beta_center_edge_else_rotate_left":
        return "beta", "center_to_edge", "rotate_left_1"
    if rule_family == "if_beta_stride3_else_window3":
        return "beta", "stride_gather_3", "window_reverse_3"
    raise ValueError(f"unknown conditional rule: {rule_family}")


def conditional_transform(rule_family: str, tokens: list[str], flag: str) -> tuple[list[str], list[str]]:
    trigger, then_rule, else_rule = conditional_branches(rule_family)
    then_tokens = order_transform(then_rule, tokens)
    else_tokens = order_transform(else_rule, tokens)
    return (then_tokens, else_tokens) if flag == trigger else (else_tokens, then_tokens)


def composed_steps(rule_family: str) -> tuple[str, str]:
    if rule_family == "mirror_then_rotate_left":
        return "full_mirror", "rotate_left_1"
    if rule_family == "rotate_left_then_mirror":
        return "rotate_left_1", "full_mirror"
    if rule_family == "pair_swap_then_mirror":
        return "pair_swap", "full_mirror"
    if rule_family == "block_swap_then_rotate_right":
        return "block_swap", "rotate_right_1"
    if rule_family == "even_odd_then_mirror":
        return "even_odd_split", "full_mirror"
    if rule_family == "mirror_then_pair_swap":
        return "full_mirror", "pair_swap"
    if rule_family == "edge_center_then_rotate_left":
        return "edge_to_center", "rotate_left_1"
    if rule_family == "rotate_left_then_even_odd":
        return "rotate_left_1", "even_odd_split"
    raise ValueError(f"unknown composed rule: {rule_family}")


def composed_transform(rule_family: str, tokens: list[str]) -> tuple[list[str], list[str]]:
    first, second = composed_steps(rule_family)
    good = order_transform(second, order_transform(first, tokens))
    wrong = order_transform(first, order_transform(second, tokens))
    if wrong == good:
        wrong = order_transform(first, tokens)
    return good, wrong


def same_bag_deranged(left: list[str], right: list[str]) -> bool:
    return sorted(left) == sorted(right) and len(left) == len(right) and all(a != b for a, b in zip(left, right))


def rotate_until_deranged(candidate: list[str], good: list[str], salt: int) -> list[str]:
    if same_bag_deranged(candidate, good):
        return candidate
    for shift in range(1, len(good)):
        rotated = rotate(candidate, shift + salt)
        if same_bag_deranged(rotated, good):
            return rotated
    for shift in range(1, len(good)):
        rotated = rotate(good, shift)
        if same_bag_deranged(rotated, good):
            return rotated
    raise ValueError("cannot build same-bag derangement")


def order_slots_for(rule_family: str, length: int) -> list[int]:
    return [int(item) for item in order_transform(rule_family, list(range(length)))]


def demo_tokens(prefix: str, length: int) -> list[str]:
    return [f"{prefix}{index}" for index in range(length)]


def action_example(operator_class: str, rule_family: str, length: int, marker: str, flag: str | None) -> str:
    demo = demo_tokens("d", min(length, 8))
    if operator_class == "order":
        slots = order_slots_for(rule_family, length)
        return (
            f"operator_class: order; operator_family: {rule_family}; "
            f"operator_slots: {' '.join(f'src{slot}' for slot in slots)}; "
            f"demo: {' '.join(demo)} -> {' '.join(order_transform(rule_family, demo))}; "
            "apply the same source-slot order"
        )
    if operator_class == "edit":
        demo = demo_tokens("d", length)
        return (
            f"operator_class: edit; operator_family: {rule_family}; marker: {marker}; "
            f"demo: {' '.join(demo)} -> {' '.join(edit_transform(rule_family, demo, marker))}; "
            "apply the same edit to the real sequence"
        )
    if operator_class == "conditional":
        trigger, then_rule, else_rule = conditional_branches(rule_family)
        then_slots = order_slots_for(then_rule, length)
        else_slots = order_slots_for(else_rule, length)
        return (
            f"operator_class: conditional; operator_family: {rule_family}; "
            f"if flag_{trigger} use {then_rule} else use {else_rule}; "
            f"then_slots: {' '.join(f'src{slot}' for slot in then_slots)}; "
            f"else_slots: {' '.join(f'src{slot}' for slot in else_slots)}; "
            f"demo_then: {' '.join(demo)} -> {' '.join(order_transform(then_rule, demo))}; "
            f"demo_else: {' '.join(demo)} -> {' '.join(order_transform(else_rule, demo))}; "
            "read current flag only from state_before condition"
        )
    if operator_class == "composed":
        first, second = composed_steps(rule_family)
        demo = demo_tokens("d", length)
        after_first = order_transform(first, demo)
        after_second = order_transform(second, after_first)
        return (
            f"operator_class: composed; operator_family: {rule_family}; "
            f"steps: {first} then {second}; "
            f"demo: {' '.join(demo)} -> {' '.join(after_first)} -> {' '.join(after_second)}; "
            "apply the same two-step operator chain"
        )
    raise ValueError(f"unknown operator class: {operator_class}")


def state_before(split: str, task_index: int, noise_type: str, tokens: list[str], flag: str | None) -> str:
    sequence = " ".join(tokens)
    flag_text = f"; condition: flag_{flag}" if flag is not None else ""
    if noise_type == "clean":
        return f"state: sequence: {sequence}{flag_text}"
    if noise_type == "prefix_suffix":
        return f"state: note_{split}_{SEED}_{task_index}; sequence: {sequence}{flag_text}; tail operator_probe"
    if noise_type == "distractor":
        return f"state: distractor: {' '.join(reversed(tokens))}; sequence: {sequence}{flag_text}; use real span only"
    if noise_type == "instruction_noise":
        return f"state: please transform carefully; keep operator stable; sequence: {sequence}{flag_text}; thanks"
    raise ValueError(f"unknown noise type: {noise_type}")


def class_transform(
    operator_class: str,
    rule_family: str,
    source: list[str],
    marker: str,
    flag: str | None,
    task_index: int,
) -> tuple[list[str], list[str], str]:
    if operator_class == "order":
        good = order_transform(rule_family, source)
        bad = rotate_until_deranged(good, good, task_index + 1)
        return good, bad, "same_bag_derangement"
    if operator_class == "edit":
        good = edit_transform(rule_family, source, marker)
        partner = {
            "delete_first": "delete_last",
            "delete_last": "delete_first",
            "delete_middle": "delete_first",
            "insert_head_marker": "insert_tail_marker",
            "insert_tail_marker": "insert_head_marker",
            "insert_middle_marker": "insert_head_marker",
            "duplicate_first": "duplicate_last",
            "duplicate_last": "duplicate_first",
            "duplicate_middle": "duplicate_first",
            "replace_first_marker": "replace_last_marker",
            "replace_last_marker": "replace_first_marker",
            "drop_every_third": "delete_middle",
        }[rule_family]
        bad = edit_transform(partner, source, marker)
        return good, bad, f"near_edit_{partner}"
    if operator_class == "conditional":
        assert flag is not None
        good, bad = conditional_transform(rule_family, source, flag)
        bad = rotate_until_deranged(bad, good, task_index + 3)
        return good, bad, "wrong_branch_same_bag"
    if operator_class == "composed":
        good, bad = composed_transform(rule_family, source)
        bad = rotate_until_deranged(bad, good, task_index + 5)
        return good, bad, "wrong_composition_same_bag"
    raise ValueError(f"unknown operator class: {operator_class}")


def unique_families_for_length(operator_class: str, length: int) -> list[str]:
    selected = []
    seen: dict[tuple[str, ...], str] = {}
    probe = [str(index) for index in range(length)]
    for family in CLASS_FAMILIES[operator_class]:
        marker = "mark_probe"
        flag = "alpha"
        good, _, _ = class_transform(operator_class, family, probe, marker, flag, 0)
        signature = tuple(good)
        if signature in seen:
            continue
        seen[signature] = family
        selected.append(family)
    return selected


def row(
    task_index: int,
    operator_class: str,
    split: str,
    length: int,
    rule_family: str,
    surface_family: str,
    noise_type: str,
    sample_index: int,
) -> dict[str, object]:
    pool = TRAIN_POOLS[surface_family] if split == "train" else HELDOUT_POOLS[surface_family]
    source = pick_tokens(pool, task_index + sample_index, length, SEED)
    marker = edit_marker(split, task_index)
    flag = None
    if operator_class == "conditional":
        flag = "alpha" if (task_index + sample_index + SEED) % 2 == 0 else "beta"
    good, bad, negative_strategy = class_transform(
        operator_class, rule_family, source, marker, flag, task_index
    )
    rule_id = f"{operator_class}_{rule_family}_len{length}"
    return {
        "schema_version": "operator_battery_v4",
        "task_id": f"opbat_v4_s{SEED:03d}_{operator_class}_{task_index:07d}",
        "language": "mixed-symbolic-ru-en",
        "operator_class": operator_class,
        "source_group": f"operator_battery_{operator_class}_{split}_{rule_id}",
        "surface_family": surface_family,
        "noise_type": noise_type,
        "sequence_length": length,
        "proof_rule_id": rule_id,
        "proof_rule_family": rule_family,
        "state_before": state_before(split, task_index, noise_type, source, flag),
        "rule_action_example": action_example(operator_class, rule_family, length, marker, flag),
        "state_after_correct": f"state: {' '.join(good)}",
        "state_after_wrong": f"state: {' '.join(bad)}",
        "source_tokens": source,
        "correct_tokens": good,
        "wrong_tokens": bad,
        "condition_flag": flag,
        "answer_status": "PROVEN",
        "why_target_is_correct": "The target applies the operator described by rule_action_example to state_before.",
        "why_negative_is_wrong": "The negative applies a near operator or wrong branch while preserving surface pressure.",
        "negative_strategy": negative_strategy,
        "shortcut_risk": [
            "exact_lookup",
            "proof_rule_id_authority",
            "operator_class_majority",
            "length_only",
            "output_position_prior",
            "bag_of_tokens",
            "markov_bigram",
            "bayesian_cooccurrence",
            "l2_neighbor_target_copy",
        ],
        "quality_status": "accepted",
    }


def build_class(operator_class: str, task_index: int) -> tuple[list[dict[str, object]], int]:
    rows: list[dict[str, object]] = []
    for split, per_cell in [("train", TRAIN_PER_CELL), ("heldout", HELDOUT_PER_CELL)]:
        for length in LENGTHS:
            for rule_family in unique_families_for_length(operator_class, length):
                for surface_family in SURFACE_FAMILIES:
                    if PAIRED_NOISE:
                        for sample_index in range(per_cell):
                            base_task_index = task_index
                            for noise_type in NOISE_TYPES:
                                item = row(
                                    base_task_index,
                                    operator_class,
                                    split,
                                    length,
                                    rule_family,
                                    surface_family,
                                    noise_type,
                                    sample_index,
                                )
                                item["task_id"] = f"{item['task_id']}_{noise_type}"
                                rows.append(item)
                            task_index += 1
                    else:
                        for noise_type in NOISE_TYPES:
                            for sample_index in range(per_cell):
                                rows.append(
                                    row(
                                        task_index,
                                        operator_class,
                                        split,
                                        length,
                                        rule_family,
                                        surface_family,
                                        noise_type,
                                        sample_index,
                                    )
                                )
                                task_index += 1
    return rows, task_index


def write_jsonl(path: Path, rows: list[dict[str, object]]) -> None:
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def manifest_for(operator_class: str, rows: list[dict[str, object]]) -> dict[str, object]:
    train = [row for row in rows if f"_{operator_class}_train_" in str(row["source_group"])]
    heldout = [row for row in rows if f"_{operator_class}_heldout_" in str(row["source_group"])]
    same_bag = sum(
        Counter(row["correct_tokens"]) == Counter(row["wrong_tokens"]) for row in rows
    )
    same_length = sum(
        len(row["correct_tokens"]) == len(row["wrong_tokens"]) for row in rows
    )
    return {
        "schema_version": "operator_battery_manifest_v4",
        "operator_class": operator_class,
        "rows": len(rows),
        "train_rows": len(train),
        "heldout_rows": len(heldout),
        "train_per_cell": TRAIN_PER_CELL,
        "heldout_per_cell": HELDOUT_PER_CELL,
        "seed": SEED,
        "paired_noise": PAIRED_NOISE,
        "lengths": LENGTHS,
        "rule_families": CLASS_FAMILIES[operator_class],
        "selected_rule_families_by_length": {
            str(length): unique_families_for_length(operator_class, length)
            for length in LENGTHS
        },
        "surface_families": SURFACE_FAMILIES,
        "noise_types": NOISE_TYPES,
        "matrix_cells": sum(
            len(unique_families_for_length(operator_class, length))
            for length in LENGTHS
        )
        * len(SURFACE_FAMILIES)
        * len(NOISE_TYPES),
        "rules": dict(sorted(Counter(str(row["proof_rule_id"]) for row in rows).items())),
        "negative_strategies": dict(sorted(Counter(str(row["negative_strategy"]) for row in rows).items())),
        "correct_wrong_same_bag_milli": round(1000 * same_bag / len(rows)) if rows else 0,
        "correct_wrong_same_length_milli": round(1000 * same_length / len(rows)) if rows else 0,
        "train_heldout_overlap_by_surface": {
            surface: sorted(set(TRAIN_POOLS[surface]) & set(HELDOUT_POOLS[surface]))
            for surface in SURFACE_FAMILIES
        },
    }


def main() -> int:
    task_index = 0
    combined: list[dict[str, object]] = []
    manifests = {}
    OUTPUT_ROOT.mkdir(parents=True, exist_ok=True)
    for operator_class in CLASSES:
        if operator_class not in CLASS_FAMILIES:
            raise ValueError(f"unknown operator class: {operator_class}")
        rows, task_index = build_class(operator_class, task_index)
        class_dir = OUTPUT_ROOT / operator_class
        class_dir.mkdir(parents=True, exist_ok=True)
        write_jsonl(class_dir / "accepted_operator_tasks_v4.jsonl", rows)
        manifest = manifest_for(operator_class, rows)
        (class_dir / "manifest.json").write_text(
            json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        manifests[operator_class] = manifest
        combined.extend(rows)

    write_jsonl(OUTPUT_ROOT / "accepted_operator_tasks_v4.jsonl", combined)
    combined_manifest = {
        "schema_version": "operator_battery_combined_manifest_v4",
        "rows": len(combined),
        "paired_noise": PAIRED_NOISE,
        "classes": CLASSES,
        "class_manifests": manifests,
        "output_root": str(OUTPUT_ROOT),
    }
    (OUTPUT_ROOT / "manifest.json").write_text(
        json.dumps(combined_manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(combined_manifest, ensure_ascii=False, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
