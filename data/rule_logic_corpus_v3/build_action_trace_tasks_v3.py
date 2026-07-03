#!/usr/bin/env python3
"""Build solved action-trace tasks for L3."""

from __future__ import annotations

import argparse
import json
import random
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parent
OUT = ROOT / "accepted_action_trace_tasks_v3.jsonl"
MANIFEST = ROOT / "manifest.json"
RNG_SEED = 20260630

RULE_FAMILY = {
    "alternation_next": "sequence",
    "repeat_block_next": "sequence",
    "increase_by_step": "quantity",
    "decrease_by_step": "quantity",
    "missing_order_item": "order",
    "mirror_complete": "symmetry",
    "cyclic_shift_left": "shift",
    "cyclic_shift_right": "shift",
    "analogy_replace_feature": "analogy",
    "classify_shared_feature": "classification",
    "odd_one_out": "classification",
    "intersection_feature": "set",
    "union_feature": "set",
    "negation_flip": "logic",
    "if_then_apply": "logic",
    "required_variable_missing": "evidence",
    "evidence_conflict": "evidence",
    "greater_less_compare": "quantity",
    "compose_shift_then_replace": "composition",
    "compose_count_then_compare": "composition",
}
RULE_IDS = list(RULE_FAMILY)

SYMBOLS = list("ABCDEFGHJKLMNPQRSTUVWXYZ")
WORDS = [
    "utro",
    "vecher",
    "sever",
    "yug",
    "vhod",
    "vyhod",
    "krasnyy",
    "siniy",
    "krug",
    "kvadrat",
    "platezh",
    "dostavka",
    "sklad",
    "zayavka",
    "schet",
    "akt",
]
COLORS = ["red", "blue", "green", "yellow", "black", "white"]
SHAPES = ["circle", "square", "triangle", "star", "line", "cross"]
FEATURES = ["red", "blue", "round", "small", "large", "metal", "soft", "paid"]


def sample(rng: random.Random, pool: list[str], count: int) -> list[str]:
    return rng.sample(pool, count)


def surface_tokens(rng: random.Random, count: int) -> tuple[str, list[str]]:
    surface = rng.choice(["symbols", "ru_words"])
    pool = SYMBOLS if surface == "symbols" else WORDS
    return surface, sample(rng, pool, count)


