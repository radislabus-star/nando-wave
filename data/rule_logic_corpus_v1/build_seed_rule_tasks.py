#!/usr/bin/env python3
"""Build a deterministic solved Rule Logic v1 seed corpus."""

from __future__ import annotations

import argparse
import json
import random
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parent
DEFAULT_OUT = ROOT / "accepted_rule_tasks_v1.jsonl"
DEFAULT_MANIFEST = ROOT / "manifest.json"
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
RU_WORDS = [
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


def pick_distinct(rng: random.Random, pool: list[str], count: int) -> list[str]:
    return rng.sample(pool, count)


def surface_tokens(rng: random.Random, count: int) -> tuple[str, list[str]]:
    surface = rng.choice(["symbols", "ru_words"])
    pool = SYMBOLS if surface == "symbols" else RU_WORDS
    return surface, pick_distinct(rng, pool, count)


def tokens_for_surface(rng: random.Random, surface: str, count: int) -> list[str]:
    pool = SYMBOLS if surface == "symbols" else RU_WORDS
    return pick_distinct(rng, pool, count)


def base_row(
    index: int,
    rule_id: str,
    surface_family: str,
    input_text: str,
    target: str,
    near_negative: str,
    answer_status: str,
    why_target: str,
    why_negative: str,
    risks: list[str],
    rng: random.Random,
) -> dict[str, object]:
    return {
        "schema_version": "rule_task_v1",
        "task_id": f"rule_v1_{index:06d}",
        "language": "mixed-symbolic-ru-en",
        "surface_family": surface_family,
        "source_group": f"logic_bucket_{rng.randrange(96):02d}",
        "input": input_text,
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
        return base_row(
            index,
            rule_id,
            surface,
            f"{a} {b} {a} {b} {a} ?",
            b,
            a,
            "PROVEN",
            "The sequence alternates two values, so the next value is the second one.",
            "The negative repeats the previous value and breaks alternation.",
            ["symbol_frequency", "local_neighbor"],
            rng,
        )

    if rule_id == "repeat_block_next":
        surface, (a, b, c) = surface_tokens(rng, 3)
        return base_row(
            index,
            rule_id,
            surface,
            f"{a} {b} {c} {a} {b} ?",
            c,
            a,
            "PROVEN",
            "The block repeats as three items, so the missing block item is the third.",
            "The negative restarts the block too early.",
            ["block_lookup", "symbol_frequency"],
            rng,
        )

    if rule_id == "increase_by_step":
        start = rng.randrange(1, 40)
        step = rng.randrange(2, 9)
        seq = [start + step * i for i in range(4)]
        target = seq[-1] + step
        return base_row(
            index,
            rule_id,
            "numbers",
            f"{seq[0]} {seq[1]} {seq[2]} {seq[3]} ?",
            str(target),
            str(target + rng.choice([-1, 1, step])),
            "PROVEN",
            "The numeric step is constant and positive.",
            "The negative uses a different final step.",
            ["last_number_bias", "markov_step"],
            rng,
        )

    if rule_id == "decrease_by_step":
        start = rng.randrange(50, 140)
        step = rng.randrange(2, 11)
        seq = [start - step * i for i in range(4)]
        target = seq[-1] - step
        return base_row(
            index,
            rule_id,
            "numbers",
            f"{seq[0]} {seq[1]} {seq[2]} {seq[3]} ?",
            str(target),
            str(target + rng.choice([1, step, -1])),
            "PROVEN",
            "The numeric step is constant and negative.",
            "The negative does not preserve the decreasing step.",
            ["last_number_bias", "markov_step"],
            rng,
        )

    if rule_id == "missing_order_item":
        surface, chain = surface_tokens(rng, 5)
        missing_idx = rng.randrange(1, 4)
        visible = chain.copy()
        target = visible[missing_idx]
        visible[missing_idx] = "?"
        return base_row(
            index,
            rule_id,
            surface,
            f"order: {' < '.join(chain)}; row: {' '.join(visible)}",
            target,
            chain[(missing_idx + 1) % len(chain)],
            "PROVEN",
            "The missing value is fixed by the explicit order chain.",
            "The negative chooses a neighbor from the chain but not the missing position.",
            ["position_bias", "neighbor_bias"],
            rng,
        )

    if rule_id == "mirror_complete":
        surface, (a, b, c) = surface_tokens(rng, 3)
        return base_row(
            index,
            rule_id,
            surface,
            f"{a} {b} {c} | {c} {b} ?",
            a,
            b,
            "PROVEN",
            "The right side mirrors the left side.",
            "The negative duplicates the middle instead of closing the mirror.",
            ["symbol_frequency", "local_neighbor"],
            rng,
        )

    if rule_id == "cyclic_shift_left":
        surface, first = surface_tokens(rng, 3)
        second = tokens_for_surface(rng, surface, 3)
        a, b, c = first
        d, e, f = second
        return base_row(
            index,
            rule_id,
            surface,
            f"rule: {a} {b} {c} -> {b} {c} {a}; apply: {d} {e} {f} -> ?",
            f"{e} {f} {d}",
            f"{f} {d} {e}",
            "PROVEN",
            "The rule moves the first item to the end.",
            "The negative applies the opposite rotation.",
            ["copy_pattern", "direction_swap"],
            rng,
        )

    if rule_id == "cyclic_shift_right":
        surface, first = surface_tokens(rng, 3)
        second = tokens_for_surface(rng, surface, 3)
        a, b, c = first
        d, e, f = second
        return base_row(
            index,
            rule_id,
            surface,
            f"rule: {a} {b} {c} -> {c} {a} {b}; apply: {d} {e} {f} -> ?",
            f"{f} {d} {e}",
            f"{e} {f} {d}",
            "PROVEN",
            "The rule moves the last item to the front.",
            "The negative applies the opposite rotation.",
            ["copy_pattern", "direction_swap"],
            rng,
        )

    if rule_id == "analogy_replace_feature":
        old_color, new_color = pick_distinct(rng, COLORS, 2)
        shape_a, shape_b = pick_distinct(rng, SHAPES, 2)
        return base_row(
            index,
            rule_id,
            "feature_words",
            f"{old_color} {shape_a} : {new_color} {shape_a} = {old_color} {shape_b} : ?",
            f"{new_color} {shape_b}",
            f"{old_color} {shape_b}",
            "PROVEN",
            "The analogy changes color while preserving shape.",
            "The negative keeps the old color and fails to apply the mapping.",
            ["surface_overlap", "feature_copy"],
            rng,
        )

    if rule_id == "classify_shared_feature":
        color = rng.choice(COLORS)
        shapes = pick_distinct(rng, SHAPES, 3)
        return base_row(
            index,
            rule_id,
            "feature_words",
            f"{color} {shapes[0]}; {color} {shapes[1]}; {color} {shapes[2]}; shared?",
            color,
            shapes[0],
            "PROVEN",
            "All examples share the same color feature.",
            "The negative is an object shape, not the shared class feature.",
            ["first_token_bias", "feature_frequency"],
            rng,
        )

    if rule_id == "odd_one_out":
        common = rng.choice(COLORS)
        odd = rng.choice([color for color in COLORS if color != common])
        shapes = pick_distinct(rng, SHAPES, 4)
        items = [f"{common} {shape}" for shape in shapes[:3]] + [f"{odd} {shapes[3]}"]
        rng.shuffle(items)
        target = next(item for item in items if item.startswith(odd))
        negative = next(item for item in items if item.startswith(common))
        return base_row(
            index,
            rule_id,
            "feature_words",
            f"odd one: {'; '.join(items)}",
            target,
            negative,
            "PROVEN",
            "One item has a different color from the other three.",
            "The negative belongs to the majority color group.",
            ["frequency_majority", "position_bias"],
            rng,
        )

    if rule_id == "intersection_feature":
        common = rng.choice(FEATURES)
        left = pick_distinct(rng, [f for f in FEATURES if f != common], 1)[0]
        right = pick_distinct(rng, [f for f in FEATURES if f not in {common, left}], 1)[0]
        return base_row(
            index,
            rule_id,
            "set_words",
            f"A={{ {common}, {left} }}; B={{ {right}, {common} }}; common?",
            common,
            left,
            "PROVEN",
            "The target is the only feature present in both sets.",
            "The negative is present only in the first set.",
            ["set_position", "token_overlap"],
            rng,
        )

    if rule_id == "union_feature":
        left, middle, right = pick_distinct(rng, FEATURES, 3)
        target_items = sorted({left, middle, right})
        negative_items = sorted({left, middle})
        return base_row(
            index,
            rule_id,
            "set_words",
            f"A={{ {left}, {middle} }}; B={{ {middle}, {right} }}; union?",
            " ".join(target_items),
            " ".join(negative_items),
            "PROVEN",
            "Union keeps every feature that appears in either set.",
            "The negative drops a feature from the second set.",
            ["set_position", "partial_copy"],
            rng,
        )

    if rule_id == "negation_flip":
        flag = rng.choice(["enabled", "paid", "verified", "online"])
        return base_row(
            index,
            rule_id,
            "logic_words",
            f"rule: {flag} -> allow; not_{flag} -> block; case: not_{flag}; answer?",
            "block",
            "allow",
            "PROVEN",
            "The case contains negation, so the blocked branch applies.",
            "The negative ignores the negation marker.",
            ["positive_branch_bias", "negation_drop"],
            rng,
        )

    if rule_id == "if_then_apply":
        condition = rng.choice(["paid", "signed", "approved", "arrived"])
        action_true = rng.choice(["ship", "release", "notify", "open"])
        action_false = rng.choice(["hold", "wait", "request", "block"])
        truth = rng.choice([True, False])
        case = condition if truth else f"not_{condition}"
        target = action_true if truth else action_false
        negative = action_false if truth else action_true
        return base_row(
            index,
            rule_id,
            "logic_words",
            f"if {condition} then {action_true}; else {action_false}; case: {case}; next?",
            target,
            negative,
            "PROVEN",
            "The branch follows the stated condition.",
            "The negative chooses the opposite branch.",
            ["branch_prior", "keyword_bias"],
            rng,
        )

    if rule_id == "required_variable_missing":
        route = rng.choice(["SPb-Msk", "warehouse-client", "port-terminal", "office-airport"])
        variable = rng.choice(["transport", "start_time", "cargo_weight", "destination_point"])
        return base_row(
            index,
            rule_id,
            "evidence_words",
            f"estimate route {route}; missing {variable}; answer status?",
            f"UNSETTLED ask_{variable}",
            "PROVEN give_single_number",
            "UNSETTLED",
            "A required variable is missing, so a proven single answer is not allowed.",
            "The negative pretends certainty without the required variable.",
            ["default_answer_bias", "overconfidence"],
            rng,
        )

    if rule_id == "evidence_conflict":
        fact = rng.choice(["paid", "signed", "delivered", "approved"])
        return base_row(
            index,
            rule_id,
            "evidence_words",
            f"source_a says {fact}; source_b says not_{fact}; answer status?",
            f"CONFLICT verify_{fact}",
            f"PROVEN {fact}",
            "CONFLICT",
            "The two sources contradict each other, so verification is required.",
            "The negative chooses one side despite conflicting evidence.",
            ["source_priority_bias", "overconfidence"],
            rng,
        )

    if rule_id == "greater_less_compare":
        a = rng.randrange(1, 20)
        b = rng.randrange(1, 20)
        while b == a:
            b = rng.randrange(1, 20)
        mode = rng.choice(["greater", "less"])
        target = "A" if (a > b if mode == "greater" else a < b) else "B"
        negative = "B" if target == "A" else "A"
        return base_row(
            index,
            rule_id,
            "numbers",
            f"A={a}; B={b}; choose {mode}",
            target,
            negative,
            "PROVEN",
            "The selected label satisfies the requested comparison.",
            "The negative is the opposite comparison result.",
            ["label_bias", "number_frequency"],
            rng,
        )

    if rule_id == "compose_shift_then_replace":
        surface, (a, b, c, x) = surface_tokens(rng, 4)
        return base_row(
            index,
            rule_id,
            surface,
            f"steps: shift_left then replace {b}->{x}; input: {a} {b} {c}; result?",
            f"{x} {c} {a}",
            f"{b} {c} {a}",
            "PROVEN",
            "After left shift the middle item moves first, then it is replaced.",
            "The negative performs the shift but skips replacement.",
            ["single_rule_shortcut", "partial_transform"],
            rng,
        )

    if rule_id == "compose_count_then_compare":
        left_red = rng.randrange(1, 5)
        right_red = rng.randrange(1, 5)
        while right_red == left_red:
            right_red = rng.randrange(1, 5)
        target = "A" if left_red > right_red else "B"
        negative = "B" if target == "A" else "A"
        return base_row(
            index,
            rule_id,
            "count_words",
            f"A has {'red ' * left_red}blue; B has {'red ' * right_red}blue; more red?",
            target,
            negative,
            "PROVEN",
            "The answer counts red items first, then compares counts.",
            "The negative chooses the smaller red count.",
            ["length_bias", "label_bias"],
            rng,
        )

    raise ValueError(f"unknown rule_id: {rule_id}")


def build(count: int) -> list[dict[str, object]]:
    rng = random.Random(RNG_SEED)
    rows = [make_task(index, RULE_IDS[index % len(RULE_IDS)], rng) for index in range(count)]
    return rows


def write_jsonl(path: Path, rows: list[dict[str, object]]) -> None:
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True))
            handle.write("\n")


def write_manifest(path: Path, rows: list[dict[str, object]]) -> None:
    by_rule = Counter(str(row["proof_rule_id"]) for row in rows)
    by_surface = Counter(str(row["surface_family"]) for row in rows)
    by_status = Counter(str(row["answer_status"]) for row in rows)
    manifest = {
        "schema_version": "rule_logic_corpus_manifest_v1",
        "generator": "build_seed_rule_tasks.py",
        "rng_seed": RNG_SEED,
        "rows": len(rows),
        "rule_count": len(by_rule),
        "surface_family_count": len(by_surface),
        "by_rule": dict(sorted(by_rule.items())),
        "by_surface_family": dict(sorted(by_surface.items())),
        "by_answer_status": dict(sorted(by_status.items())),
        "training_authority": "input,target,near_negative only",
        "proof_only_fields": ["proof_rule_id", "proof_rule_family", "why_target_is_correct", "why_negative_is_wrong"],
    }
    path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--count", type=int, default=12000)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    args = parser.parse_args()

    rows = build(args.count)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    write_jsonl(args.out, rows)
    write_manifest(args.manifest, rows)
    print(f"wrote {args.out} rows={len(rows)}")
    print(f"wrote {args.manifest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
