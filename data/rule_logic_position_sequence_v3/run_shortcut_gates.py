#!/usr/bin/env python3
"""Shortcut gates for v3 ordered position-binding tasks."""

from __future__ import annotations

import json
import math
import os
import re
from collections import Counter, defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parent
TASKS = Path(os.environ.get("POSITION_SEQUENCE_TASKS", ROOT / "accepted_position_sequence_tasks_v3.jsonl")).resolve()
REPORT = Path(
    os.environ.get("POSITION_SEQUENCE_SHORTCUT_REPORT", TASKS.parent / "shortcut_gate_report.json")
).resolve()
TOKEN_RE = re.compile(r"[A-Za-zА-Яа-я0-9_]+|[{}:;?=<>|,-]")


def tokens(text: str) -> list[str]:
    return [token.lower() for token in TOKEN_RE.findall(text)]


def state_tokens(text: str) -> list[str]:
    items = tokens(text)
    if items and items[0] == "state":
        items = items[1:]
    if items and items[0] == ":":
        items = items[1:]
    return items


def load_rows() -> list[dict[str, object]]:
    return [json.loads(line) for line in TASKS.read_text(encoding="utf-8").splitlines() if line.strip()]


def split_rows(rows: list[dict[str, object]]) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
    train = [row for row in rows if str(row["source_group"]).startswith("position_sequence_train_")]
    heldout = [row for row in rows if str(row["source_group"]).startswith("position_sequence_heldout_")]
    return train, heldout


def milli(num: float, den: int) -> int:
    return 0 if den == 0 else round(1000 * num / den)


def exact_lookup(train: list[dict[str, object]], heldout: list[dict[str, object]]) -> int:
    table = {
        str(row["state_before"]) + "\n" + str(row["rule_action_example"]): str(row["state_after_correct"])
        for row in train
    }
    hits = 0
    for row in heldout:
        key = str(row["state_before"]) + "\n" + str(row["rule_action_example"])
        hits += int(table.get(key) == str(row["state_after_correct"]))
    return milli(hits, len(heldout))


def majority_by_key(train: list[dict[str, object]], heldout: list[dict[str, object]], key: str) -> int:
    groups: dict[str, Counter[str]] = defaultdict(Counter)
    for row in train:
        groups[str(row[key])][str(row["state_after_correct"])] += 1
    hits = 0
    seen = 0
    for row in heldout:
        group = groups.get(str(row[key]))
        if not group:
            continue
        seen += 1
        hits += int(group.most_common(1)[0][0] == str(row["state_after_correct"]))
    return milli(hits, seen)


def bag_pairwise(heldout: list[dict[str, object]]) -> int:
    score = 0.0
    for row in heldout:
        good = Counter(state_tokens(str(row["state_after_correct"])))
        bad = Counter(state_tokens(str(row["state_after_wrong"])))
        if good == bad:
            score += 0.5
        elif good:
            score += 1.0
    return milli(score, len(heldout))


def same_bag_derangement_milli(rows: list[dict[str, object]]) -> int:
    hits = 0
    for row in rows:
        good = list(row["correct_tokens"])
        bad = list(row["wrong_tokens"])
        hits += int(Counter(good) == Counter(bad) and all(left != right for left, right in zip(good, bad)))
    return milli(hits, len(rows))


def markov_bigram_pairwise(train: list[dict[str, object]], heldout: list[dict[str, object]]) -> int:
    counts: dict[tuple[str, str], Counter[str]] = defaultdict(Counter)
    unigram = Counter()
    vocab = set()
    for row in train:
        context = set(tokens(str(row["state_before"]) + " " + str(row["rule_action_example"])))
        target = state_tokens(str(row["state_after_correct"]))
        for token in target:
            vocab.add(token)
            unigram[token] += 1
        for left, right in zip(target, target[1:]):
            for context_token in context:
                counts[(context_token, left)][right] += 1
    vocab_size = max(1, len(vocab))

    def score(row: dict[str, object], candidate_field: str) -> float:
        context = set(tokens(str(row["state_before"]) + " " + str(row["rule_action_example"])))
        candidate = state_tokens(str(row[candidate_field]))
        total = 0.0
        for token in candidate:
            total += math.log((unigram[token] + 1) / (sum(unigram.values()) + vocab_size))
        for left, right in zip(candidate, candidate[1:]):
            link = 0.0
            seen = 0
            for context_token in context:
                counter = counts.get((context_token, left))
                if not counter:
                    continue
                seen += 1
                link += (counter[right] + 1) / (sum(counter.values()) + vocab_size)
            if seen:
                total += math.log(link / seen)
        return total

    hits = 0
    ties = 0
    for row in heldout:
        good = score(row, "state_after_correct")
        bad = score(row, "state_after_wrong")
        if good > bad:
            hits += 1
        elif good == bad:
            ties += 1
    return milli(hits + ties * 0.5, len(heldout))


