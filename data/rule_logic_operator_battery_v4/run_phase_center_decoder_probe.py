#!/usr/bin/env python3
"""Zero-epoch phase-center decoder probe for v4.

This is the generator version of the phase-center probe:

    train transitions -> relation phase centers -> decode heldout output slots

It does not choose between provided correct/wrong candidates at inference time.
For each output slot it enumerates possible source slots/marker and chooses the
relation wave closest to the learned phase center.

The cleanup mode also learns adjacent relation phase centers. That keeps the
decoder zero-epoch, but gives the beam a generic continuity term:

    out_i relation + out_{i+1} relation
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
from collections import Counter, defaultdict
from functools import lru_cache
from pathlib import Path
from typing import Any, Iterable


ROOT = Path(__file__).resolve().parent
DEFAULT_CORPUS = ROOT / "accepted_operator_tasks_v4.jsonl"
DEFAULT_REPORT = ROOT / "diagnostics" / "phase_center_decoder_probe_report.json"
DEFAULT_MD = ROOT / "diagnostics" / "PHASE_CENTER_DECODER_PROBE.md"

MARKER_RE = re.compile(r"marker:\s*([^;]+)")

ComplexVector = tuple[complex, ...]
Relation = tuple[str, int | str]


def relation_label(relation: Relation) -> str:
    kind, value = relation
    return f"{kind}:{value}"


def load_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line_no, line in enumerate(handle, 1):
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            row["_line_no"] = line_no
            rows.append(row)
    return rows


def split_of(row: dict[str, Any]) -> str:
    group = str(row["source_group"])
    if "_train_" in group:
        return "train"
    if "_heldout_" in group:
        return "heldout"
    return "unknown"


def action_marker(row: dict[str, Any]) -> str | None:
    match = MARKER_RE.search(str(row["rule_action_example"]))
    if match is None:
        return None
    return match.group(1).strip()


def normalized_action(row: dict[str, Any]) -> str:
    action = str(row["rule_action_example"])
    marker = action_marker(row)
    if marker:
        action = action.replace(marker, "<MARKER>")
    return re.sub(r"\s+", " ", action).strip()


def operator_key(row: dict[str, Any], key_mode: str) -> str:
    condition = row.get("condition_flag")
    condition_key = f"condition={condition}" if condition is not None else "condition=<none>"
    parts = [
        f"class={row['operator_class']}",
        f"length={row['sequence_length']}",
        condition_key,
    ]
    if key_mode == "action":
        parts.append(f"action={normalized_action(row)}")
    elif key_mode == "class_length":
        pass
    else:
        raise ValueError(f"unknown key mode: {key_mode}")
    return "|".join(parts)


@lru_cache(maxsize=524_288)
def hash_phase(atom: str, cell: int) -> complex:
    digest = hashlib.blake2b(
        f"{cell}\0{atom}".encode("utf-8"),
        digest_size=8,
        person=b"nwdecoder",
    ).digest()
    raw = int.from_bytes(digest, "little")
    angle = (raw / float(1 << 64)) * math.tau
    return complex(math.cos(angle), math.sin(angle))


def circular_unit(value: complex) -> complex:
    magnitude = abs(value)
    if magnitude == 0:
        return 0j
    return value / magnitude


def vector_from_atoms(atoms: Iterable[str], cells: int) -> ComplexVector:
    sums = [0j] * cells
    count = 0
    for atom in atoms:
        count += 1
        for cell in range(cells):
            sums[cell] += hash_phase(atom, cell)
    if count == 0:
        return tuple(0j for _ in range(cells))
    return tuple(circular_unit(value) for value in sums)


def relation_atoms(
    operator_class: str,
    source_len: int,
    relation: Relation,
    out_slot: int,
    out_len: int,
) -> list[str]:
    atoms = [
        f"class:{operator_class}",
        f"src_len:{source_len}",
        f"out_len:{out_len}",
        f"out:o{out_slot}",
    ]
    kind, value = relation
    if kind == "src":
        src_slot = int(value)
        delta = out_slot - src_slot
        atoms.extend(
            [
                f"src:s{src_slot}",
                f"rel:o{out_slot}:s{src_slot}",
                f"delta:{delta}",
                f"abs_delta:{abs(delta)}",
            ]
        )
    elif kind == "marker":
        atoms.extend(["src:marker", f"rel:o{out_slot}:marker"])
    else:
        raise ValueError(f"unknown relation kind: {kind}")
    return atoms


def pair_relation_atoms(
    operator_class: str,
    source_len: int,
    left: Relation,
    right: Relation,
    out_slot: int,
    out_len: int,
) -> list[str]:
    atoms = [
        f"class:{operator_class}",
        f"src_len:{source_len}",
        f"out_len:{out_len}",
        f"pair:o{out_slot}:o{out_slot + 1}",
        f"left:{relation_label(left)}",
        f"right:{relation_label(right)}",
        f"kind_pair:{left[0]}->{right[0]}",
    ]
    if left[0] == "src" and right[0] == "src":
        left_slot = int(left[1])
        right_slot = int(right[1])
        step = right_slot - left_slot
        atoms.extend(
            [
                f"src_pair:s{left_slot}:s{right_slot}",
                f"src_step:{step}",
                f"abs_src_step:{abs(step)}",
            ]
        )
    elif left[0] == "marker" and right[0] == "src":
        atoms.append(f"marker_then_src:s{int(right[1])}")
    elif left[0] == "src" and right[0] == "marker":
        atoms.append(f"src_then_marker:s{int(left[1])}")
    else:
        atoms.append("marker_then_marker")
    return atoms


@lru_cache(maxsize=262_144)
def relation_vector_cached(
    operator_class: str,
    source_len: int,
    relation_kind: str,
    relation_value: int | str,
    out_slot: int,
    out_len: int,
    cells: int,
) -> ComplexVector:
    return vector_from_atoms(
        relation_atoms(
            operator_class,
            source_len,
            (relation_kind, relation_value),
            out_slot,
            out_len,
        ),
        cells,
    )


def relation_vector(
    row: dict[str, Any],
    relation: Relation,
    out_slot: int,
    out_len: int,
    cells: int,
) -> ComplexVector:
    kind, value = relation
    return relation_vector_cached(
        str(row["operator_class"]),
        len(row["source_tokens"]),
        kind,
        value,
        out_slot,
        out_len,
        cells,
    )


@lru_cache(maxsize=262_144)
def pair_relation_vector_cached(
    operator_class: str,
    source_len: int,
    left_kind: str,
    left_value: int | str,
    right_kind: str,
    right_value: int | str,
    out_slot: int,
    out_len: int,
    cells: int,
) -> ComplexVector:
    return vector_from_atoms(
        pair_relation_atoms(
            operator_class,
            source_len,
            (left_kind, left_value),
            (right_kind, right_value),
            out_slot,
            out_len,
        ),
        cells,
    )


def pair_relation_vector(
    row: dict[str, Any],
    left: Relation,
    right: Relation,
    out_slot: int,
    out_len: int,
    cells: int,
) -> ComplexVector:
    left_kind, left_value = left
    right_kind, right_value = right
    return pair_relation_vector_cached(
        str(row["operator_class"]),
        len(row["source_tokens"]),
        left_kind,
        left_value,
        right_kind,
        right_value,
        out_slot,
        out_len,
        cells,
    )


def add_vec(left: list[complex], right: ComplexVector, sign: float = 1.0) -> None:
    for index, value in enumerate(right):
        left[index] += sign * value


def center_from_sum(values: list[complex]) -> ComplexVector:
    return tuple(circular_unit(value) for value in values)


def coherence(vector: ComplexVector, center: ComplexVector | None) -> float:
    if center is None:
        return 0.0
    return sum((v * c.conjugate()).real for v, c in zip(vector, center)) / float(len(vector))


def token_relation(row: dict[str, Any], token: str) -> Relation | None:
    source_tokens = [str(item) for item in row["source_tokens"]]
    positions = [index for index, source in enumerate(source_tokens) if source == token]
    if len(positions) == 1:
        return ("src", positions[0])
    marker = action_marker(row)
    if marker is not None and token == marker:
        return ("marker", "<MARKER>")
    return None


def candidate_relations(row: dict[str, Any]) -> list[Relation]:
    relations: list[Relation] = [("src", index) for index, _ in enumerate(row["source_tokens"])]
    if action_marker(row) is not None:
        relations.append(("marker", "<MARKER>"))
    return relations


def token_relations(row: dict[str, Any], tokens: list[str]) -> list[Relation] | None:
    relations: list[Relation] = []
    for token in tokens:
        relation = token_relation(row, token)
        if relation is None:
            return None
        relations.append(relation)
    return relations


def relation_to_token(row: dict[str, Any], relation: Relation) -> str | None:
    kind, value = relation
    if kind == "src":
        index = int(value)
        source_tokens = [str(item) for item in row["source_tokens"]]
        if 0 <= index < len(source_tokens):
            return source_tokens[index]
        return None
    if kind == "marker":
        return action_marker(row)
    return None


def milli(numerator: int, denominator: int) -> int:
    if denominator == 0:
        return 0
    return round(1000 * numerator / denominator)


def compile_centers(
    train_rows: list[dict[str, Any]],
    cells: int,
    key_mode: str,
) -> dict[str, Any]:
    positive: dict[tuple[str, int], list[complex]] = defaultdict(lambda: [0j] * cells)
    negative: dict[tuple[str, int], list[complex]] = defaultdict(lambda: [0j] * cells)
    positive_pairs: dict[tuple[str, int], list[complex]] = defaultdict(lambda: [0j] * cells)
    negative_pairs: dict[tuple[str, int], list[complex]] = defaultdict(lambda: [0j] * cells)
    out_lens: dict[str, Counter[int]] = defaultdict(Counter)
    capacity_profiles: dict[str, Counter[tuple[Relation, ...]]] = defaultdict(Counter)
    slot_counts: Counter[tuple[str, int]] = Counter()
    pair_counts: Counter[tuple[str, int]] = Counter()
    skipped = 0

    for row in train_rows:
        key = operator_key(row, key_mode)
        correct_tokens = [str(item) for item in row["correct_tokens"]]
        wrong_tokens = [str(item) for item in row["wrong_tokens"]]
        correct_relations = token_relations(row, correct_tokens)
        wrong_relations = token_relations(row, wrong_tokens)
        out_lens[key][len(correct_tokens)] += 1
        row_capacity: Counter[Relation] = Counter()

        if correct_relations is None:
            skipped += len(correct_tokens)
        else:
            for out_slot, relation in enumerate(correct_relations):
                row_capacity[relation] += 1
                vec = relation_vector(row, relation, out_slot, len(correct_tokens), cells)
                add_vec(positive[(key, out_slot)], vec)
                slot_counts[(key, out_slot)] += 1

            for out_slot, (left, right) in enumerate(zip(correct_relations, correct_relations[1:])):
                vec = pair_relation_vector(row, left, right, out_slot, len(correct_tokens), cells)
                add_vec(positive_pairs[(key, out_slot)], vec)
                pair_counts[(key, out_slot)] += 1

        if wrong_relations is None:
            skipped += len(wrong_tokens)
        else:
            for out_slot, relation in enumerate(wrong_relations):
                vec = relation_vector(row, relation, out_slot, len(correct_tokens), cells)
                add_vec(negative[(key, out_slot)], vec)

            for out_slot, (left, right) in enumerate(zip(wrong_relations, wrong_relations[1:])):
                vec = pair_relation_vector(row, left, right, out_slot, len(correct_tokens), cells)
                add_vec(negative_pairs[(key, out_slot)], vec)

        if row_capacity:
            profile: tuple[Relation, ...] = tuple(
                relation
                for relation, count in sorted(row_capacity.items(), key=lambda item: repr(item[0]))
                for _ in range(count)
            )
            capacity_profiles[key][profile] += 1

    return {
        "positive_centers": {key: center_from_sum(value) for key, value in positive.items()},
        "negative_centers": {key: center_from_sum(value) for key, value in negative.items()},
        "positive_pair_centers": {
            key: center_from_sum(value) for key, value in positive_pairs.items()
        },
        "negative_pair_centers": {
            key: center_from_sum(value) for key, value in negative_pairs.items()
        },
        "out_len_by_key": {
            key: counter.most_common(1)[0][0] for key, counter in out_lens.items()
        },
        "capacity_by_key": {
            key: Counter(profile)
            for key, counter in capacity_profiles.items()
            for profile, _ in [counter.most_common(1)[0]]
        },
        "compiled_operator_keys": len(out_lens),
        "compiled_slot_centers": len(positive),
        "compiled_pair_centers": len(positive_pairs),
        "skipped_train_relations": skipped,
        "slot_counts": slot_counts,
        "pair_counts": pair_counts,
    }


def relation_scores_for_slot(
    row: dict[str, Any],
    compiled: dict[str, Any],
    key: str,
    out_slot: int,
    out_len: int,
    cells: int,
    candidates: list[Relation],
) -> list[tuple[float, Relation]]:
    pos_center = compiled["positive_centers"].get((key, out_slot))
    neg_center = compiled["negative_centers"].get((key, out_slot))
    if pos_center is None:
        return []
    ranked: list[tuple[float, Relation]] = []
    for relation in candidates:
        vec = relation_vector(row, relation, out_slot, out_len, cells)
        score = coherence(vec, pos_center) - coherence(vec, neg_center)
        ranked.append((score, relation))
    ranked.sort(key=lambda item: item[0], reverse=True)
    return ranked


def pair_relation_score(
    row: dict[str, Any],
    compiled: dict[str, Any],
    key: str,
    out_slot: int,
    out_len: int,
    cells: int,
    left: Relation,
    right: Relation,
) -> float:
    pos_center = compiled["positive_pair_centers"].get((key, out_slot))
    if pos_center is None:
        return 0.0
    neg_center = compiled["negative_pair_centers"].get((key, out_slot))
    vec = pair_relation_vector(row, left, right, out_slot, out_len, cells)
    return coherence(vec, pos_center) - coherence(vec, neg_center)


def decode_row_local(
    row: dict[str, Any], compiled: dict[str, Any], cells: int, key_mode: str
) -> dict[str, Any]:
    key = operator_key(row, key_mode)
    out_len = compiled["out_len_by_key"].get(key)
    if out_len is None:
        return {"status": "missing_operator_key", "tokens": []}

    tokens: list[str] = []
    relations: list[Relation] = []
    margins: list[float] = []
    candidates = candidate_relations(row)
    for out_slot in range(out_len):
        ranked = relation_scores_for_slot(row, compiled, key, out_slot, out_len, cells, candidates)
        if not ranked:
            return {"status": "missing_slot_center", "tokens": tokens}
        best_score, best_relation = ranked[0]
        second_score = ranked[1][0] if len(ranked) > 1 else -1.0e9
        token = relation_to_token(row, best_relation)
        if token is None:
            return {"status": "bad_relation_token", "tokens": tokens}
        tokens.append(token)
        relations.append(best_relation)
        margins.append(best_score - second_score)

    duplicate_source_uses = len(
        [
            value
            for kind, value in relations
            if kind == "src" and Counter(relations)[(kind, value)] > 1
        ]
    )
    return {
        "status": "decoded",
        "tokens": tokens,
        "relations": relations,
        "min_slot_margin": min(margins) if margins else None,
        "median_slot_margin": sorted(margins)[len(margins) // 2] if margins else None,
        "duplicate_source_uses": duplicate_source_uses,
    }


def decode_row_capacity_cleanup(
    row: dict[str, Any],
    compiled: dict[str, Any],
    cells: int,
    key_mode: str,
    beam_width: int,
    slot_top_k: int,
    relation_top_slots: int,
    pairwise_weight: float,
) -> dict[str, Any]:
    key = operator_key(row, key_mode)
    out_len = compiled["out_len_by_key"].get(key)
    capacity: Counter[Relation] | None = compiled["capacity_by_key"].get(key)
    if out_len is None:
        return {"status": "missing_operator_key", "tokens": []}
    if capacity is None:
        return {"status": "missing_capacity_profile", "tokens": []}

    row_candidates = set(candidate_relations(row))
    capacity = Counter({relation: count for relation, count in capacity.items() if relation in row_candidates})
    if sum(capacity.values()) != out_len:
        return {"status": "capacity_mismatch", "tokens": []}

    all_slot_rankings: list[list[tuple[float, Relation]]] = []
    for out_slot in range(out_len):
        ranked = relation_scores_for_slot(
            row,
            compiled,
            key,
            out_slot,
            out_len,
            cells,
            list(capacity.keys()),
        )
        if not ranked:
            return {"status": "missing_slot_center", "tokens": []}
        all_slot_rankings.append(ranked)

    slot_rankings = [ranked[:slot_top_k] for ranked in all_slot_rankings]
    best_slots_for_relation: dict[Relation, list[tuple[float, int]]] = defaultdict(list)
    for out_slot, ranked in enumerate(all_slot_rankings):
        for score, relation in ranked:
            best_slots_for_relation[relation].append((score, out_slot))

    for relation in capacity:
        best_slots = best_slots_for_relation.get(relation)
        if not best_slots:
            continue
        best_slots.sort(key=lambda item: item[0], reverse=True)
        for score, out_slot in best_slots[:relation_top_slots]:
            if any(item_relation == relation for _, item_relation in slot_rankings[out_slot]):
                continue
            slot_rankings[out_slot].append((score, relation))
            slot_rankings[out_slot].sort(key=lambda item: item[0], reverse=True)

    pair_scores_by_slot: list[dict[tuple[Relation, Relation], float]] = [
        {} for _ in range(out_len)
    ]
    if pairwise_weight != 0.0:
        capacity_relations = list(capacity.keys())
        for out_slot in range(1, out_len):
            scores = pair_scores_by_slot[out_slot]
            for left in capacity_relations:
                for right in capacity_relations:
                    scores[(left, right)] = pair_relation_score(
                        row,
                        compiled,
                        key,
                        out_slot - 1,
                        out_len,
                        cells,
                        left,
                        right,
                    )

    beams: list[tuple[float, list[Relation], Counter[Relation]]] = [(0.0, [], capacity)]
    for out_slot, ranked in enumerate(slot_rankings):
        next_beams: list[tuple[float, list[Relation], Counter[Relation]]] = []
        for score_so_far, relations_so_far, remaining in beams:
            for score, relation in ranked:
                if remaining[relation] <= 0:
                    continue
                pair_score = 0.0
                if pairwise_weight != 0.0 and relations_so_far:
                    pair_score = pair_scores_by_slot[out_slot].get(
                        (relations_so_far[-1], relation),
                        0.0,
                    )
                updated = remaining.copy()
                updated[relation] -= 1
                if updated[relation] == 0:
                    del updated[relation]
                total = score_so_far + score + pairwise_weight * pair_score
                next_beams.append((total, relations_so_far + [relation], updated))
        if not next_beams:
            return {"status": "beam_empty", "tokens": []}
        next_beams.sort(key=lambda item: item[0], reverse=True)
        beams = next_beams[:beam_width]

    best_score, best_relations, remaining = beams[0]
    if remaining:
        return {"status": "beam_incomplete", "tokens": []}
    second_score = beams[1][0] if len(beams) > 1 else -1.0e9

    tokens: list[str] = []
    for relation in best_relations:
        token = relation_to_token(row, relation)
        if token is None:
            return {"status": "bad_relation_token", "tokens": tokens}
        tokens.append(token)

    return {
        "status": "decoded",
        "tokens": tokens,
        "relations": best_relations,
        "min_slot_margin": best_score - second_score,
        "median_slot_margin": best_score - second_score,
        "duplicate_source_uses": 0,
    }


def eval_mode(
    rows: list[dict[str, Any]],
    cells: int,
    key_mode: str,
    decoder_mode: str,
    beam_width: int,
    slot_top_k: int,
    relation_top_slots: int,
    pairwise_weight: float,
) -> dict[str, Any]:
    train = [row for row in rows if split_of(row) == "train"]
    heldout = [row for row in rows if split_of(row) == "heldout"]
    compiled = compile_centers(train, cells, key_mode)

    correct = 0
    wrong_exact = 0
    same_bag = 0
    decoded = 0
    missing = 0
    duplicate_rows = 0
    min_margins: list[float] = []
    by_class: dict[str, Counter[str]] = defaultdict(Counter)
    by_surface: dict[str, Counter[str]] = defaultdict(Counter)
    by_noise: dict[str, Counter[str]] = defaultdict(Counter)
    failures: list[dict[str, Any]] = []

    for row in heldout:
        if decoder_mode == "local":
            result = decode_row_local(row, compiled, cells, key_mode)
        elif decoder_mode == "capacity_cleanup":
            result = decode_row_capacity_cleanup(
                row,
                compiled,
                cells,
                key_mode,
                beam_width,
                slot_top_k,
                relation_top_slots,
                pairwise_weight,
            )
        else:
            raise ValueError(f"unknown decoder mode: {decoder_mode}")
        outcome = result["status"]
        tokens = list(result["tokens"])
        cls = str(row["operator_class"])
        surface = str(row["surface_family"])
        noise = str(row["noise_type"])
        by_class[cls]["rows"] += 1
        by_surface[surface]["rows"] += 1
        by_noise[noise]["rows"] += 1

        if outcome == "decoded":
            decoded += 1
            if result.get("min_slot_margin") is not None:
                min_margins.append(float(result["min_slot_margin"]))
            if result.get("duplicate_source_uses", 0) > 0:
                duplicate_rows += 1
            if tokens == list(row["correct_tokens"]):
                correct += 1
                outcome = "correct"
            elif tokens == list(row["wrong_tokens"]):
                wrong_exact += 1
                outcome = "wrong_exact"
            else:
                outcome = "other_wrong"
            if Counter(tokens) == Counter(row["correct_tokens"]):
                same_bag += 1
        else:
            missing += 1

        by_class[cls][outcome] += 1
        by_surface[surface][outcome] += 1
        by_noise[noise][outcome] += 1
        if outcome != "correct" and len(failures) < 20:
            failures.append(
                {
                    "task_id": row["task_id"],
                    "operator_class": row["operator_class"],
                    "proof_rule_family": row["proof_rule_family"],
                    "sequence_length": row["sequence_length"],
                    "surface_family": row["surface_family"],
                    "noise_type": row["noise_type"],
                    "outcome": outcome,
                    "generated": tokens,
                    "correct": row["correct_tokens"],
                    "wrong": row["wrong_tokens"],
                }
            )

    sorted_margins = sorted(min_margins)
    return {
        "key_mode": key_mode,
        "decoder_mode": decoder_mode,
        "cells": cells,
        "beam_width": beam_width if decoder_mode == "capacity_cleanup" else None,
        "slot_top_k": slot_top_k if decoder_mode == "capacity_cleanup" else None,
        "relation_top_slots": relation_top_slots if decoder_mode == "capacity_cleanup" else None,
        "train_rows": len(train),
        "heldout_rows": len(heldout),
        "compiled_operator_keys": compiled["compiled_operator_keys"],
        "compiled_slot_centers": compiled["compiled_slot_centers"],
        "compiled_pair_centers": compiled["compiled_pair_centers"],
        "skipped_train_relations": compiled["skipped_train_relations"],
        "decoded_rows": decoded,
        "missing_rows": missing,
        "heldout_correct_rows": correct,
        "heldout_accuracy_milli": milli(correct, len(heldout)),
        "wrong_exact_rows": wrong_exact,
        "same_bag_output_milli": milli(same_bag, decoded),
        "duplicate_source_rows": duplicate_rows,
        "median_min_slot_margin": sorted_margins[len(sorted_margins) // 2]
        if sorted_margins
        else None,
        "p10_min_slot_margin": sorted_margins[len(sorted_margins) // 10]
        if sorted_margins
        else None,
        "by_class": {key: dict(value) for key, value in sorted(by_class.items())},
        "by_surface": {key: dict(value) for key, value in sorted(by_surface.items())},
        "by_noise": {key: dict(value) for key, value in sorted(by_noise.items())},
        "failure_examples": failures,
    }


def make_report(
    rows: list[dict[str, Any]],
    cells: int,
    beam_width: int,
    slot_top_k: int,
    relation_top_slots: int,
    pairwise_weight: float,
) -> dict[str, Any]:
    action_local = eval_mode(
        rows,
        cells,
        "action",
        "local",
        beam_width=1,
        slot_top_k=1,
        relation_top_slots=1,
        pairwise_weight=0.0,
    )
    action_cleanup = eval_mode(
        rows,
        cells,
        "action",
        "capacity_cleanup",
        beam_width=beam_width,
        slot_top_k=slot_top_k,
        relation_top_slots=relation_top_slots,
        pairwise_weight=pairwise_weight,
    )
    no_action = eval_mode(
        rows,
        cells,
        "class_length",
        "capacity_cleanup",
        beam_width=beam_width,
        slot_top_k=slot_top_k,
        relation_top_slots=relation_top_slots,
        pairwise_weight=pairwise_weight,
    )
    verdict = "PHASE_CENTER_DECODER_PROBE_WATCH"
    if (
        action_cleanup["heldout_accuracy_milli"] == 1000
        and action_cleanup["wrong_exact_rows"] == 0
        and action_cleanup["missing_rows"] == 0
        and no_action["heldout_accuracy_milli"] < action_cleanup["heldout_accuracy_milli"]
    ):
        verdict = "PHASE_CENTER_DECODER_PROBE_PASS"

    return {
        "schema_version": "phase_center_decoder_probe_v2",
        "diagnostic_only": True,
        "verdict": verdict,
        "method": "decode_output_slots_from_relation_phase_centers_with_pairwise_cleanup",
        "pairwise_weight": pairwise_weight,
        "not_used_at_heldout_decode_time": [
            "correct_tokens",
            "wrong_tokens",
            "state_after_correct",
            "state_after_wrong",
            "proof_rule_id as key",
            "source_group as key",
            "explicit out<-src program table",
            "epoch repair",
        ],
        "claim_boundary": [
            "This is a zero-epoch phase-center decoder diagnostic.",
            "It is not yet the production Wave flat runtime.",
            "It does not prove semantic grokking.",
            "The next step is to compile this decoder into Rust/Wave readout and run parity/ablations.",
        ],
        "action_key_local_decoder": action_local,
        "action_key_capacity_cleanup_decoder": action_cleanup,
        "no_action_key_capacity_cleanup_ablation": no_action,
    }


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    local = report["action_key_local_decoder"]
    action = report["action_key_capacity_cleanup_decoder"]
    no_action = report["no_action_key_capacity_cleanup_ablation"]
    text = f"""# Phase Center Decoder Probe

