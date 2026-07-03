#!/usr/bin/env python3
"""Shortcut gates for v4 operator battery corpora."""

from __future__ import annotations

import json
import math
import os
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parent
TASKS = Path(os.environ.get("OPERATOR_BATTERY_TASKS", ROOT / "accepted_operator_tasks_v4.jsonl")).resolve()
REPORT = Path(os.environ.get("OPERATOR_BATTERY_SHORTCUT_REPORT", TASKS.parent / "shortcut_gate_report.json")).resolve()
TOKEN_RE = re.compile(r"[A-Za-zА-Яа-я0-9_]+|[{}:;?=<>|,-]")
PERMUTATION_CLASSES = {"order", "conditional", "composed"}


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
    train = [row for row in rows if "_train_" in str(row["source_group"])]
    heldout = [row for row in rows if "_heldout_" in str(row["source_group"])]
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
        groups[str(row.get(key, ""))][str(row["state_after_correct"])] += 1
    hits = 0
    seen = 0
    for row in heldout:
        group = groups.get(str(row.get(key, "")))
        if not group:
            continue
        seen += 1
        hits += int(group.most_common(1)[0][0] == str(row["state_after_correct"]))
    return milli(hits, seen)


def same_bag_derangement_milli(rows: list[dict[str, object]]) -> int:
    hits = 0
    checked = 0
    for row in rows:
        if str(row["operator_class"]) not in PERMUTATION_CLASSES:
            continue
        checked += 1
        good = list(row["correct_tokens"])
        bad = list(row["wrong_tokens"])
        hits += int(Counter(good) == Counter(bad) and all(left != right for left, right in zip(good, bad)))
    return milli(hits, checked)


def bag_pairwise(heldout: list[dict[str, object]]) -> int:
    score = 0.0
    checked = 0
    for row in heldout:
        if str(row["operator_class"]) not in PERMUTATION_CLASSES:
            continue
        checked += 1
        good = Counter(state_tokens(str(row["state_after_correct"])))
        bad = Counter(state_tokens(str(row["state_after_wrong"])))
        if good == bad:
            score += 0.5
        elif good:
            score += 1.0
    return milli(score, checked)


def edit_candidate_overlap_milli(rows: list[dict[str, object]]) -> int:
    total = 0
    checked = 0
    for row in rows:
        if str(row["operator_class"]) != "edit":
            continue
        checked += 1
        good = Counter(row["correct_tokens"])
        bad = Counter(row["wrong_tokens"])
        intersection = sum((good & bad).values())
        union = sum((good | bad).values())
        total += milli(intersection, union)
    return 0 if checked == 0 else round(total / checked)


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
    total_unigrams = max(1, sum(unigram.values()))

    def score(row: dict[str, object], candidate_field: str) -> float:
        context = set(tokens(str(row["state_before"]) + " " + str(row["rule_action_example"])))
        candidate = state_tokens(str(row[candidate_field]))
        total = 0.0
        for token in candidate:
            total += math.log((unigram[token] + 1) / (total_unigrams + vocab_size))
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

    return pairwise_metric(heldout, score)


def bayesian_cooccurrence_pairwise(train: list[dict[str, object]], heldout: list[dict[str, object]]) -> int:
    counts: dict[str, Counter[str]] = defaultdict(Counter)
    target_counts = Counter()
    vocab = set()
    for row in train:
        context = set(tokens(str(row["state_before"]) + " " + str(row["rule_action_example"])))
        target = state_tokens(str(row["state_after_correct"]))
        for token in target:
            vocab.add(token)
            target_counts[token] += 1
            for context_token in context:
                counts[context_token][token] += 1
    vocab_size = max(1, len(vocab))
    total_targets = max(1, sum(target_counts.values()))

    def score(row: dict[str, object], candidate_field: str) -> float:
        context = set(tokens(str(row["state_before"]) + " " + str(row["rule_action_example"])))
        candidate = state_tokens(str(row[candidate_field]))
        total = 0.0
        for token in candidate:
            token_score = (target_counts[token] + 1) / (total_targets + vocab_size)
            for context_token in context:
                counter = counts.get(context_token)
                if counter:
                    token_score += (counter[token] + 1) / (sum(counter.values()) + vocab_size)
            total += math.log(token_score / (len(context) + 1))
        return total

    return pairwise_metric(heldout, score)


def pairwise_metric(heldout: list[dict[str, object]], score_fn) -> int:
    hits = 0
    ties = 0
    for row in heldout:
        good = score_fn(row, "state_after_correct")
        bad = score_fn(row, "state_after_wrong")
        if good > bad:
            hits += 1
        elif good == bad:
            ties += 1
    return milli(hits + ties * 0.5, len(heldout))


def output_position_prior(train: list[dict[str, object]], heldout: list[dict[str, object]]) -> int:
    priors: dict[tuple[str, int, int], Counter[int]] = defaultdict(Counter)
    for row in train:
        source = list(row["source_tokens"])
        target = list(row["correct_tokens"])
        operator_class = str(row["operator_class"])
        for out_index, token in enumerate(target):
            if token in source:
                priors[(operator_class, len(source), out_index)][source.index(token)] += 1

    hits = 0
    seen = 0
    for row in heldout:
        source = list(row["source_tokens"])
        predicted = []
        operator_class = str(row["operator_class"])
        for out_index in range(len(row["correct_tokens"])):
            prior = priors.get((operator_class, len(source), out_index))
            if not prior:
                continue
            input_index = prior.most_common(1)[0][0]
            if input_index >= len(source):
                continue
            predicted.append(source[input_index])
        if len(predicted) != len(row["correct_tokens"]):
            continue
        seen += 1
        hits += int(predicted == list(row["correct_tokens"]))
    return milli(hits, seen)


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