def output_position_prior(train: list[dict[str, object]], heldout: list[dict[str, object]]) -> int:
    priors: dict[tuple[int, int], Counter[int]] = defaultdict(Counter)
    for row in train:
        source = list(row["source_tokens"])
        target = list(row["correct_tokens"])
        for out_index, token in enumerate(target):
            if token in source:
                priors[(len(source), out_index)][source.index(token)] += 1

    hits = 0
    seen = 0
    for row in heldout:
        source = list(row["source_tokens"])
        predicted = []
        for out_index in range(len(source)):
            prior = priors.get((len(source), out_index))
            if not prior:
                continue
            input_index = prior.most_common(1)[0][0]
            predicted.append(source[input_index])
        if len(predicted) != len(source):
            continue
        seen += 1
        hits += int(predicted == list(row["correct_tokens"]))
    return milli(hits, seen)


def template_without_sequence(train: list[dict[str, object]], heldout: list[dict[str, object]]) -> int:
    return majority_by_key(train, heldout, "proof_rule_id")


def l2_neighbor_target_copy(train: list[dict[str, object]], heldout: list[dict[str, object]]) -> int:
    train_vectors = [
        (set(tokens(str(row["state_before"]) + " " + str(row["rule_action_example"]))), row)
        for row in train
    ]
    hits = 0
    for row in heldout:
        query = set(tokens(str(row["state_before"]) + " " + str(row["rule_action_example"])))
        best = max(
            train_vectors,
            key=lambda item: len(query & item[0]) / max(1, len(query | item[0])),
        )[1]
        hits += int(str(best["state_after_correct"]) == str(row["state_after_correct"]))
    return milli(hits, len(heldout))


def matrix_cell_count(rows: list[dict[str, object]]) -> int:
    return len(
        {
            (
                int(row["sequence_length"]),
                str(row["proof_rule_family"]),
                str(row["surface_family"]),
                str(row["noise_type"]),
            )
            for row in rows
        }
    )


def verdict(metrics: dict[str, int]) -> str:
    if metrics["exact_lookup_accuracy_milli"] > 0:
        return "REJECT_EXACT_LOOKUP_SHORTCUT"
    if metrics["bag_of_tokens_accuracy_milli"] != 500:
        return "REJECT_BAG_GATE_NOT_HARD_PAIRWISE"
    if metrics["same_bag_derangement_milli"] != 1000:
        return "REJECT_NOT_ALL_NEGATIVES_ARE_DERANGEMENTS"
    for key in [
        "proof_rule_id_majority_accuracy_milli",
        "surface_family_majority_accuracy_milli",
        "length_only_accuracy_milli",
        "output_position_prior_accuracy_milli",
        "template_without_sequence_accuracy_milli",
    ]:
        if metrics[key] > 350:
            return f"REJECT_{key.upper()}"
    if metrics["markov_bigram_pairwise_accuracy_milli"] > 650:
        return "REJECT_MARKOV_BIGRAM_SHORTCUT"
    if metrics["l2_neighbor_target_copy_accuracy_milli"] > 650:
        return "REJECT_L2_NEIGHBOR_TARGET_COPY_SHORTCUT"
    return "VALID_POSITION_SEQUENCE_V3_CANDIDATE"


def main() -> int:
    rows = load_rows()
    train, heldout = split_rows(rows)
    metrics = {
        "rows": len(rows),
        "train_rows": len(train),
        "heldout_rows": len(heldout),
        "matrix_cells": matrix_cell_count(rows),
        "exact_lookup_accuracy_milli": exact_lookup(train, heldout),
        "proof_rule_id_majority_accuracy_milli": majority_by_key(train, heldout, "proof_rule_id"),
        "surface_family_majority_accuracy_milli": majority_by_key(train, heldout, "surface_family"),
        "length_only_accuracy_milli": majority_by_key(train, heldout, "sequence_length"),
        "output_position_prior_accuracy_milli": output_position_prior(train, heldout),
        "template_without_sequence_accuracy_milli": template_without_sequence(train, heldout),
        "bag_of_tokens_accuracy_milli": bag_pairwise(heldout),
        "same_bag_derangement_milli": same_bag_derangement_milli(rows),
        "markov_bigram_pairwise_accuracy_milli": markov_bigram_pairwise(train, heldout),
        "l2_neighbor_target_copy_accuracy_milli": l2_neighbor_target_copy(train, heldout),
    }
    report = {
        "schema_version": "position_sequence_shortcut_gate_v3",
        "metrics": metrics,
        "verdict": verdict(metrics),
    }
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True))
    return 0 if report["verdict"] == "VALID_POSITION_SEQUENCE_V3_CANDIDATE" else 1


if __name__ == "__main__":
    raise SystemExit(main())
