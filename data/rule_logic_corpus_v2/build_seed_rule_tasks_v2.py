#!/usr/bin/env python3
"""Build a harder multiple-choice Rule Logic v2 corpus."""

from __future__ import annotations

import argparse
import json
import random
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parent
OUT = ROOT / "accepted_rule_tasks_v2.jsonl"
MANIFEST = ROOT / "manifest.json"
RNG_SEED = 20260630
LABELS = ["A", "B", "C", "D"]

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


def pack_choices(
    rng: random.Random,
    correct: str,
    wrongs: list[str],
) -> tuple[str, str, str]:
    values = []
    seen = set()
    for value in [correct, *wrongs]:
        if value not in seen:
            values.append(value)
            seen.add(value)
    while len(values) < len(LABELS):
        filler = f"none_{rng.randrange(10_000)}"
        if filler not in seen:
            values.append(filler)
            seen.add(filler)
    values = values[: len(LABELS)]
    rng.shuffle(values)
    label_by_value = dict(zip(values, LABELS, strict=True))
    choices = "; ".join(f"{label}={value}" for value, label in zip(values, LABELS, strict=True))
    target = f"choice={label_by_value[correct]}"
    wrong_value = rng.choice([value for value in values if value != correct])
    near_negative = f"choice={label_by_value[wrong_value]}"
    return choices, target, near_negative


def row(
    index: int,
    rule_id: str,
    surface_family: str,
    prompt: str,
    correct: str,
    wrongs: list[str],
    answer_status: str,
    why_target: str,
    why_negative: str,
    risks: list[str],
    rng: random.Random,
) -> dict[str, object]:
    choices, target, near_negative = pack_choices(rng, correct, wrongs)
    return {
        "schema_version": "rule_task_v1",
        "task_id": f"rule_v2_{index:06d}",
        "language": "mixed-symbolic-ru-en",
        "surface_family": surface_family,
        "source_group": f"logic_bucket_{rng.randrange(128):03d}",
        "input": f"{prompt}; choices: {choices}; answer?",
        "target": target,
        "near_negative": near_negative,
        "answer_status": answer_status,
        "proof_rule_id": rule_id,
        "proof_rule_family": RULE_FAMILY[rule_id],
        "why_target_is_correct": why_target,
        "why_negative_is_wrong": why_negative,
        "shortcut_risk": risks,
        "quality_status": "accepted",
    }