def row(
    index: int,
    rule_id: str,
    surface_family: str,
    state_before: str,
    rule_action_example: str,
    correct: str,
    wrong: str,
    answer_status: str,
    why_target: str,
    why_wrong: str,
    risks: list[str],
    rng: random.Random,
) -> dict[str, object]:
    return {
        "schema_version": "rule_action_trace_v1",
        "task_id": f"trace_v3_{index:06d}",
        "language": "mixed-symbolic-ru-en",
        "surface_family": surface_family,
        "source_group": f"trace_bucket_{rng.randrange(128):03d}",
        "state_before": state_before,
        "rule_action_example": rule_action_example,
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


def make_task(index: int, rule_id: str, rng: random.Random) -> dict[str, object]:
    if rule_id == "alternation_next":
        surface, (x, y, a, b) = surface_tokens(rng, 4)
        before = f"state: {a} {b} {a} {b} {a}"
        demo = f"action_demo: {x} {y} {x} -> {y}; action: continue alternation"
        return row(index, rule_id, surface, before, demo, f"state: {a} {b} {a} {b} {a} {b}", f"state: {a} {b} {a} {b} {a} {a}", "PROVEN", "The action continues the alternating pair.", "The wrong state repeats the previous item.", ["surface_copy", "last_token_bias"], rng)

    if rule_id == "repeat_block_next":
        surface, (x, y, z, a, b, c) = surface_tokens(rng, 6)
        before = f"state: {a} {b} {c} {a} {b}"
        demo = f"action_demo: {x} {y} {z} {x} {y} -> {z}; action: complete repeated block"
        return row(index, rule_id, surface, before, demo, f"state: {a} {b} {c} {a} {b} {c}", f"state: {a} {b} {c} {a} {b} {a}", "PROVEN", "The action completes the current block.", "The wrong state restarts the block.", ["block_copy", "first_token_bias"], rng)

    if rule_id == "increase_by_step":
        step = rng.randrange(2, 9)
        start = rng.randrange(1, 60)
        demo_start = rng.randrange(80, 150)
        seq = [start + step * i for i in range(3)]
        demo = [demo_start + step * i for i in range(3)]
        correct = seq + [seq[-1] + step]
        wrong = seq + [seq[-1] + step + 1]
        return row(index, rule_id, "numbers_trace", f"state: {' '.join(map(str, seq))}", f"action_demo: {' '.join(map(str, demo))} -> {demo[-1] + step}; action: add same step", f"state: {' '.join(map(str, correct))}", f"state: {' '.join(map(str, wrong))}", "PROVEN", "The action adds the same positive step.", "The wrong state uses a different final step.", ["last_number_bias", "number_prior"], rng)

    if rule_id == "decrease_by_step":
        step = rng.randrange(2, 10)
        start = rng.randrange(70, 160)
        demo_start = rng.randrange(180, 260)
        seq = [start - step * i for i in range(3)]
        demo = [demo_start - step * i for i in range(3)]
        correct = seq + [seq[-1] - step]
        wrong = seq + [seq[-1] - step + 1]
        return row(index, rule_id, "numbers_trace", f"state: {' '.join(map(str, seq))}", f"action_demo: {' '.join(map(str, demo))} -> {demo[-1] - step}; action: subtract same step", f"state: {' '.join(map(str, correct))}", f"state: {' '.join(map(str, wrong))}", "PROVEN", "The action subtracts the same step.", "The wrong state uses a different final step.", ["last_number_bias", "number_prior"], rng)

    if rule_id == "missing_order_item":
        surface, chain = surface_tokens(rng, 5)
        idx = rng.randrange(1, 4)
        visible = chain.copy()
        missing = visible[idx]
        visible[idx] = "?"
        before = f"order: {' < '.join(chain)}; state: {' '.join(visible)}"
        demo = "action_demo: ordered chain fixes missing position; action: fill placeholder"
        correct = f"state: {' '.join(chain)}"
        wrong_chain = chain.copy()
        wrong_chain[idx] = chain[idx - 1]
        return row(index, rule_id, surface, before, demo, correct, f"state: {' '.join(wrong_chain)}", "PROVEN", "The action fills the exact missing ordered item.", "The wrong state copies a neighbor.", ["neighbor_bias", "position_bias"], rng)

    if rule_id == "mirror_complete":
        surface, (x, y, z, a, b, c) = surface_tokens(rng, 6)
        before = f"state: {a} {b} {c} | {c} {b}"
        demo = f"action_demo: {x} {y} {z} | {z} {y} -> {x}; action: close mirror"
        return row(index, rule_id, surface, before, demo, f"state: {a} {b} {c} | {c} {b} {a}", f"state: {a} {b} {c} | {c} {b} {b}", "PROVEN", "The action completes the mirror.", "The wrong state repeats the middle.", ["local_neighbor", "symbol_frequency"], rng)

    if rule_id in {"cyclic_shift_left", "cyclic_shift_right"}:
        surface, (a, b, c, d, e, f) = surface_tokens(rng, 6)
        before = f"state: {d} {e} {f}"
        if rule_id == "cyclic_shift_left":
            demo = f"action_demo: {a} {b} {c} -> {b} {c} {a}; action: rotate left"
            correct, wrong = f"state: {e} {f} {d}", f"state: {f} {d} {e}"
            why = "The action moves the first item to the end."
        else:
            demo = f"action_demo: {a} {b} {c} -> {c} {a} {b}; action: rotate right"
            correct, wrong = f"state: {f} {d} {e}", f"state: {e} {f} {d}"
            why = "The action moves the last item to the front."
        return row(index, rule_id, surface, before, demo, correct, wrong, "PROVEN", why, "The wrong state applies the opposite rotation.", ["direction_swap", "copy_pattern"], rng)

    if rule_id == "analogy_replace_feature":
        old, new = sample(rng, COLORS, 2)
        shape_a, shape_b = sample(rng, SHAPES, 2)
        before = f"state: {old} {shape_b}"
        demo = f"action_demo: {old} {shape_a} -> {new} {shape_a}; action: replace color preserve shape"
        return row(index, rule_id, "feature_trace", before, demo, f"state: {new} {shape_b}", f"state: {old} {shape_b}", "PROVEN", "The action replaces color and preserves shape.", "The wrong state keeps the old color.", ["feature_copy", "surface_overlap"], rng)

    if rule_id == "classify_shared_feature":
        color = rng.choice(COLORS)
        shapes = sample(rng, SHAPES, 3)
        before = f"state: {color} {shapes[0]}; {color} {shapes[1]}; {color} {shapes[2]}"
        demo = "action_demo: same feature across examples -> feature name; action: extract shared feature"
        return row(index, rule_id, "feature_trace", before, demo, f"state: shared {color}", f"state: shared {shapes[0]}", "PROVEN", "The action extracts the common color.", "The wrong state names an object shape.", ["feature_frequency", "first_token_bias"], rng)

    if rule_id == "odd_one_out":
        common = rng.choice(COLORS)
        odd = rng.choice([color for color in COLORS if color != common])
        shapes = sample(rng, SHAPES, 4)
        items = [f"{common} {shape}" for shape in shapes[:3]] + [f"{odd} {shapes[3]}"]
        rng.shuffle(items)
        correct = next(item for item in items if item.startswith(odd))
        wrong = next(item for item in items if item.startswith(common))
        return row(index, rule_id, "feature_trace", f"state: {'; '.join(items)}", "action_demo: majority feature holds; action: select item outside majority", f"state: odd {correct}", f"state: odd {wrong}", "PROVEN", "The action selects the item outside the majority feature.", "The wrong state selects a majority item.", ["majority_bias", "position_bias"], rng)

    if rule_id == "intersection_feature":
        common, left, right = sample(rng, FEATURES, 3)
        before = f"state: A={{ {common}, {left} }}; B={{ {right}, {common} }}"
        return row(index, rule_id, "set_trace", before, "action_demo: item in both sets -> common; action: intersect sets", f"state: common {common}", f"state: common {left}", "PROVEN", "The action keeps only features present in both sets.", "The wrong state keeps a one-sided feature.", ["set_position", "token_overlap"], rng)

    if rule_id == "union_feature":
        left, middle, right = sample(rng, FEATURES, 3)
        before = f"state: A={{ {left}, {middle} }}; B={{ {middle}, {right} }}"
        correct = " ".join(sorted({left, middle, right}))
        wrong = " ".join(sorted({left, middle}))
        return row(index, rule_id, "set_trace", before, "action_demo: item in either set -> union; action: merge sets", f"state: union {correct}", f"state: union {wrong}", "PROVEN", "The action keeps every feature from either set.", "The wrong state drops one side.", ["partial_copy", "set_position"], rng)

    if rule_id == "negation_flip":
        flag = rng.choice(["enabled", "paid", "verified", "online"])
        before = f"state: not_{flag}"
        demo = f"action_demo: {flag}->allow; not_{flag}->block; action: apply negation branch"
        return row(index, rule_id, "logic_trace", before, demo, "state: block", "state: allow", "PROVEN", "The action follows the negated branch.", "The wrong state ignores negation.", ["positive_branch_bias", "negation_drop"], rng)

    if rule_id == "if_then_apply":
        cond = rng.choice(["paid", "signed", "approved", "arrived"])
        yes = rng.choice(["ship", "release", "notify", "open"])
        no = rng.choice(["hold", "wait", "request", "block"])
        truth = rng.choice([True, False])
        case = cond if truth else f"not_{cond}"
        correct, wrong = (yes, no) if truth else (no, yes)
        before = f"state: case {case}"
        demo = f"action_demo: if {cond} then {yes}; else {no}; action: branch"
        return row(index, rule_id, "logic_trace", before, demo, f"state: {correct}", f"state: {wrong}", "PROVEN", "The action applies the stated branch.", "The wrong state chooses the opposite branch.", ["branch_prior", "keyword_bias"], rng)

    if rule_id == "required_variable_missing":
        route = rng.choice(["SPb-Msk", "warehouse-client", "port-terminal", "office-airport"])
        variable = rng.choice(["transport", "start_time", "cargo_weight", "destination_point"])
        before = f"state: estimate {route}; missing {variable}"
        demo = "action_demo: required variable missing -> ask variable; action: refuse proven answer"
        return row(index, rule_id, "evidence_trace", before, demo, f"state: UNSETTLED ask {variable}", "state: PROVEN give number", "UNSETTLED", "The action asks for the missing required variable.", "The wrong state pretends certainty.", ["overconfidence", "default_answer_bias"], rng)

    if rule_id == "evidence_conflict":
        fact = rng.choice(["paid", "signed", "delivered", "approved"])
        before = f"state: source_a {fact}; source_b not_{fact}"
        demo = "action_demo: source conflict -> verify; action: reject single-side proof"
        return row(index, rule_id, "evidence_trace", before, demo, f"state: CONFLICT verify {fact}", f"state: PROVEN {fact}", "CONFLICT", "The action detects conflicting evidence.", "The wrong state chooses one side.", ["source_priority_bias", "overconfidence"], rng)

    if rule_id == "greater_less_compare":
        a = rng.randrange(1, 40)
        b = rng.randrange(1, 40)
        while b == a:
            b = rng.randrange(1, 40)
        mode = rng.choice(["greater", "less"])
        correct = "A" if (a > b if mode == "greater" else a < b) else "B"
        wrong = "B" if correct == "A" else "A"
        before = f"state: A={a}; B={b}; choose {mode}"
        demo = f"action_demo: compare values and select {mode}; action: compare"
        return row(index, rule_id, "numbers_trace", before, demo, f"state: {correct}", f"state: {wrong}", "PROVEN", "The action selects the requested comparison result.", "The wrong state selects the opposite label.", ["label_bias", "number_prior"], rng)

    if rule_id == "compose_shift_then_replace":
        surface, (a, b, c, x) = surface_tokens(rng, 4)
        before = f"state: {a} {b} {c}"
        demo = f"action_demo: shift_left then replace {b}->{x}; action: compose two transforms"
        return row(index, rule_id, surface, before, demo, f"state: {x} {c} {a}", f"state: {b} {c} {a}", "PROVEN", "The action shifts, then replaces.", "The wrong state applies only the shift.", ["partial_transform", "single_rule_shortcut"], rng)

    if rule_id == "compose_count_then_compare":
        left = rng.randrange(1, 5)
        right = rng.randrange(1, 5)
        while left == right:
            right = rng.randrange(1, 5)
        correct = "A" if left > right else "B"
        wrong = "B" if correct == "A" else "A"
        before = f"state: A has {'red ' * left}blue; B has {'red ' * right}blue"
        demo = "action_demo: count red then compare greater count; action: compose count and compare"
        return row(index, rule_id, "count_trace", before, demo, f"state: {correct}", f"state: {wrong}", "PROVEN", "The action counts red items, then compares.", "The wrong state selects the smaller count.", ["length_bias", "label_bias"], rng)

    raise ValueError(rule_id)


def build(count: int) -> list[dict[str, object]]:
    rng = random.Random(RNG_SEED)
    return [make_task(index, RULE_IDS[index % len(RULE_IDS)], rng) for index in range(count)]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--count", type=int, default=12_000)
    args = parser.parse_args()

    rows = build(args.count)
    with OUT.open("w", encoding="utf-8") as handle:
        for item in rows:
            handle.write(json.dumps(item, ensure_ascii=False, sort_keys=True) + "\n")
    manifest = {
        "schema_version": "rule_logic_action_trace_manifest_v3",
        "rows": len(rows),
        "rng_seed": RNG_SEED,
        "rules": dict(sorted(Counter(str(row["proof_rule_id"]) for row in rows).items())),
        "surface_families": dict(sorted(Counter(str(row["surface_family"]) for row in rows).items())),
        "answer_status": dict(sorted(Counter(str(row["answer_status"]) for row in rows).items())),
        "training_shape": "state_before + rule_action_example -> state_after",
        "proof_rule_id_training_authority": False,
    }
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {OUT} rows={len(rows)}")
    print(f"wrote {MANIFEST}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