Date: 2026-07-02

## Question

Can phase centers generate heldout output slots, not merely judge correct vs
wrong candidates?

## Method

Script:

```text
data/rule_logic_operator_battery_v4/run_phase_center_decoder_probe.py
```

Training creates relation phase centers for each operator/output slot:

```text
out_i receives src_j
out_i receives marker
```

Heldout decoding receives only:

```text
state_before source tokens
rule_action_example
condition flag
```

It enumerates possible source-slot/marker relations and selects the relation
closest to the phase center. The stronger mode also uses a learned capacity
cleanup profile from train, so the decoded sequence must use the right number
of source/marker factors. In v2 it also uses adjacent relation phase centers:

```text
out_i relation + out_i+1 relation
```

This is a generic continuity cleanup term, not an explicit edit program table.
It does not choose between provided correct/wrong candidates.

## Result

Verdict:

```text
{report["verdict"]}
```

Action-key decoder:

```text
compiled_operator_keys: {action["compiled_operator_keys"]}
compiled_slot_centers: {action["compiled_slot_centers"]}
compiled_pair_centers: {action["compiled_pair_centers"]}
pairwise_weight: {report["pairwise_weight"]}
heldout_rows: {action["heldout_rows"]}
decoded_rows: {action["decoded_rows"]}
heldout_accuracy_milli: {action["heldout_accuracy_milli"]}
wrong_exact_rows: {action["wrong_exact_rows"]}
same_bag_output_milli: {action["same_bag_output_milli"]}
median_min_slot_margin: {action["median_min_slot_margin"]}
p10_min_slot_margin: {action["p10_min_slot_margin"]}
```