def make_task(index: int, rule_id: str, rng: random.Random) -> dict[str, object]:
    if rule_id == "alternation_next":
        surface, (a, b) = surface_tokens(rng, 2)
        return row(index, rule_id, surface, f"continue: {a} {b} {a} {b} {a} ?", b, [a],
                   "PROVEN", "The sequence alternates two values.", "The negative repeats the previous value.", ["label_bias", "symbol_frequency"], rng)

    if rule_id == "repeat_block_next":
        surface, (a, b, c) = surface_tokens(rng, 3)
        return row(index, rule_id, surface, f"continue block: {a} {b} {c} {a} {b} ?", c, [a, b],
                   "PROVEN", "The repeated block has three items.", "The negative restarts the block early.", ["label_bias", "block_copy"], rng)

    if rule_id == "increase_by_step":
        start = rng.randrange(1, 50)
        step = rng.randrange(2, 9)
        seq = [start + step * i for i in range(4)]
        correct = str(seq[-1] + step)
        return row(index, rule_id, "mixed_choice", f"continue: {' '.join(map(str, seq))} ?", correct, [str(int(correct) + 1), str(int(correct) - step)],
                   "PROVEN", "The numeric step is constant and positive.", "The negative uses a wrong step.", ["label_bias", "number_prior"], rng)

    if rule_id == "decrease_by_step":
        start = rng.randrange(60, 150)
        step = rng.randrange(2, 11)
        seq = [start - step * i for i in range(4)]
        correct = str(seq[-1] - step)
        return row(index, rule_id, "mixed_choice", f"continue: {' '.join(map(str, seq))} ?", correct, [str(int(correct) + 1), str(int(correct) + step)],
                   "PROVEN", "The numeric step is constant and negative.", "The negative uses a wrong step.", ["label_bias", "number_prior"], rng)

    if rule_id == "missing_order_item":
        surface, chain = surface_tokens(rng, 5)
        missing = rng.randrange(1, 4)
        visible = chain.copy()
        correct = visible[missing]
        visible[missing] = "?"
        return row(index, rule_id, surface, f"order: {' < '.join(chain)}; row: {' '.join(visible)}", correct, [chain[missing - 1], chain[missing + 1]],
                   "PROVEN", "The missing value is fixed by the order chain.", "The negative is only a neighbor.", ["label_bias", "neighbor_bias"], rng)

    if rule_id == "mirror_complete":
        surface, (a, b, c) = surface_tokens(rng, 3)
        return row(index, rule_id, surface, f"mirror: {a} {b} {c} | {c} {b} ?", a, [b, c],
                   "PROVEN", "The second side mirrors the first.", "The negative does not close the mirror.", ["label_bias", "local_neighbor"], rng)

    if rule_id in {"cyclic_shift_left", "cyclic_shift_right"}:
        surface, (a, b, c, d, e, f) = surface_tokens(rng, 6)
        if rule_id == "cyclic_shift_left":
            prompt = f"rule: {a} {b} {c} -> {b} {c} {a}; apply: {d} {e} {f}"
            correct, wrong = f"{e} {f} {d}", f"{f} {d} {e}"
            why = "The first item moves to the end."
        else:
            prompt = f"rule: {a} {b} {c} -> {c} {a} {b}; apply: {d} {e} {f}"
            correct, wrong = f"{f} {d} {e}", f"{e} {f} {d}"
            why = "The last item moves to the front."
        return row(index, rule_id, surface, prompt, correct, [wrong, f"{d} {e} {f}"],
                   "PROVEN", why, "The negative applies the wrong direction.", ["label_bias", "direction_swap"], rng)

    if rule_id == "analogy_replace_feature":
        old, new = sample(rng, COLORS, 2)
        shape_a, shape_b = sample(rng, SHAPES, 2)
        return row(index, rule_id, "feature_choice", f"{old} {shape_a} : {new} {shape_a} = {old} {shape_b} : ?", f"{new} {shape_b}", [f"{old} {shape_b}", f"{new} {shape_a}"],
                   "PROVEN", "The analogy changes color and preserves shape.", "The negative misses one feature relation.", ["label_bias", "feature_copy"], rng)

    if rule_id == "classify_shared_feature":
        color = rng.choice(COLORS)
        shapes = sample(rng, SHAPES, 3)
        return row(index, rule_id, "feature_choice", f"shared feature: {color} {shapes[0]}; {color} {shapes[1]}; {color} {shapes[2]}", color, [shapes[0], shapes[1]],
                   "PROVEN", "All examples share the same color.", "The negative is an object feature, not the shared class.", ["label_bias", "feature_frequency"], rng)

    if rule_id == "odd_one_out":
        common = rng.choice(COLORS)
        odd = rng.choice([color for color in COLORS if color != common])
        shapes = sample(rng, SHAPES, 4)
        items = [f"{common} {shape}" for shape in shapes[:3]] + [f"{odd} {shapes[3]}"]
        rng.shuffle(items)
        correct = next(item for item in items if item.startswith(odd))
        wrong = next(item for item in items if item.startswith(common))
        return row(index, rule_id, "feature_choice", f"odd one: {'; '.join(items)}", correct, [wrong],
                   "PROVEN", "One item has a different color from the majority.", "The negative belongs to the majority.", ["label_bias", "majority_bias"], rng)

    if rule_id == "intersection_feature":
        common, left, right = sample(rng, FEATURES, 3)
        return row(index, rule_id, "set_choice", f"A={{ {common}, {left} }}; B={{ {right}, {common} }}; common?", common, [left, right],
                   "PROVEN", "The target appears in both sets.", "The negative appears in only one set.", ["label_bias", "set_position"], rng)

    if rule_id == "union_feature":
        left, middle, right = sample(rng, FEATURES, 3)
        correct = " ".join(sorted({left, middle, right}))
        return row(index, rule_id, "set_choice", f"A={{ {left}, {middle} }}; B={{ {middle}, {right} }}; union?", correct, [" ".join(sorted({left, middle})), " ".join(sorted({middle, right}))],
                   "PROVEN", "Union keeps every item from either set.", "The negative drops one side.", ["label_bias", "partial_copy"], rng)

    if rule_id == "negation_flip":
        flag = rng.choice(["enabled", "paid", "verified", "online"])
        return row(index, rule_id, "logic_choice", f"rule: {flag}->allow; not_{flag}->block; case: not_{flag}", "block", ["allow"],
                   "PROVEN", "The negated case uses the block branch.", "The negative ignores negation.", ["label_bias", "positive_branch_bias"], rng)

    if rule_id == "if_then_apply":
        condition = rng.choice(["paid", "signed", "approved", "arrived"])
        yes = rng.choice(["ship", "release", "notify", "open"])
        no = rng.choice(["hold", "wait", "request", "block"])
        truth = rng.choice([True, False])
        case = condition if truth else f"not_{condition}"
        correct, wrong = (yes, no) if truth else (no, yes)
        return row(index, rule_id, "logic_choice", f"if {condition} then {yes}; else {no}; case: {case}", correct, [wrong],
                   "PROVEN", "The branch follows the stated condition.", "The negative chooses the opposite branch.", ["label_bias", "branch_prior"], rng)

    if rule_id == "required_variable_missing":
        route = rng.choice(["SPb-Msk", "warehouse-client", "port-terminal", "office-airport"])
        variable = rng.choice(["transport", "start_time", "cargo_weight", "destination_point"])
        return row(index, rule_id, "evidence_choice", f"estimate route {route}; required variable missing: {variable}", f"UNSETTLED ask {variable}", ["PROVEN give number", "LIKELY guess"],
                   "UNSETTLED", "A required variable is missing.", "The negative pretends certainty.", ["label_bias", "overconfidence"], rng)

    if rule_id == "evidence_conflict":
        fact = rng.choice(["paid", "signed", "delivered", "approved"])
        return row(index, rule_id, "evidence_choice", f"source_a says {fact}; source_b says not_{fact}", f"CONFLICT verify {fact}", [f"PROVEN {fact}", f"PROVEN not_{fact}"],
                   "CONFLICT", "Two sources contradict each other.", "The negative chooses one side.", ["label_bias", "source_priority_bias"], rng)

    if rule_id == "greater_less_compare":
        a = rng.randrange(1, 40)
        b = rng.randrange(1, 40)
        while b == a:
            b = rng.randrange(1, 40)
        mode = rng.choice(["greater", "less"])
        correct = "A" if (a > b if mode == "greater" else a < b) else "B"
        wrong = "B" if correct == "A" else "A"
        return row(index, rule_id, "mixed_choice", f"A={a}; B={b}; choose {mode}", correct, [wrong],
                   "PROVEN", "The selected label satisfies the comparison.", "The negative is the opposite result.", ["label_bias", "number_prior"], rng)

    if rule_id == "compose_shift_then_replace":
        surface, (a, b, c, x) = surface_tokens(rng, 4)
        return row(index, rule_id, surface, f"steps: shift_left then replace {b}->{x}; input: {a} {b} {c}", f"{x} {c} {a}", [f"{b} {c} {a}", f"{c} {a} {x}"],
                   "PROVEN", "The task applies shift first, then replacement.", "The negative applies only part of the composition.", ["label_bias", "partial_transform"], rng)

    if rule_id == "compose_count_then_compare":
        left = rng.randrange(1, 5)
        right = rng.randrange(1, 5)
        while left == right:
            right = rng.randrange(1, 5)
        correct = "A" if left > right else "B"
        wrong = "B" if correct == "A" else "A"
        return row(index, rule_id, "count_choice", f"A has {'red ' * left}blue; B has {'red ' * right}blue; more red", correct, [wrong],
                   "PROVEN", "The task counts red items, then compares.", "The negative chooses the lower count.", ["label_bias", "length_bias"], rng)

    raise ValueError(rule_id)


def build(count: int) -> list[dict[str, object]]:
    rng = random.Random(RNG_SEED)
    return [make_task(index, RULE_IDS[index % len(RULE_IDS)], rng) for index in range(count)]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--count", type=int, default=12000)
    args = parser.parse_args()
    rows = build(args.count)
    with OUT.open("w", encoding="utf-8") as handle:
        for item in rows:
            handle.write(json.dumps(item, ensure_ascii=False, sort_keys=True) + "\n")
    manifest = {
        "schema_version": "rule_logic_corpus_manifest_v2",
        "rows": len(rows),
        "rng_seed": RNG_SEED,
        "target_contract": "multiple_choice_label_only",
        "rules": dict(sorted(Counter(str(row["proof_rule_id"]) for row in rows).items())),
        "surface_families": dict(sorted(Counter(str(row["surface_family"]) for row in rows).items())),
        "answer_status": dict(sorted(Counter(str(row["answer_status"]) for row in rows).items())),
        "training_authority": "input,target,near_negative,answer_status",
        "proof_rule_id_training_authority": False,
    }
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {OUT} rows={len(rows)}")
    print(f"wrote {MANIFEST}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
