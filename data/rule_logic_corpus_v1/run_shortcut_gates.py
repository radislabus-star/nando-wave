#!/usr/bin/env python3
"""Shortcut baselines for rule_task_v1."""

from __future__ import annotations

import json
import math
import re
from collections import Counter, defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parent
TASKS_PATH = ROOT / "accepted_rule_tasks_v1.jsonl"
REPORT_PATH = ROOT / "shortcut_gate_report.json"
TOKEN_RE = re.compile(r"[A-Za-zА-Яа-я0-9_]+|[{}:;?=<>|,-]")


def tokens(text: str) -> list[str]:
    return [token.lower() for token in TOKEN_RE.findall(text)]


def load_rows(path: Path) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            if line.strip():
                rows.append(json.loads(line))
    return rows


def split_rows(rows: list[dict[str, object]]) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
    train: list[dict[str, object]] = []
    heldout: list[dict[str, object]] = []
    for index, row in enumerate(rows):
        (heldout if index % 5 == 0 else train).append(row)
    return train, heldout


def milli(numerator: int, denominator: int) -> int:
    if denominator == 0:
        return 0
    return round(1000 * numerator / denominator)


def exact_lookup(train: list[dict[str, object]], heldout: list[dict[str, object]]) -> int:
    table = {str(row["input"]): str(row["target"]) for row in train}
    hits = sum(1 for row in heldout if table.get(str(row["input"])) == str(row["target"]))
    return milli(hits, len(heldout))


def majority_by_key(train: list[dict[str, object]], heldout: list[dict[str, object]], key: str) -> int:
    buckets: dict[str, Counter[str]] = defaultdict(Counter)
    for row in train:
        buckets[str(row[key])][str(row["target"])] += 1
    hits = 0
    seen = 0
    for row in heldout:
        bucket = buckets.get(str(row[key]))
        if not bucket:
            continue
        seen += 1
        guess = bucket.most_common(1)[0][0]
        hits += int(guess == str(row["target"]))
    return milli(hits, seen)


def build_token_target_counts(train: list[dict[str, object]]) -> dict[str, Counter[str]]:
    counts: dict[str, Counter[str]] = defaultdict(Counter)
    for row in train:
        target_tokens = tokens(str(row["target"]))
        for input_token in set(tokens(str(row["input"]))):
            for target_token in target_tokens:
                counts[input_token][target_token] += 1
    return counts


def markov_choice(train: list[dict[str, object]], heldout: list[dict[str, object]]) -> int:
    counts = build_token_target_counts(train)
    vocab = set()
    for counter in counts.values():
        vocab.update(counter)
    vocab_size = max(1, len(vocab))

    def score(input_text: str, candidate: str) -> float:
        score_value = 0.0
        input_tokens = set(tokens(input_text))
        for cand_token in tokens(candidate):
            prob = 1.0
            for input_token in input_tokens:
                counter = counts.get(input_token)
                if not counter:
                    continue
                total = sum(counter.values()) + vocab_size
                prob += (counter[cand_token] + 1) / total
            score_value += math.log(prob)
        return score_value

    hits = 0
    ties = 0
    for row in heldout:
        target_score = score(str(row["input"]), str(row["target"]))
        negative_score = score(str(row["input"]), str(row["near_negative"]))
        if target_score > negative_score:
            hits += 1
        elif target_score == negative_score:
            ties += 1
    return milli(hits + ties // 2, len(heldout))


def target_in_input(rows: list[dict[str, object]]) -> int:
    hits = 0
    for row in rows:
        input_tokens = set(tokens(str(row["input"])))
        target_tokens = set(tokens(str(row["target"])))
        hits += int(bool(input_tokens & target_tokens))
    return milli(hits, len(rows))


def avg_target_negative_jaccard(rows: list[dict[str, object]]) -> int:
    total = 0.0
    for row in rows:
        target_tokens = set(tokens(str(row["target"])))
        negative_tokens = set(tokens(str(row["near_negative"])))
        union = target_tokens | negative_tokens
        if union:
            total += len(target_tokens & negative_tokens) / len(union)
    return round(1000 * total / len(rows))


def verdict(metrics: dict[str, int]) -> str:
    if metrics["exact_lookup_accuracy_milli"] > 0:
        return "REJECT_EXACT_LOOKUP_SHORTCUT"
    if metrics["source_group_majority_accuracy_milli"] > 250:
        return "REJECT_SOURCE_GROUP_SHORTCUT"
    if metrics["rule_id_majority_accuracy_milli"] > 450:
        return "REJECT_RULE_ID_TARGET_SHORTCUT"
    if metrics["markov_choice_accuracy_milli"] > 700:
        return "REJECT_MARKOV_SURFACE_SHORTCUT"
    return "VALID_RULE_OPERATOR_PRESSURE_CANDIDATE"


def main() -> int:
    rows = load_rows(TASKS_PATH)
    train, heldout = split_rows(rows)
    metrics = {
        "rows": len(rows),
        "train_rows": len(train),
        "heldout_rows": len(heldout),
        "exact_lookup_accuracy_milli": exact_lookup(train, heldout),
        "source_group_majority_accuracy_milli": majority_by_key(train, heldout, "source_group"),
        "rule_id_majority_accuracy_milli": majority_by_key(train, heldout, "proof_rule_id"),
        "surface_majority_accuracy_milli": majority_by_key(train, heldout, "surface_family"),
        "markov_choice_accuracy_milli": markov_choice(train, heldout),
        "target_in_input_milli": target_in_input(rows),
        "target_negative_jaccard_milli": avg_target_negative_jaccard(rows),
    }
    result = {
        "schema_version": "rule_logic_shortcut_gate_report_v1",
        "metrics": metrics,
        "verdict": verdict(metrics),
    }
    REPORT_PATH.write_text(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))
    return 0 if result["verdict"] == "VALID_RULE_OPERATOR_PRESSURE_CANDIDATE" else 1


if __name__ == "__main__":
    raise SystemExit(main())