Local-only decoder boundary:

```text
heldout_accuracy_milli: {local["heldout_accuracy_milli"]}
wrong_exact_rows: {local["wrong_exact_rows"]}
duplicate_source_rows: {local["duplicate_source_rows"]}
```

No-action key ablation:

```text
compiled_operator_keys: {no_action["compiled_operator_keys"]}
compiled_slot_centers: {no_action["compiled_slot_centers"]}
heldout_accuracy_milli: {no_action["heldout_accuracy_milli"]}
wrong_exact_rows: {no_action["wrong_exact_rows"]}
```

By class:

```text
{json.dumps(action["by_class"], ensure_ascii=False, indent=2)}
```

## Interpretation

This is stronger than the previous phase-center judge:

```text
phase centers -> output slots -> learned capacity cleanup -> full sequence
phase centers -> adjacent relation cleanup -> full sequence
```

It tests whether the center can act as a decoder/generator for the transition,
not only as a scorer for an already supplied candidate.

## Claim Boundary

Allowed:

```text
The current v4 battery has a zero-epoch phase-center decoder signal.
```

Not allowed:

```text
semantic grokking proven
production Wave runtime solved
enterprise benchmark complete
```

Next:

```text
compile the phase-center decoder into Rust/Wave flat runtime and add ablations:
without action center, without role center, shuffled centers, and reduced cells.
```
"""
    path.write_text(text, encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    parser.add_argument("--cells", type=int, default=32)
    parser.add_argument("--beam-width", type=int, default=32)
    parser.add_argument("--slot-top-k", type=int, default=8)
    parser.add_argument("--relation-top-slots", type=int, default=4)
    parser.add_argument("--pairwise-weight", type=float, default=1.0)
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    parser.add_argument("--markdown", type=Path, default=DEFAULT_MD)
    args = parser.parse_args()
    if args.cells < 2:
        raise SystemExit("--cells must be >= 2")
    if args.beam_width < 1:
        raise SystemExit("--beam-width must be >= 1")
    if args.slot_top_k < 1:
        raise SystemExit("--slot-top-k must be >= 1")
    if args.relation_top_slots < 1:
        raise SystemExit("--relation-top-slots must be >= 1")

    rows = load_rows(args.corpus)
    report = make_report(
        rows,
        args.cells,
        args.beam_width,
        args.slot_top_k,
        args.relation_top_slots,
        args.pairwise_weight,
    )
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    write_markdown(args.markdown, report)

    local = report["action_key_local_decoder"]
    action = report["action_key_capacity_cleanup_decoder"]
    no_action = report["no_action_key_capacity_cleanup_ablation"]
    print("phase_center_decoder_probe:")
    print(f"  verdict: {report['verdict']}")
    print(f"  corpus: {args.corpus}")
    print(f"  report: {args.report}")
    print(f"  cells: {args.cells}")
    print(f"  local_heldout_accuracy_milli: {local['heldout_accuracy_milli']}")
    print(f"  local_duplicate_source_rows: {local['duplicate_source_rows']}")
    print(f"  cleanup_compiled_operator_keys: {action['compiled_operator_keys']}")
    print(f"  cleanup_compiled_slot_centers: {action['compiled_slot_centers']}")
    print(f"  cleanup_compiled_pair_centers: {action['compiled_pair_centers']}")
    print(f"  pairwise_weight: {report['pairwise_weight']}")
    print(f"  cleanup_heldout_accuracy_milli: {action['heldout_accuracy_milli']}")
    print(f"  cleanup_wrong_exact_rows: {action['wrong_exact_rows']}")
    print(f"  cleanup_median_min_slot_margin: {action['median_min_slot_margin']}")
    print(f"  cleanup_p10_min_slot_margin: {action['p10_min_slot_margin']}")
    print(f"  no_action_heldout_accuracy_milli: {no_action['heldout_accuracy_milli']}")
    print(f"  no_action_wrong_exact_rows: {no_action['wrong_exact_rows']}")


if __name__ == "__main__":
    main()