def operator_slots_non_order_count(rows: list[dict[str, object]]) -> int:
    return sum(
        1
        for row in rows
        if str(row["operator_class"]) != "order" and "operator_slots:" in str(row["rule_action_example"])
    )


def class_metrics(rows: list[dict[str, object]]) -> dict[str, dict[str, int]]:
    out = {}
    for operator_class in sorted({str(row["operator_class"]) for row in rows}):
        print(f"operator_battery_shortcut: class_start {operator_class}", file=sys.stderr, flush=True)
        subset = [row for row in rows if str(row["operator_class"]) == operator_class]
        train, heldout = split_rows(subset)
        out[operator_class] = metrics_for(subset, train, heldout)
        print(f"operator_battery_shortcut: class_done {operator_class}", file=sys.stderr, flush=True)
    return out


def metrics_for(rows: list[dict[str, object]], train: list[dict[str, object]], heldout: list[dict[str, object]]) -> dict[str, int]:
    return {
        "rows": len(rows),
        "train_rows": len(train),
        "heldout_rows": len(heldout),
        "exact_lookup_accuracy_milli": exact_lookup(train, heldout),
        "proof_rule_id_majority_accuracy_milli": majority_by_key(train, heldout, "proof_rule_id"),
        "proof_rule_family_majority_accuracy_milli": majority_by_key(train, heldout, "proof_rule_family"),
        "operator_class_majority_accuracy_milli": majority_by_key(train, heldout, "operator_class"),
        "surface_family_majority_accuracy_milli": majority_by_key(train, heldout, "surface_family"),
        "length_only_accuracy_milli": majority_by_key(train, heldout, "sequence_length"),
        "output_position_prior_accuracy_milli": output_position_prior(train, heldout),
        "bag_of_tokens_accuracy_milli": bag_pairwise(heldout),
        "same_bag_derangement_milli": same_bag_derangement_milli(rows),
        "edit_candidate_overlap_milli": edit_candidate_overlap_milli(rows),
        "markov_bigram_pairwise_accuracy_milli": markov_bigram_pairwise(train, heldout),
        "bayesian_cooccurrence_pairwise_accuracy_milli": bayesian_cooccurrence_pairwise(train, heldout),
        "l2_neighbor_target_copy_accuracy_milli": l2_neighbor_target_copy(train, heldout),
        "operator_slots_non_order_count": operator_slots_non_order_count(rows),
    }


def class_verdict(operator_class: str, metrics: dict[str, int]) -> str:
    if metrics["exact_lookup_accuracy_milli"] > 0:
        return "REJECT_EXACT_LOOKUP_SHORTCUT"
    if metrics["operator_slots_non_order_count"] > 0:
        return "REJECT_OPERATOR_SLOTS_OUTSIDE_ORDER"
    for key in [
        "proof_rule_id_majority_accuracy_milli",
        "proof_rule_family_majority_accuracy_milli",
        "surface_family_majority_accuracy_milli",
        "length_only_accuracy_milli",
        "output_position_prior_accuracy_milli",
    ]:
        if metrics[key] > 350:
            return f"REJECT_{key.upper()}"
    if metrics["markov_bigram_pairwise_accuracy_milli"] > 650:
        return "REJECT_MARKOV_BIGRAM_SHORTCUT"
    if metrics["bayesian_cooccurrence_pairwise_accuracy_milli"] > 650:
        return "REJECT_BAYESIAN_COOCCURRENCE_SHORTCUT"
    if metrics["l2_neighbor_target_copy_accuracy_milli"] > 650:
        return "REJECT_L2_NEIGHBOR_TARGET_COPY_SHORTCUT"
    if operator_class in PERMUTATION_CLASSES:
        if metrics["bag_of_tokens_accuracy_milli"] != 500:
            return "REJECT_BAG_GATE_NOT_PAIRWISE_CHANCE"
        if metrics["same_bag_derangement_milli"] != 1000:
            return "REJECT_NOT_ALL_PERMUTATION_NEGATIVES_ARE_DERANGED"
    if operator_class == "edit" and metrics["edit_candidate_overlap_milli"] < 500:
        return "REJECT_EDIT_NEGATIVE_TOO_ALIEN"
    return "VALID_OPERATOR_BATTERY_V4_CANDIDATE"


def main() -> int:
    print(f"operator_battery_shortcut: load {TASKS}", file=sys.stderr, flush=True)
    rows = load_rows()
    train, heldout = split_rows(rows)
    print(
        f"operator_battery_shortcut: overall_start rows={len(rows)} train={len(train)} heldout={len(heldout)}",
        file=sys.stderr,
        flush=True,
    )
    overall = metrics_for(rows, train, heldout)
    print("operator_battery_shortcut: overall_done", file=sys.stderr, flush=True)
    by_class = class_metrics(rows)
    verdicts = {
        operator_class: class_verdict(operator_class, metrics)
        for operator_class, metrics in by_class.items()
    }
    report = {
        "schema_version": "operator_battery_shortcut_gate_v4",
        "tasks": str(TASKS),
        "overall_metrics": overall,
        "class_metrics": by_class,
        "class_verdicts": verdicts,
        "verdict": "VALID_OPERATOR_BATTERY_V4_CANDIDATE"
        if all(value == "VALID_OPERATOR_BATTERY_V4_CANDIDATE" for value in verdicts.values())
        else "REJECT_OPERATOR_BATTERY_V4",
    }
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True))
    return 0 if report["verdict"] == "VALID_OPERATOR_BATTERY_V4_CANDIDATE" else 1


if __name__ == "__main__":
    raise SystemExit(main())
